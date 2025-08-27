use std::{net::SocketAddr, str::FromStr};

use serde::{Deserialize, de::Visitor};
use tokio::{
    io::{AsyncRead, AsyncWrite},
    net::TcpStream,
};
use tokio_socks::{TargetAddr, tcp::Socks5Stream};
use url::Url;

use crate::error::{ProxyError, ProxyTransportError};

pub trait ProxyStream: AsyncRead + AsyncWrite + Unpin + Send {}

impl<T> ProxyStream for T where T: AsyncRead + AsyncWrite + Unpin + Send {}

#[derive(Debug)]
pub enum ProxyTransport {
    Direct,
    Socks5(Vec<SocketAddr>),
}

impl ProxyTransport {
    pub async fn connect(&self, hostname: &str) -> Result<Box<dyn ProxyStream>, ProxyError> {
        if hostname == "localhost" {
            Err(ProxyError::InfiniteLoop) // TODO: better loop detection
        } else {
            Ok(match self {
                ProxyTransport::Direct => {
                    Box::new(TcpStream::connect(format!("{hostname}:443")).await?)
                }
                ProxyTransport::Socks5(socket_addr) => {
                    let stream = Socks5Stream::connect(
                        socket_addr.as_slice(),
                        TargetAddr::Domain(std::borrow::Cow::Borrowed(hostname), 443),
                    )
                    .await?;
                    Box::new(stream)
                }
            })
        }
    }
}

impl TryFrom<Url> for ProxyTransport {
    type Error = ProxyTransportError;

    fn try_from(value: Url) -> Result<Self, Self::Error> {
        let scheme = value.scheme();
        Ok(match scheme {
            "socks" | "socks5" | "socks5h" => {
                ProxyTransport::Socks5(value.socket_addrs(|| Some(1080))?)
            }
            _ => Err(ProxyTransportError::UnsupportedScheme(scheme.to_string()))?,
        })
    }
}

impl FromStr for ProxyTransport {
    type Err = ProxyTransportError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "direct" {
            Ok(ProxyTransport::Direct)
        } else {
            ProxyTransport::try_from(Url::from_str(s)?)
        }
    }
}

impl<'de> Deserialize<'de> for ProxyTransport {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct ProxyTargetVisitor;
        impl Visitor<'_> for ProxyTargetVisitor {
            type Value = ProxyTransport;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("valid proxy target url")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(FromStr::from_str(v).unwrap())
            }
        }

        de.deserialize_str(ProxyTargetVisitor)
    }
}
