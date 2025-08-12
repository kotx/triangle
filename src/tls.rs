use crate::error::ProxyError;
use rustls::server::Acceptor;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWrite;
use tokio_rustls::LazyConfigAcceptor;
use tokio_util::io::InspectReader;

/// Parses an SNI extension and returns the hostname and read bytes.
// TODO(kotx): This function should return read bytes regardless of errors.
// This would allow for falling back to different protocols.
pub async fn parse_sni<S: AsyncReadExt + AsyncWrite + Unpin>(
    stream: &mut S,
) -> Result<(String, Vec<u8>), ProxyError> {
    let (read, write) = tokio::io::split(stream);
    let mut initial_buf = Vec::with_capacity(4096);
    let mut inspector = InspectReader::new(read, |data| {
        initial_buf.extend_from_slice(data);
    });
    let mut stream = tokio::io::join(&mut inspector, write);
    let acceptor = LazyConfigAcceptor::new(Acceptor::default(), &mut stream);

    let handshake = acceptor.await?;
    let client_hello = handshake.client_hello();
    let hostname = client_hello.server_name().ok_or(ProxyError::MissingSNI)?;

    Ok((hostname.to_string(), initial_buf))
}
