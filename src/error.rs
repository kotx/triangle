use thiserror::Error;
use tokio::time::error::Elapsed;

#[derive(Error, Debug)]
pub enum ProxyError {
    #[error("SOCKS error: {0}")]
    Socks(#[from] tokio_socks::Error),

    #[error("timed out: {0}")]
    Timeout(#[from] Elapsed),

    #[error("no forwards were matched for {0}")]
    NoMatch(String),

    #[error("I/O error: {0}")]
    IO(#[from] std::io::Error),

    #[error("could not find valid SNI extension")]
    MissingSNI,
}

#[derive(Error, Debug)]
pub enum ProxyTransportError {
    #[error(transparent)]
    Url(#[from] url::ParseError),

    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error(transparent)]
    Socks5(#[from] tokio_socks::Error),

    #[error("url scheme {0} is not yet supported")]
    UnsupportedScheme(String),
}
