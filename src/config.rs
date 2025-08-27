use std::{net::SocketAddr, time::Duration};

use serde::Deserialize;

use crate::proxy::ProxyTransport;

fn default_addr() -> SocketAddr {
    SocketAddr::new("0.0.0.0".parse().unwrap(), 8443)
}

fn default_timeout() -> u64 {
    10 * 1000
}

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default = "default_addr")]
    pub listen_addr: SocketAddr,
    #[serde(default = "default_timeout")]
    pub timeout_ms: u64,
    pub forwards: Vec<Forward>,
}

#[derive(Debug, Deserialize)]
pub struct Forward {
    pub src: Vec<String>,
    pub dst: Vec<ProxyTransport>,
}
