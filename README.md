# BitTorrent Tracker

High-performance BitTorrent tracker written in Rust.
## Configuration

Copy `config.example.toml` to `config.toml`:

```toml
port = 8080
api_key = "your-secret-key"
api_url = "https://your-api.com"
```

## API Endpoints

### Public

```
GET /announce    - BitTorrent announce endpoint
GET /health      - Health check (no auth required)
```

**Announce Parameters:**
- passkey - User authentication key
- info_hash - Torrent identifier (20 bytes, url-encoded)
- peer_id - Client identifier (20 bytes, url-encoded)
- port - Peer listening port
- uploaded - Bytes uploaded
- downloaded - Bytes downloaded
- left - Bytes remaining

Returns bencoded dictionary with peer list and announce interval.

### Admin (require API key)

```
GET  /metrics           - Performance metrics
GET  /update            - Export peer and torrent data
POST /reload            - Reload user and torrent data from external API
GET  /torrent/add       - Add a torrent to the cache
GET  /torrent/remove    - Remove a torrent from the cache
GET  /user/add          - Add a user to the cache
GET  /user/remove       - Remove a user from the cache
GET  /ip/ban            - Ban an IP address
GET  /ip/unban          - Unban an IP address
GET  /ip/list           - List all banned IPs
GET  /client/ban        - Ban a client string
GET  /client/unban      - Unban a client string
GET  /client/list       - List all banned clients
```