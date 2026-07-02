use std::{error::Error, net::SocketAddr, sync::Arc};

use quinn::{ClientConfig, Connection, Endpoint, RecvStream, SendStream, ServerConfig, VarInt};
use rustls::{RootCertStore, pki_types::CertificateDer, server::WebPkiClientVerifier};

use crate::{
    certs,
    config::{Config, SecurityMode},
};

pub(crate) const ALPN: &[u8] = b"biz-tunnel/1";
pub(crate) const STREAM_CONTROL: u8 = 1;
pub(crate) const STREAM_OPEN: u8 = 2;
pub(crate) const STREAM_TEST: u8 = 3;
pub(crate) const CLOSE_CODE: VarInt = VarInt::from_u32(0);
const MAX_STREAM_MESSAGE_LEN: usize = 1024 * 1024;

pub(crate) type QuicResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

pub(crate) fn server_endpoint(config: &Config) -> QuicResult<Endpoint> {
    install_ring_provider();
    let listen = config
        .tunnel
        .listen
        .as_deref()
        .ok_or("relay missing tunnel.listen")?
        .parse::<SocketAddr>()?;
    Ok(Endpoint::server(server_config(config)?, listen)?)
}

pub(crate) fn client_endpoint(config: &Config) -> QuicResult<Endpoint> {
    install_ring_provider();
    let mut endpoint = Endpoint::client("0.0.0.0:0".parse()?)?;
    endpoint.set_default_client_config(client_config(config)?);
    Ok(endpoint)
}

pub(crate) fn server_name(config: &Config) -> QuicResult<&str> {
    config
        .security
        .server_name
        .as_deref()
        .ok_or_else(|| "security.server_name is required for QUIC client".into())
}

pub(crate) fn verify_expected_peer_fingerprint(
    connection: &Connection,
    config: &Config,
) -> QuicResult<()> {
    let Some(expected) = config.security.expected_peer_cert_sha256.as_deref() else {
        return Ok(());
    };
    let actual = peer_certificate_fingerprint(connection)
        .ok_or("peer did not present a certificate for fingerprint validation")?;
    if !actual.eq_ignore_ascii_case(expected) {
        return Err(format!(
            "peer certificate fingerprint mismatch: expected {expected}, got {actual}"
        )
        .into());
    }
    Ok(())
}

pub(crate) async fn write_stream_message(
    send: &mut SendStream,
    kind: u8,
    payload: &[u8],
) -> QuicResult<()> {
    let len = u32::try_from(payload.len()).map_err(|_| "QUIC stream payload is too large")?;
    send.write_all(&[kind]).await?;
    send.write_all(&len.to_be_bytes()).await?;
    send.write_all(payload).await?;
    Ok(())
}

pub(crate) async fn read_stream_message(recv: &mut RecvStream) -> QuicResult<(u8, Vec<u8>)> {
    let mut kind = [0_u8; 1];
    recv.read_exact(&mut kind).await?;
    let mut len = [0_u8; 4];
    recv.read_exact(&mut len).await?;
    let len = u32::from_be_bytes(len) as usize;
    if len > MAX_STREAM_MESSAGE_LEN {
        return Err(format!("QUIC stream message too large: {len} bytes").into());
    }
    let mut payload = vec![0_u8; len];
    recv.read_exact(&mut payload).await?;
    Ok((kind[0], payload))
}

fn server_config(config: &Config) -> QuicResult<ServerConfig> {
    let cert_path = config
        .security
        .cert
        .as_deref()
        .ok_or("security.cert is required for QUIC relay")?;
    let key_path = config
        .security
        .key
        .as_deref()
        .ok_or("security.key is required for QUIC relay")?;
    let certs = certs::load_certs(cert_path)?;
    let key = certs::load_private_key(key_path)?;

    let mut tls = if config.security.mode == SecurityMode::Mtls {
        let ca_path = config
            .security
            .ca_cert
            .as_deref()
            .ok_or("security.ca_cert is required for QUIC mTLS relay")?;
        let roots = Arc::new(certs::load_root_store(ca_path)?);
        let verifier = WebPkiClientVerifier::builder(roots).build()?;
        rustls::ServerConfig::builder()
            .with_client_cert_verifier(verifier)
            .with_single_cert(certs, key)?
    } else {
        rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)?
    };
    tls.alpn_protocols = vec![ALPN.to_vec()];

    let mut server_config = ServerConfig::with_crypto(Arc::new(
        quinn::crypto::rustls::QuicServerConfig::try_from(tls)?,
    ));
    if let Some(transport) = Arc::get_mut(&mut server_config.transport) {
        transport.max_concurrent_uni_streams(0_u8.into());
    }
    Ok(server_config)
}

fn client_config(config: &Config) -> QuicResult<ClientConfig> {
    let ca_path = config
        .security
        .ca_cert
        .as_deref()
        .ok_or("security.ca_cert is required for QUIC agent")?;
    let roots: RootCertStore = certs::load_root_store(ca_path)?;
    let builder = rustls::ClientConfig::builder().with_root_certificates(roots);

    let mut tls = if config.security.mode == SecurityMode::Mtls {
        let cert_path = config
            .security
            .cert
            .as_deref()
            .ok_or("security.cert is required for QUIC mTLS agent")?;
        let key_path = config
            .security
            .key
            .as_deref()
            .ok_or("security.key is required for QUIC mTLS agent")?;
        builder.with_client_auth_cert(
            certs::load_certs(cert_path)?,
            certs::load_private_key(key_path)?,
        )?
    } else {
        builder.with_no_client_auth()
    };
    tls.alpn_protocols = vec![ALPN.to_vec()];

    Ok(ClientConfig::new(Arc::new(
        quinn::crypto::rustls::QuicClientConfig::try_from(tls)?,
    )))
}

fn peer_certificate_fingerprint(connection: &Connection) -> Option<String> {
    let identity = connection.peer_identity()?;
    let certs = identity.downcast::<Vec<CertificateDer<'static>>>().ok()?;
    certs.first().map(certs::certificate_fingerprint)
}

fn install_ring_provider() {
    let _ = rustls::crypto::ring::default_provider().install_default();
}
