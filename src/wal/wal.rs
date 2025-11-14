use anyhow::{bail, Context, Result};
use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// WAL operation types
#[derive(Debug, Clone, PartialEq)]
pub enum WalOperation {
    AddTorrent {
        id: u32,
        info_hash: [u8; 20],
        freeleech: bool,
    },
    RemoveTorrent {
        info_hash: [u8; 20],
    },
    AddUser {
        id: u32,
        passkey: [u8; 32],
        class: u8,
    },
    RemoveUser {
        passkey: [u8; 32],
    },
}

impl WalOperation {
    fn to_string(&self) -> String {
        match self {
            WalOperation::AddTorrent {
                id,
                info_hash,
                freeleech,
            } => {
                let hex_hash = hex::encode(info_hash);
                let freeleech_flag = if *freeleech { "1" } else { "0" };
                format!("ADD_TORRENT|{}|{}|{}", id, hex_hash, freeleech_flag)
            }
            WalOperation::RemoveTorrent { info_hash } => {
                let hex_hash = hex::encode(info_hash);
                format!("REMOVE_TORRENT|{}", hex_hash)
            }
            WalOperation::AddUser { id, passkey, class } => {
                let hex_passkey = hex::encode(passkey);
                format!("ADD_USER|{}|{}|{}", id, hex_passkey, class)
            }
            WalOperation::RemoveUser { passkey } => {
                let hex_passkey = hex::encode(passkey);
                format!("REMOVE_USER|{}", hex_passkey)
            }
        }
    }

    fn from_string(line: &str) -> Result<Self> {
        let parts: Vec<&str> = line.split('|').collect();

        match parts.get(0) {
            Some(&"ADD_TORRENT") => {
                if parts.len() != 4 {
                    bail!("Invalid ADD_TORRENT format");
                }
                let id = parts[1].parse::<u32>().context("Invalid torrent ID")?;
                let info_hash_bytes = hex::decode(parts[2]).context("Invalid info_hash hex")?;
                if info_hash_bytes.len() != 20 {
                    bail!("info_hash must be 20 bytes");
                }
                let mut info_hash = [0u8; 20];
                info_hash.copy_from_slice(&info_hash_bytes);
                let freeleech = parts[3] == "1";

                Ok(WalOperation::AddTorrent {
                    id,
                    info_hash,
                    freeleech,
                })
            }
            Some(&"REMOVE_TORRENT") => {
                if parts.len() != 2 {
                    bail!("Invalid REMOVE_TORRENT format");
                }
                let info_hash_bytes = hex::decode(parts[1]).context("Invalid info_hash hex")?;
                if info_hash_bytes.len() != 20 {
                    bail!("info_hash must be 20 bytes");
                }
                let mut info_hash = [0u8; 20];
                info_hash.copy_from_slice(&info_hash_bytes);

                Ok(WalOperation::RemoveTorrent { info_hash })
            }
            Some(&"ADD_USER") => {
                if parts.len() != 4 {
                    bail!("Invalid ADD_USER format");
                }
                let id = parts[1].parse::<u32>().context("Invalid user ID")?;
                let passkey_bytes = hex::decode(parts[2]).context("Invalid passkey hex")?;
                if passkey_bytes.len() != 32 {
                    bail!("passkey must be 32 bytes");
                }
                let mut passkey = [0u8; 32];
                passkey.copy_from_slice(&passkey_bytes);
                let class = parts[3].parse::<u8>().context("Invalid user class")?;

                Ok(WalOperation::AddUser { id, passkey, class })
            }
            Some(&"REMOVE_USER") => {
                if parts.len() != 2 {
                    bail!("Invalid REMOVE_USER format");
                }
                let passkey_bytes = hex::decode(parts[1]).context("Invalid passkey hex")?;
                if passkey_bytes.len() != 32 {
                    bail!("passkey must be 32 bytes");
                }
                let mut passkey = [0u8; 32];
                passkey.copy_from_slice(&passkey_bytes);

                Ok(WalOperation::RemoveUser { passkey })
            }
            _ => bail!("Unknown operation type"),
        }
    }
}

pub struct Wal {
    file: Arc<Mutex<File>>,
    path: PathBuf,
}

impl Wal {
    pub fn new(path: PathBuf) -> Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .context("Failed to open WAL file")?;

        Ok(Wal {
            file: Arc::new(Mutex::new(file)),
            path,
        })
    }

    pub fn log_operation(&self, op: WalOperation) -> Result<()> {
        let line = op.to_string();
        let mut file = self.file.lock().unwrap();
        writeln!(file, "{}", line).context("Failed to write to WAL")?;
        file.flush().context("Failed to flush WAL")?;
        Ok(())
    }


    pub fn replay(&self) -> Result<Vec<WalOperation>> {
        let file = File::open(&self.path).context("Failed to open WAL for replay")?;
        let reader = BufReader::new(file);
        let mut operations = Vec::new();

        for (line_num, line_result) in reader.lines().enumerate() {
            let line = line_result.context("Failed to read line from WAL")?;
            let line = line.trim();

            // Skip empty lines
            if line.is_empty() {
                continue;
            }

            match WalOperation::from_string(line) {
                Ok(op) => operations.push(op),
                Err(e) => {
                    tracing::warn!(
                        line_num = line_num + 1,
                        error = %e,
                        "Failed to parse WAL line, skipping"
                    );
                }
            }
        }

        Ok(operations)
    }


    pub fn truncate(&self) -> Result<()> {
        let mut file = self.file.lock().unwrap();
        file.set_len(0).context("Failed to truncate WAL")?;
        file.flush().context("Failed to flush WAL after truncate")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_wal_operation_serialization() {
        let info_hash = [1u8; 20];
        let passkey = [2u8; 32];

        // Test AddTorrent
        let op = WalOperation::AddTorrent {
            id: 123,
            info_hash,
            freeleech: true,
        };
        let serialized = op.to_string();
        assert_eq!(
            serialized,
            format!("ADD_TORRENT|123|{}|1", hex::encode(info_hash))
        );
        let deserialized = WalOperation::from_string(&serialized).unwrap();
        assert_eq!(op, deserialized);

        // Test RemoveTorrent
        let op = WalOperation::RemoveTorrent { info_hash };
        let serialized = op.to_string();
        assert_eq!(serialized, format!("REMOVE_TORRENT|{}", hex::encode(info_hash)));
        let deserialized = WalOperation::from_string(&serialized).unwrap();
        assert_eq!(op, deserialized);

        // Test AddUser
        let op = WalOperation::AddUser {
            id: 456,
            passkey,
            class: 1,
        };
        let serialized = op.to_string();
        assert_eq!(
            serialized,
            format!("ADD_USER|456|{}|1", hex::encode(passkey))
        );
        let deserialized = WalOperation::from_string(&serialized).unwrap();
        assert_eq!(op, deserialized);

        // Test RemoveUser
        let op = WalOperation::RemoveUser { passkey };
        let serialized = op.to_string();
        assert_eq!(serialized, format!("REMOVE_USER|{}", hex::encode(passkey)));
        let deserialized = WalOperation::from_string(&serialized).unwrap();
        assert_eq!(op, deserialized);
    }

    #[test]
    fn test_wal_log_and_replay() {
        let temp_dir = TempDir::new().unwrap();
        let wal_path = temp_dir.path().join("test.wal");

        let wal = Wal::new(wal_path.clone()).unwrap();

        let info_hash = [1u8; 20];
        let passkey = [2u8; 32];

        // Log operations
        wal.log_operation(WalOperation::AddTorrent {
            id: 123,
            info_hash,
            freeleech: true,
        })
        .unwrap();

        wal.log_operation(WalOperation::AddUser {
            id: 456,
            passkey,
            class: 1,
        })
        .unwrap();

        wal.log_operation(WalOperation::RemoveTorrent { info_hash })
            .unwrap();

        wal.log_operation(WalOperation::RemoveUser { passkey })
            .unwrap();

        // Replay operations
        let operations = wal.replay().unwrap();
        assert_eq!(operations.len(), 4);

        match &operations[0] {
            WalOperation::AddTorrent {
                id,
                info_hash: h,
                freeleech,
            } => {
                assert_eq!(*id, 123);
                assert_eq!(*h, info_hash);
                assert_eq!(*freeleech, true);
            }
            _ => panic!("Expected AddTorrent"),
        }

        match &operations[1] {
            WalOperation::AddUser {
                id,
                passkey: p,
                class,
            } => {
                assert_eq!(*id, 456);
                assert_eq!(*p, passkey);
                assert_eq!(*class, 1);
            }
            _ => panic!("Expected AddUser"),
        }

        match &operations[2] {
            WalOperation::RemoveTorrent { info_hash: h } => {
                assert_eq!(*h, info_hash);
            }
            _ => panic!("Expected RemoveTorrent"),
        }

        match &operations[3] {
            WalOperation::RemoveUser { passkey: p } => {
                assert_eq!(*p, passkey);
            }
            _ => panic!("Expected RemoveUser"),
        }
    }

    #[test]
    fn test_wal_truncate() {
        let temp_dir = TempDir::new().unwrap();
        let wal_path = temp_dir.path().join("test.wal");

        let wal = Wal::new(wal_path.clone()).unwrap();

        let info_hash = [1u8; 20];

        // Log an operation
        wal.log_operation(WalOperation::AddTorrent {
            id: 123,
            info_hash,
            freeleech: false,
        })
        .unwrap();

        // Verify it was logged
        let operations = wal.replay().unwrap();
        assert_eq!(operations.len(), 1);

        // Truncate
        wal.truncate().unwrap();

        // Verify it's empty
        let operations = wal.replay().unwrap();
        assert_eq!(operations.len(), 0);
    }

    #[test]
    fn test_wal_invalid_lines() {
        let temp_dir = TempDir::new().unwrap();
        let wal_path = temp_dir.path().join("test.wal");

        // Write invalid data directly to file
        fs::write(&wal_path, "INVALID_OP|data\nADD_TORRENT|123|0101010101010101010101010101010101010101|1\n").unwrap();

        let wal = Wal::new(wal_path).unwrap();
        let operations = wal.replay().unwrap();

        // Should skip invalid line and parse valid one
        assert_eq!(operations.len(), 1);
    }
}
