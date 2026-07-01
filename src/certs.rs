use std::{
    error::Error,
    fs,
    io::{self, BufReader},
    path::Path,
};

use rustls::{RootCertStore, pki_types::CertificateDer, pki_types::PrivateKeyDer};
use sha2::{Digest, Sha256};

pub type CertResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

pub fn load_certs(path: impl AsRef<Path>) -> CertResult<Vec<CertificateDer<'static>>> {
    let bytes = fs::read(path)?;
    let mut reader = BufReader::new(bytes.as_slice());
    let certs = rustls_pemfile::certs(&mut reader).collect::<Result<Vec<_>, _>>()?;
    if certs.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "no certificates found").into());
    }
    Ok(certs)
}

pub fn load_private_key(path: impl AsRef<Path>) -> CertResult<PrivateKeyDer<'static>> {
    let bytes = fs::read(path)?;
    let mut reader = BufReader::new(bytes.as_slice());
    let Some(key) = rustls_pemfile::private_key(&mut reader)? else {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "no private key found").into());
    };
    Ok(key)
}

pub fn load_root_store(path: impl AsRef<Path>) -> CertResult<RootCertStore> {
    let mut roots = RootCertStore::empty();
    for cert in load_certs(path)? {
        roots.add(cert)?;
    }
    Ok(roots)
}

pub fn certificate_fingerprint_from_path(path: impl AsRef<Path>) -> CertResult<String> {
    let cert = load_certs(path)?
        .into_iter()
        .next()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "no certificate found"))?;
    Ok(certificate_fingerprint(&cert))
}

pub fn certificate_fingerprint(cert: &CertificateDer<'_>) -> String {
    hex::encode(Sha256::digest(cert.as_ref()))
}
