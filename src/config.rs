use std::net::SocketAddr;

use memoize::memoize;
use serde::Deserialize;
use tracing::instrument;
use url::Url;

fn default_addr() -> SocketAddr {
    SocketAddr::new("0.0.0.0".parse().unwrap(), 8443)
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_addr")]
    pub listen_addr: SocketAddr,
    pub forwards: Vec<Forward>,
}

#[derive(Debug, Deserialize)]
pub struct Forward {
    pub src: Vec<String>,
    pub dst: Vec<Url>,
}

#[memoize]
fn url_to_addrs(url: Url) -> Vec<SocketAddr> {
    url.socket_addrs(|| match url.scheme() {
        "socks" | "socks5" | "socks5h" => Some(1080),
        _ => None,
    })
    .unwrap()
}

impl Forward {
    #[instrument(level = "trace")]
    pub fn dst_addrs(&self) -> Vec<SocketAddr> {
        self.dst
            .iter()
            .flat_map(|dst| url_to_addrs(dst.clone()))
            .collect()
    }
}
