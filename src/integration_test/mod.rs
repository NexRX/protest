use crate::{Method, Server, Status};
use bytes::Bytes;
use futures_util::SinkExt as _;
use quiche::h3;
use quiche::h3::NameValue as _;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::OnceLock;
use std::time::Duration;
use test_context::AsyncTestContext;
use tokio::net::UdpSocket;
use tokio::task::JoinHandle;
use tokio_quiche::ClientH3Controller;
use tokio_quiche::http3::driver::{
    ClientH3Event, H3Event, InboundFrame, NewClientRequest, OutboundFrame,
};
use tracing::trace;

pub struct IntegrationTest {
    server: Option<Server>,
    addr: SocketAddr,
    server_handle: Option<JoinHandle<()>>,
    client: Option<(tokio_quiche::QuicConnection, ClientH3Controller)>,
}

impl IntegrationTest {
    async fn new() -> Self {
        let addr = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("failed to bind server socket")
            .local_addr()
            .expect("failed to get server local addr");
        let mut server = Server::new();
        server.with_address(addr);
        Self {
            server: Some(server),
            addr,
            server_handle: None,
            client: None,
        }
    }

    pub async fn start(&mut self) {
        let server = self.server.take().expect("Server not initialized");
        self.server_handle = Some(tokio::spawn(async move {
            let _ = server.start().await;
        }));

        // instead of sleep poll the server addr untill its accepting connections
        const MAX_MS: u64 = 2 * 1000;
        const INTERVAL_MS: u64 = 20;
        const MAX_TRIES: u64 = MAX_MS / INTERVAL_MS;

        let mut i = 0;
        while i < MAX_TRIES {
            if UdpSocket::bind(&self.addr).await.is_ok() {
                break;
            }
            i += 1;
            if i > MAX_TRIES
                || self
                    .server_handle
                    .as_ref()
                    .is_some_and(JoinHandle::is_finished)
            {
                panic!("failed to start server socket");
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    pub async fn new_client(&self) -> (tokio_quiche::QuicConnection, ClientH3Controller) {
        let socket = UdpSocket::bind("127.0.0.1:0")
            .await
            .expect("failed to bind client socket");
        socket
            .connect(&self.addr)
            .await
            .expect("failed to connect client socket");

        tokio_quiche::quic::connect(socket, Some("localhost"))
            .await
            .expect("QUIC/H3 handshake failed")
    }

    pub async fn send(
        &mut self,
        method: Method,
        path: &str,
        body: Option<impl Into<Bytes>>,
        request_id: u64,
    ) -> TestResponse {
        if self.server_handle.is_none() {
            self.start().await;
        }
        if self.client.is_none() {
            self.client = Some(self.new_client().await);
        }
        let (_, controller) = self.client.as_mut().unwrap();
        // Only provide a body_writer channel when there is actually a body to
        // send.  Passing body_writer: None causes tokio-quiche to set FIN on
        // the headers frame, so the server sees read_fin: true and can
        // dispatch the request without waiting for body bytes.
        let body_writer_rx = if body.is_some() {
            let (tx, rx) = tokio::sync::oneshot::channel();
            controller
                .request_sender()
                .send(NewClientRequest {
                    request_id,
                    headers: vec![
                        h3::Header::new(b":method", method.as_bytes()),
                        h3::Header::new(b":path", path.as_bytes()),
                        h3::Header::new(b":scheme", b"https"),
                        h3::Header::new(b":authority", b"localhost"),
                    ],
                    body_writer: Some(tx),
                })
                .expect("failed to enqueue request");
            Some(rx)
        } else {
            controller
                .request_sender()
                .send(NewClientRequest {
                    request_id,
                    headers: vec![
                        h3::Header::new(b":method", method.as_bytes()),
                        h3::Header::new(b":path", path.as_bytes()),
                        h3::Header::new(b":scheme", b"https"),
                        h3::Header::new(b":authority", b"localhost"),
                    ],
                    body_writer: None,
                })
                .expect("failed to enqueue request");
            None
        };

        if let Some(body) = body {
            let mut outbound = body_writer_rx
                .unwrap()
                .await
                .expect("body_writer sender dropped before sending");
            outbound
                .send(OutboundFrame::Body(body.into(), true))
                .await
                .expect("failed to write request body");
        }

        Self::recv_response(controller).await
    }

    /// Like [`send`], but splits the body into fixed-size chunks sent as
    /// separate DATA frames.  The last chunk carries FIN.  This exercises the
    /// `(true, false)` arm in the controller's `BodyBytesReceived` handler.
    pub async fn send_chunked(
        &mut self,
        method: Method,
        path: &str,
        body: impl Into<Bytes>,
        chunk_size: usize,
        request_id: u64,
    ) -> TestResponse {
        assert!(chunk_size > 0, "chunk_size must be > 0");

        if self.server_handle.is_none() {
            self.start().await;
        }
        if self.client.is_none() {
            self.client = Some(self.new_client().await);
        }
        let (_, controller) = self.client.as_mut().unwrap();

        let (tx, rx) = tokio::sync::oneshot::channel();
        controller
            .request_sender()
            .send(NewClientRequest {
                request_id,
                headers: vec![
                    h3::Header::new(b":method", method.as_bytes()),
                    h3::Header::new(b":path", path.as_bytes()),
                    h3::Header::new(b":scheme", b"https"),
                    h3::Header::new(b":authority", b"localhost"),
                ],
                body_writer: Some(tx),
            })
            .expect("failed to enqueue request");

        let body_bytes: Bytes = body.into();
        // Spawn the chunked body writes in a background task so they can
        // make progress concurrently with the QUIC driver (which may need
        // the client to poll events before accepting more data).
        tokio::spawn(async move {
            let mut outbound = rx.await.expect("body_writer sender dropped");
            let total = body_bytes.len();
            let mut offset = 0;
            while offset < total {
                let end = (offset + chunk_size).min(total);
                let fin = end == total;
                let chunk = body_bytes.slice(offset..end);
                outbound
                    .send(OutboundFrame::Body(chunk, fin))
                    .await
                    .expect("failed to write body chunk");
                offset = end;
            }
        });

        Self::recv_response(controller).await
    }

    /// Sends multiple requests concurrently on a single QUIC connection and
    /// collects all responses.  Each entry is `(method, path, body)`.
    /// Returns responses in the order they are received from the server
    /// (which may differ from the send order).
    pub async fn send_concurrent(
        &mut self,
        requests: Vec<(Method, &str, Option<&[u8]>)>,
    ) -> Vec<TestResponse> {
        if self.server_handle.is_none() {
            self.start().await;
        }
        if self.client.is_none() {
            self.client = Some(self.new_client().await);
        }
        let (_, controller) = self.client.as_mut().unwrap();

        let count = requests.len();

        // Enqueue all requests before waiting for any response.
        for (idx, (method, path, body)) in requests.into_iter().enumerate() {
            let request_id = idx as u64;
            if let Some(body) = body {
                let (tx, rx) = tokio::sync::oneshot::channel();
                controller
                    .request_sender()
                    .send(NewClientRequest {
                        request_id,
                        headers: vec![
                            h3::Header::new(b":method", method.as_bytes()),
                            h3::Header::new(b":path", path.as_bytes()),
                            h3::Header::new(b":scheme", b"https"),
                            h3::Header::new(b":authority", b"localhost"),
                        ],
                        body_writer: Some(tx),
                    })
                    .expect("failed to enqueue request");
                let body = Bytes::copy_from_slice(body);
                tokio::spawn(async move {
                    let mut outbound = rx.await.expect("body_writer sender dropped");
                    outbound
                        .send(OutboundFrame::Body(body, true))
                        .await
                        .expect("failed to write request body");
                });
            } else {
                controller
                    .request_sender()
                    .send(NewClientRequest {
                        request_id,
                        headers: vec![
                            h3::Header::new(b":method", method.as_bytes()),
                            h3::Header::new(b":path", path.as_bytes()),
                            h3::Header::new(b":scheme", b"https"),
                            h3::Header::new(b":authority", b"localhost"),
                        ],
                        body_writer: None,
                    })
                    .expect("failed to enqueue request");
            }
        }

        // Collect all responses.
        let mut responses = Vec::with_capacity(count);
        for _ in 0..count {
            responses.push(Self::recv_response(controller).await);
        }
        responses
    }

    pub async fn recv_response(controller: &mut ClientH3Controller) -> TestResponse {
        loop {
            match controller.event_receiver_mut().recv().await {
                Some(ClientH3Event::Core(H3Event::IncomingHeaders(incoming))) => {
                    let mut seen_regular = false;
                    for h in &incoming.headers {
                        if h.name().starts_with(b":") {
                            assert!(
                                !seen_regular,
                                "pseudo-header '{}' appears after regular headers",
                                String::from_utf8_lossy(h.name()),
                            );
                        } else {
                            seen_regular = true;
                        }
                    }

                    let status = incoming
                        .headers
                        .iter()
                        .find(|h| h.name() == b":status")
                        .and_then(|h| std::str::from_utf8(h.value()).ok())
                        .and_then(|s| s.parse::<u16>().ok())
                        .unwrap_or(0);

                    let mut body_bytes = Vec::new();
                    if !incoming.read_fin {
                        let mut recv = incoming.recv;
                        while let Some(InboundFrame::Body(chunk, fin)) = recv.recv().await {
                            body_bytes.extend_from_slice(&chunk);
                            if fin {
                                break;
                            }
                        }
                    }
                    let status = Status::try_from(status).expect("HTTP status could not valid");

                    let headers = incoming
                        .headers
                        .iter()
                        .map(|h| {
                            (
                                String::from_utf8(h.name().to_owned())
                                    .expect("Header name not valid utf8"),
                                String::from_utf8(h.value().to_owned())
                                    .expect("Header value not valid utf8"),
                            )
                        })
                        .collect::<HashMap<String, String>>();

                    return TestResponse {
                        status,
                        headers,
                        body_bytes,
                    };
                }
                Some(event) => {
                    trace!(?event, "H3 Client Event - unhandled");
                    continue;
                }
                None => panic!("H3 event stream closed unexpectedly"),
            }
        }
    }

    fn logging() {
        static ONCE_INIT: OnceLock<()> = OnceLock::new();

        ONCE_INIT.get_or_init(|| {
            if std::env::var("TEST_LOG").is_ok() {
                tracing_subscriber::fmt()
                    .with_env_filter(tracing_subscriber::EnvFilter::new("protest=trace"))
                    .with_target(true)
                    .init();
            }
        });
    }

    pub fn server(&mut self) -> &mut Server {
        self.server
            .as_mut()
            .expect("server had started and not replaced")
    }
}

impl AsyncTestContext for IntegrationTest {
    fn setup() -> impl Future<Output = Self> + Send {
        Self::logging();
        Self::new()
    }

    async fn teardown(self) {
        if let Some(handle) = self.server_handle {
            handle.abort();
        }
    }
}

#[derive(Clone, Eq, PartialEq)]
pub struct TestResponse {
    pub status: Status,
    pub headers: HashMap<String, String>,
    pub body_bytes: Vec<u8>,
}

/// Asserts that the given response matches the expected status, headers, and body.
///
/// # Arguments
///
/// * *Response* - The response to assert (Must me a variable name i.e. `res`)
/// * *Status* - The expected status. i.g. `Ok` or `NotFound`.
/// * *Headers* - The expected headers to assert over, if headers are present but but stated they will be ignored. i.e. `[("Content-Type", "application/json")]`, to make no header assertions, give `[]`.
/// * *Body* - The expected body. Any expression that can be converted to `bytes::Bytes`.
///
/// # Examples
///
/// **String without header assertions**
/// ```ignore
/// assert_response!(res, Ok, [], "Hello, World!");
/// ```
///
/// **Plain Text**
/// ```ignore
/// assert_response!(res, Ok, [("Content-Type", "text/plain")], "Hello, World!");
/// ```
///
/// **Json**
/// ```ignore
/// assert_response!(res, Ok, [("Content-Type", "application/json")], "{}");
/// ```
#[macro_export]
macro_rules! assert_response {
    ($res:ident, $status:ident, [$(($header_name:literal, $header_value:literal)),*]) => {
        assert_response!($res, $status, [$(($header_name, $header_value)),*], "");
    };
    ($res:ident, $status:ident, [$(($header_name:literal, $header_value:literal)),*], $body:expr) => {{

        assert_eq!($res.status, $crate::Status::$status);


        $(
            assert_eq!($res.headers.get($header_name).cloned(), Some($header_value));
        )*

        let expected = bytes::Bytes::from_owner($body);
        if $res.body_bytes != expected {
            let expected_as_string = String::from_utf8_lossy(&expected).to_string();
            let actual_as_string = String::from_utf8_lossy(&$res.body_bytes).to_string();
            panic!(
                "body mismatch: expected `{}`, got `{}`",
                expected_as_string, actual_as_string
            );
        }
    }};
}
