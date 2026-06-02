use tokio_quiche::settings::CertificateKind;

pub struct TlsCertificatePathsOwned {
    pub cert: String,
    pub private_key: String,
    pub kind: CertificateKind,
}
