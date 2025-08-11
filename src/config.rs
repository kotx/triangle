use std::net::SocketAddr;

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

impl Forward {
    #[instrument(level = "trace")]
    pub fn dst_addrs(&self) -> Vec<SocketAddr> {
        self.dst
            .iter()
            .flat_map(|dst| {
                dst.socket_addrs(|| {
                    match dst.scheme() {
                        "socks" | "socks5" | "socks5h" => Some(1080),
                        _ => None,
                    }
                })
                .unwrap()
            })
            .collect()
    }
}
