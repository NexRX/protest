use crate::{
    Method, Request, Response, ResponseHeaders, RouteHandler, Router, Server, Status, TypedRoute,
};
use bytes::Bytes;
use futures_util::SinkExt as _;
use quiche::h3;
use quiche::h3::NameValue as _;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;
use test_context::{AsyncTestContext, test_context};
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
        server.with_address(addr.clone());
        Self {
            server: Some(server),
            addr,
            server_handle: None,
            client: None,
        }
    }

    async fn start(&mut self) {
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
            if let Ok(_) = UdpSocket::bind(&self.addr).await {
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
        let (body_writer_tx, body_writer_rx) = tokio::sync::oneshot::channel();

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
                body_writer: Some(body_writer_tx),
            })
            .expect("failed to enqueue request");

        if let Some(body) = body {
            let mut outbound = body_writer_rx
                .await
                .expect("body_writer sender dropped before sending");
            outbound
                .send(OutboundFrame::Body(body.into(), true))
                .await
                .expect("failed to write request body");
        }

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

                    return TestResponse { status, body_bytes };
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
            tracing_subscriber::fmt()
                .with_env_filter(tracing_subscriber::EnvFilter::new("protest=trace"))
                .with_target(true)
                .init();
        });
    }

    pub fn server<'a>(&'a mut self) -> &'a mut Server {
        self.server
            .as_mut()
            .expect("server had started and not replaced")
    }

    pub fn addr<'a>(&'a self) -> &'a SocketAddr {
        &self.addr
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
    pub status: u16,
    pub body_bytes: Vec<u8>,
}

#[test_context(IntegrationTest)]
#[tokio::test]
async fn post_echo_returns_same_body(test: &mut IntegrationTest) {
    let mut router = Router::<()>::default();
    router.add(TypedRoute {
        method: Method::POST,
        path: PathBuf::from("/echo"),
        handler: RouteHandler::Sync(|req: Request<String>| Response::<String> {
            status: Status::OK,
            headers: ResponseHeaders::default(),
            body: req.body,
        }),
    });
    test.server().routes(router);

    let resp = test
        .send(
            Method::POST,
            "/echo",
            Some(Bytes::from_static(b"\"hello world\"")),
            1,
        )
        .await;

    assert_eq!(resp.status, 200);
    assert_eq!(resp.body_bytes, b"\"hello world\"");
}

#[test_context(IntegrationTest)]
#[tokio::test]
async fn unknown_route_returns_404(test: &mut IntegrationTest) {
    let mut router = Router::<()>::default();
    router.add(TypedRoute {
        method: Method::POST,
        path: PathBuf::from("/exists"),
        handler: RouteHandler::Sync(|_req: Request<String>| Response::<String> {
            status: Status::OK,
            headers: ResponseHeaders::default(),
            body: "ok".to_string(),
        }),
    });

    let resp = test
        .send(
            Method::POST,
            "/does-not-exist",
            Some(Bytes::from_static(b"\"body\"")),
            1,
        )
        .await;
    assert_eq!(resp.status, 404);
}

#[test_context(IntegrationTest)]
#[tokio::test]
async fn handler_can_return_created_201(test: &mut IntegrationTest) {
    let mut router = Router::<()>::default();
    router.add(TypedRoute {
        method: Method::POST,
        path: PathBuf::from("/create"),
        handler: RouteHandler::Sync(|req: Request<String>| Response::<String> {
            status: Status::Created,
            headers: ResponseHeaders::default(),
            body: req.body,
        }),
    });
    test.server().routes(router);

    let resp = test
        .send(
            Method::POST,
            "/create",
            Some(Bytes::from_static(b"\"item\"")),
            1,
        )
        .await;
    assert_eq!(resp.status, 201);
    assert_eq!(resp.body_bytes, b"\"item\"");
}

#[test_context(IntegrationTest)]
#[tokio::test]
async fn json_object_body_round_trips(test: &mut IntegrationTest) {
    let mut router = Router::<()>::default();
    router.add(TypedRoute {
        method: Method::POST,
        path: PathBuf::from("/json"),
        handler: RouteHandler::Sync(|req: Request<serde_json::Value>| Response {
            status: Status::OK,
            headers: ResponseHeaders::default(),
            body: req.body,
        }),
    });
    test.server().routes(router);

    let json_payload = br#"{"key":"value","num":42}"#;
    let resp = test
        .send(
            Method::POST,
            "/json",
            Some(Bytes::from_static(json_payload)),
            1,
        )
        .await;
    assert_eq!(resp.status, 200);

    let expected: serde_json::Value = serde_json::from_slice(json_payload).unwrap();
    let actual: serde_json::Value =
        serde_json::from_slice(&resp.body_bytes).expect("response body is not valid JSON");
    assert_eq!(expected, actual);
}

#[test_context(IntegrationTest)]
#[tokio::test]
async fn sequential_requests_on_same_connection(test: &mut IntegrationTest) {
    let mut router = Router::<()>::default();
    router.add(TypedRoute {
        method: Method::POST,
        path: PathBuf::from("/echo"),
        handler: RouteHandler::Sync(|req: Request<String>| Response::<String> {
            status: Status::OK,
            headers: ResponseHeaders::default(),
            body: req.body,
        }),
    });
    test.server().routes(router);

    let resp1 = test
        .send(
            Method::POST,
            "/echo",
            Some(Bytes::from_static(b"\"first\"")),
            1,
        )
        .await;
    assert_eq!(resp1.status, 200);
    assert_eq!(resp1.body_bytes, b"\"first\"");

    let resp2 = test
        .send(
            Method::POST,
            "/echo",
            Some(Bytes::from_static(b"\"second\"")),
            2,
        )
        .await;
    assert_eq!(resp2.status, 200);
    assert_eq!(resp2.body_bytes, b"\"second\"");
}

#[test_context(IntegrationTest)]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn curl_http3_post_echo(test: &mut IntegrationTest) {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("protest=debug"))
        .try_init();

    let ver_out = tokio::process::Command::new("curl")
        .arg("--version")
        .output()
        .await;
    let Ok(ver_out) = ver_out else {
        eprintln!("curl not found – skipping curl integration test");
        return;
    };

    let ver_str = String::from_utf8_lossy(&ver_out.stdout);
    if !ver_str.contains("HTTP3") {
        eprintln!("curl has no HTTP/3 support – skipping (features: {ver_str})");
        return;
    }

    let mut router = Router::<()>::default();
    router.add(TypedRoute {
        method: Method::POST,
        path: PathBuf::from("/echo"),
        handler: RouteHandler::Sync(|req: Request<String>| Response::<String> {
            status: Status::OK,
            headers: ResponseHeaders::default(),
            body: req.body,
        }),
    });

    test.start().await;

    let resolve = format!("localhost:{}:127.0.0.1", test.addr().port());
    let url = format!("https://localhost:{}/echo", test.addr().port());

    let out = tokio::process::Command::new("curl")
        .args([
            "--http3-only",
            "--insecure",
            "--verbose",
            "--fail",
            "--max-time",
            "10",
            "--request",
            "POST",
            "--data-raw",
            r#""hello curl""#,
            "--header",
            "content-type: application/json",
            "--resolve",
            &resolve,
            &url,
        ])
        .output()
        .await
        .expect("failed to spawn curl");

    assert!(
        out.status.success(),
        "curl exited with {}\nstdout: {}\nstderr: {}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    let body = String::from_utf8_lossy(&out.stdout);
    assert_eq!(body.trim(), r#""hello curl""#);
}

#[test_context(IntegrationTest)]
#[tokio::test]
async fn async_handler_is_dispatched(test: &mut IntegrationTest) {
    let mut router = Router::<()>::default();
    router.add(TypedRoute {
        method: Method::POST,
        path: PathBuf::from("/async-echo"),
        handler: RouteHandler::Async(Box::new(|req: Request<String>| {
            Box::pin(async move {
                tokio::time::sleep(Duration::from_millis(1)).await;
                Response::<String> {
                    status: Status::OK,
                    headers: ResponseHeaders::default(),
                    body: req.body,
                }
            })
        })),
    });
    test.server().routes(router);

    let resp = test
        .send(Method::POST, "/async-echo", Some("\"async body\""), 1)
        .await;
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body_bytes, b"\"async body\"");
}

/// Starts a server for manual testing and does nothing if not specifically ran
#[tokio::test]
async fn start_server() -> Result<(), Box<dyn std::error::Error>> {
    // skip test if it wasnt specifically ran (starts real server)
    let is_targeted = std::env::args().any(|a| a == "start_server");
    if !is_targeted {
        return Ok(());
    }

    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::new("protest=trace"))
        .with_target(true)
        .init();

    let mut router = Router::<()>::default();
    router.add(TypedRoute {
        method: Method::POST,
        path: PathBuf::from("/"),
        handler: RouteHandler::Sync(|x: Request<String>| Response::<String> {
            status: Status::OK,
            headers: ResponseHeaders::default(),
            body: x.body,
        }),
    });
    let mut server = Server::new();
    server.routes(router);
    server.start().await
}
