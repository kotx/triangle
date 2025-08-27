mod config;
mod error;
mod proxy;
mod tls;

use std::net::SocketAddr;
use std::time::Duration;

use color_eyre::eyre::Result;
use figment::Figment;
use figment::providers::{Env, Format, Json};
use tokio::io::AsyncWriteExt;
use tokio::{
    io::copy_bidirectional,
    net::{TcpListener, TcpStream},
};
use tracing::instrument;
use wildmatch::WildMatch;

use crate::config::Config;
use crate::error::ProxyError;
use crate::tls::parse_sni;

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

    let mut conn = {
        let dest = &forward.dst[0];
        tracing::info!("proxying {} -> {:?}", &hostname, &dest);
        dest.connect(&hostname).await?
    };
    conn.write_all(&initial_buf).await?;
    copy_bidirectional(&mut conn, &mut stream).await?;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
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
