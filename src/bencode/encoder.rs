pub trait BencodeEncode {
    fn bencode(&self, buf: &mut Vec<u8>);
}

impl BencodeEncode for i64 {
    fn bencode(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(b"i");

        let mut buffer = itoa::Buffer::new();
        buf.extend_from_slice(buffer.format(*self).as_bytes());
        buf.extend_from_slice(b"e");
    }
}

impl BencodeEncode for &[u8] {
    fn bencode(&self, buf: &mut Vec<u8>) {
        let mut buffer = itoa::Buffer::new();
        buf.extend_from_slice(buffer.format(self.len()).as_bytes());
        buf.extend_from_slice(b":");
        buf.extend_from_slice(self);
    }
}

impl BencodeEncode for &str {
    fn bencode(&self, buf: &mut Vec<u8>) {
        self.as_bytes().bencode(buf);
    }
}

impl BencodeEncode for Vec<u8> {
    fn bencode(&self, buf: &mut Vec<u8>) {
        self.as_slice().bencode(buf);
    }
}

pub fn encode_list<T: BencodeEncode>(items: &[T], buf: &mut Vec<u8>) {
    buf.extend_from_slice(b"l");
    for item in items {
        item.bencode(buf);
    }
    buf.extend_from_slice(b"e");
}

pub fn encode_dict<K: BencodeEncode, V: BencodeEncode>(
    pairs: &[(K, V)],
    buf: &mut Vec<u8>,
) {
    buf.extend_from_slice(b"d");
    for (key, value) in pairs {
        key.bencode(buf);
        value.bencode(buf);
    }
    buf.extend_from_slice(b"e");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_integer() {
        let mut buf = Vec::new();
        42i64.bencode(&mut buf);
        assert_eq!(buf, b"i42e");

        let mut buf = Vec::new();
        (-42i64).bencode(&mut buf);
        assert_eq!(buf, b"i-42e");

        let mut buf = Vec::new();
        0i64.bencode(&mut buf);
        assert_eq!(buf, b"i0e");
    }

    #[test]
    fn test_encode_bytes() {
        let mut buf = Vec::new();
        b"hello".as_slice().bencode(&mut buf);
        assert_eq!(buf, b"5:hello");

        let mut buf = Vec::new();
        b"".as_slice().bencode(&mut buf);
        assert_eq!(buf, b"0:");
    }

    #[test]
    fn test_encode_string() {
        let mut buf = Vec::new();
        "spam".bencode(&mut buf);
        assert_eq!(buf, b"4:spam");
    }

    #[test]
    fn test_encode_vec() {
        let mut buf = Vec::new();
        vec![1u8, 2, 3, 4].bencode(&mut buf);
        assert_eq!(buf, b"4:\x01\x02\x03\x04");
    }

    #[test]
    fn test_encode_list() {
        let mut buf = Vec::new();
        encode_list(&[1i64, 2i64, 3i64], &mut buf);
        assert_eq!(buf, b"li1ei2ei3ee");
    }

    #[test]
    fn test_encode_dict() {
        let mut buf = Vec::new();
        // Keys must be in sorted order for bencode 
        encode_dict(&[("bar", 100i64), ("foo", 42i64)], &mut buf);
        assert_eq!(buf, b"d3:bari100e3:fooi42ee");
    }
}
