mod config;

use std::net::SocketAddr;
use std::time::Duration;

use color_eyre::eyre::Result;
use figment::Figment;
use figment::providers::{Env, Format, Json};
use thiserror::Error;
use tls_parser::{SNIType, nom::Needed};
use tokio::io::AsyncWriteExt;
use tokio::time::error::Elapsed;
use tokio::{
    io::{AsyncReadExt, copy_bidirectional},
    net::{TcpListener, TcpStream},
};
use tokio_socks::TargetAddr;
use tokio_socks::tcp::Socks5Stream;
use tracing::instrument;
use wildmatch::WildMatch;

use crate::config::Config;

#[derive(Error, Debug)]
enum ProxyError {
    #[error("timed out: {0}")]
    Timeout(#[from] Elapsed),

    #[error("SOCKS error: {0}")]
    Socks(#[from] tokio_socks::Error),

    #[error("no forwards were matched for {0}")]
    NoMatch(String),

    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),

    #[error("could not parse TLS packet")]
    BadPacket, // TODO: figure out how to map nom errors

    #[error("could not find TLS handshake")]
    MissingHandshake,

    #[error("could not find TLS ClientHello")]
    MissingClientHello,

    #[error("could not find valid SNI extension")]
    MissingSNI,

    #[error("could not find hostname in SNI extension")]
    MissingSNIHostName,

    #[error("SNI was not valid UTF-8: {0}")]
    BadSNI(#[from] std::string::FromUtf8Error),
}

/// Parses an SNI extension and returns the read bytes.
async fn parse_sni<R: AsyncReadExt + Unpin>(
    reader: &mut R,
) -> Result<(String, Vec<u8>), ProxyError> {
    let mut initial_buf = Vec::new();
    _ = reader.read(&mut initial_buf).await?;
    let packet = loop {
        match tls_parser::parse_tls_plaintext(&initial_buf) {
            Ok((_, packet)) => break packet,
            Err(tls_parser::Err::Incomplete(Needed::Size(needed))) => {
                // tracing::trace!("reading more data for TLS packet: {needed}");
                let mut buf = vec![0; needed.into()];
                reader.read_exact(&mut buf).await?;
                initial_buf.append(&mut buf);
            }
            Err(err) => todo!("{err}"),
        }
    };

    let hostname = match packet.msg.first() {
        Some(tls_parser::TlsMessage::Handshake(handshake)) => match handshake {
            tls_parser::TlsMessageHandshake::ClientHello(contents) => {
                let (_, exts) = tls_parser::parse_tls_client_hello_extensions(
                    contents.ext.ok_or(ProxyError::MissingSNI)?,
                )
                .or(Err(ProxyError::BadPacket))?;

                exts.iter().find_map(|ext| match ext {
                    tls_parser::TlsExtension::SNI(data) => {
                        let hostname_bytes = data
                            .iter()
                            .find(|(t, _)| *t == SNIType::HostName)
                            .map(|(_, v)| v);

                        Some(match hostname_bytes {
                            Some(hostname_bytes) => String::from_utf8(hostname_bytes.to_vec())
                                .map_err(ProxyError::BadSNI),
                            None => Err(ProxyError::MissingSNIHostName),
                        })
                    }
                    _ => None,
                })
            }
            _ => Err(ProxyError::MissingClientHello)?,
        },
        _ => Err(ProxyError::MissingHandshake)?,
    }
    .ok_or(ProxyError::MissingSNI)?;

    Ok((hostname?, initial_buf))
}

#[instrument(skip(config, stream))]
async fn handle_connection(
    config: &Config,
    mut stream: TcpStream,
    addr: SocketAddr,
) -> Result<(), ProxyError> {
    let (hostname, initial_buf) =
        tokio::time::timeout(Duration::from_secs(10), parse_sni(&mut stream)).await??;

    let forward = config
        .forwards
        .iter()
        .find(|fwd| {
            fwd.src
                .iter()
                .any(|pat| WildMatch::new(pat).matches(&hostname))
        })
        .ok_or(ProxyError::NoMatch(hostname.clone()))?;

    tracing::info!("proxying {} -> {:?}", &hostname, &forward.dst);
    let mut conn = Socks5Stream::connect(
        forward.dst_addrs().as_slice(),
        TargetAddr::Domain(std::borrow::Cow::Borrowed(&hostname), 443),
    )
    .await?;
    conn.write_all(&initial_buf).await?;
    copy_bidirectional(&mut conn, &mut stream).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    console_subscriber::init();
    color_eyre::install()?;

    let config: Config = Figment::new()
        .merge(Json::file("sniproxy.json"))
        .merge(Env::prefixed("TRIANGLE_"))
        .extract()?;

    tracing::info!("{config:?}");

    let lc = TcpListener::bind(config.listen_addr).await.unwrap();
    loop {
        match lc.accept().await {
            Ok((stream, addr)) => {
                if let Err(err) = handle_connection(&config, stream, addr).await {
                    tracing::error!("error proxying connection: {err}");
                }
            }
            Err(err) => tracing::error!("error accepting connection: {err}"),
        }
    }
}
