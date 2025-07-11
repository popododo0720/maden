use std::{
    fs::File,
    io::BufReader,
    path::Path,
};

pub fn load_certs(path: &Path) -> std::io::Result<Vec<rustls::pki_types::CertificateDer<'static>>> {
    rustls_pemfile::certs(&mut BufReader::new(File::open(path)?)).collect()
}

pub fn load_private_key(path: &Path) -> std::io::Result<rustls::pki_types::PrivateKeyDer<'static>> {
    rustls_pemfile::private_key(&mut BufReader::new(File::open(path)?))
        .and_then(|key| key.ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid key")))
}
