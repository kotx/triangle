use std::net::SocketAddr;

use serde::Deserialize;

use crate::proxy::ProxyTransport;

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
    pub dst: Vec<ProxyTransport>,
}
