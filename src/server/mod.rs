mod controller;
#[cfg(test)]
mod mod_test;
mod router;
#[cfg(test)]
mod router_test;

pub(crate) use controller::*;
pub(crate) use router::*;

use crate::{FromBody, Response, ResponseSender};
use serde::de::DeserializeOwned;
use std::env::temp_dir;
use std::fmt::Debug;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::OnceLock;
use tokio_quiche::http3::settings::Http3Settings;
use tokio_quiche::listen;
use tokio_quiche::metrics::DefaultMetrics;
use tokio_quiche::settings::{CertificateKind, TlsCertificatePaths};
use tokio_quiche::{ConnectionParams, ServerH3Driver};
use tokio_stream::StreamExt;

pub struct Server {
    pub(crate) cert: Option<TlsCertificatePathsOwned>,
    pub(crate) addr: SocketAddr,
    pub(crate) router: Router,
}

pub struct TlsCertificatePathsOwned {
    pub cert: String,
    pub private_key: String,
    pub kind: CertificateKind,
}

impl Server {
    /// Creates a new [`Server`] with the following defaults:
    /// - Certificate (TLS): Generic one bundled for development purposes
    /// - Address: listens on `0.0.0.0:4043`
    /// - Routes: none
    pub fn new() -> Self {
        Self {
            cert: None,
            addr: SocketAddr::from(([0, 0, 0, 0], 4043)),
            router: Router::default(),
        }
    }

    pub fn with_address(&mut self, addr: SocketAddr) -> &mut Self {
        self.addr = addr;
        self
    }

    pub fn with_cert(
        &mut self,
        private_key: impl Into<String>,
        cert: impl Into<String>,
        kind: CertificateKind,
    ) -> &mut Self {
        self.cert = Some(TlsCertificatePathsOwned {
            cert: cert.into(),
            private_key: private_key.into(),
            kind,
        });
        self
    }

    pub fn nest_routes(&mut self, router: Router) -> &mut Self {
        self.router.nest(router);
        self
    }

    pub fn add_route<T, R>(&mut self, route: TypedRoute<T, R>) -> &mut Self
    where
        T: FromBody + DeserializeOwned + Send + 'static,
        serde_json::Error: From<<T as FromBody>::Error>,
        R: Debug + Send + 'static,
        Response<R>: ResponseSender,
    {
        self.router.add_route(route);
        self
    }

    pub async fn start(self) -> Result<(), Box<dyn std::error::Error>> {
        let socket = tokio::net::UdpSocket::bind(self.addr).await?;

        let mut listeners = listen(
            [socket],
            ConnectionParams::new_server(Default::default(), self.cert(), Default::default()),
            DefaultMetrics,
        )?;
        let accept_stream = &mut listeners[0];

        let router = Arc::new(self.router);

        tracing::info!(?self.addr, "Starting server");
        while let Some(conn) = accept_stream.next().await {
            let (driver, controller) = ServerH3Driver::new(Http3Settings::default());
            conn?.start(driver);

            let router = router.clone();
            tokio::spawn(async move {
                safely_handle_connection(controller, &router.clone()).await;
            });
        }
        Ok(())
    }

    // --------- Utils ---------

    fn bundled_certs() -> TlsCertificatePaths<'static> {
        static PATHS: OnceLock<(String, String)> = OnceLock::new();
        const CERT: &[u8] = include_bytes!("../../certs/cert.pem");
        const KEY: &[u8] = include_bytes!("../../certs/key.pem");

        let (cert, key) = PATHS.get_or_init(|| {
            let temp_dir = temp_dir();
            let cert_path = temp_dir.join("cert.pem");
            let key_path = temp_dir.join("key.pem");

            std::fs::write(&cert_path, CERT).expect("Failed to create");
            std::fs::write(&key_path, KEY).expect("Failed to create");

            (
                cert_path
                    .to_str()
                    .ok_or("invalid path")
                    .expect("Failed to create")
                    .to_owned(),
                key_path
                    .to_str()
                    .ok_or("invalid path")
                    .expect("Failed to create")
                    .to_owned(),
            )
        });

        TlsCertificatePaths {
            cert,
            private_key: key,
            kind: tokio_quiche::settings::CertificateKind::X509,
        }
    }

    fn cert<'a>(&'a self) -> TlsCertificatePaths<'a> {
        if let Some(cert) = &self.cert {
            TlsCertificatePaths {
                cert: cert.cert.as_str(),
                private_key: cert.private_key.as_str(),
                kind: cert.kind,
            }
        } else {
            Self::bundled_certs()
        }
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}
