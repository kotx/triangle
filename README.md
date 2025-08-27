# Triangle

![i guess we doin socks now](triangle.jpg)

A simple TLS-with-SNI to SOCKS5 proxy inspired by [sniproxy](https://github.com/ameshkov/sniproxy).

## Installation

Install the [latest release](/releases/latest) from GitHub releases:

```
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/kotx/triangle/releases/latest/download/triangle-proxy-installer.sh | sh
```

or build from source:

```
cargo install triangle-proxy
```

## Configuration

Configuring Triangle is as simple as putting this in `sniproxy.json`:

```json
{
  "listen_addr": "127.0.0.1:8443",
  "timeout_ms": 10000, // timeout to initial handshake
  "forwards": [
    {
      "src": ["myip.wtf", "*.bsky.app"],
      "dst": ["socks5://127.0.0.1:9150"] // retry functionality is to be implemented
    },
    {
      "src": ["*"], // fallback
      "dst": ["direct"] // forward connecton directly (useful for proxy servers)
    }
  ]
}
```
