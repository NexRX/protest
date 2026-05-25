use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use bytes::Bytes;
use futures_util::SinkExt as _;
use quiche::h3;
use quiche::h3::NameValue as _;
use tokio::net::UdpSocket;
use tokio_quiche::ClientH3Controller;
use tokio_quiche::http3::driver::{
    ClientH3Event, H3Event, InboundFrame, NewClientRequest, OutboundFrame,
};

use crate::{
    Method, Request, Response, ResponseHeaders, RouteHandler, Router, Server, Status, TypedRoute,
};

async fn spawn_server(router: Router) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let addr = UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("failed to bind server socket")
        .local_addr()
        .expect("failed to get server local addr");

    let mut server = Server::new();
    server.with_address(addr);
    server.nest_routes(router);

    let handle = tokio::spawn(async move {
        let _ = server.start().await;
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    (addr, handle)
}

async fn connect_client(
    server_addr: SocketAddr,
) -> (tokio_quiche::QuicConnection, ClientH3Controller) {
    let socket = UdpSocket::bind("127.0.0.1:0")
        .await
        .expect("failed to bind client socket");
    socket
        .connect(server_addr)
        .await
        .expect("failed to connect client socket");

    tokio_quiche::quic::connect(socket, Some("localhost"))
        .await
        .expect("QUIC/H3 handshake failed")
}

struct TestResponse {
    status: u16,
    body_bytes: Vec<u8>,
}

async fn send_post(
    controller: &mut ClientH3Controller,
    path: &str,
    body: Bytes,
    request_id: u64,
) -> TestResponse {
    let (body_writer_tx, body_writer_rx) = tokio::sync::oneshot::channel();

    controller
        .request_sender()
        .send(NewClientRequest {
            request_id,
            headers: vec![
                h3::Header::new(b":method", b"POST"),
                h3::Header::new(b":path", path.as_bytes()),
                h3::Header::new(b":scheme", b"https"),
                h3::Header::new(b":authority", b"localhost"),
            ],
            body_writer: Some(body_writer_tx),
        })
        .expect("failed to enqueue request");

    let mut outbound = body_writer_rx
        .await
        .expect("body_writer sender dropped before sending");
    outbound
        .send(OutboundFrame::Body(body, true))
        .await
        .expect("failed to write request body");

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
            Some(_) => continue,
            None => panic!("H3 event stream closed unexpectedly"),
        }
    }
}

#[tokio::test]
async fn post_echo_returns_same_body() {
    let mut router = Router::default();
    router.add_route(TypedRoute {
        method: Method::POST,
        path: PathBuf::from("/echo"),
        handler: RouteHandler::Sync(|req: Request<String>| Response::<String> {
            status: Status::OK,
            headers: ResponseHeaders::default(),
            body: req.body,
        }),
    });

    let (addr, server_handle) = spawn_server(router).await;
    let (_conn, mut client) = connect_client(addr).await;

    let resp = send_post(
        &mut client,
        "/echo",
        Bytes::from_static(b"\"hello world\""),
        1,
    )
    .await;

    assert_eq!(resp.status, 200);
    assert_eq!(resp.body_bytes, b"\"hello world\"");

    server_handle.abort();
}

#[tokio::test]
async fn unknown_route_returns_404() {
    let mut router = Router::default();
    router.add_route(TypedRoute {
        method: Method::POST,
        path: PathBuf::from("/exists"),
        handler: RouteHandler::Sync(|_req: Request<String>| Response::<String> {
            status: Status::OK,
            headers: ResponseHeaders::default(),
            body: "ok".to_string(),
        }),
    });

    let (addr, server_handle) = spawn_server(router).await;
    let (_conn, mut client) = connect_client(addr).await;

    let resp = send_post(
        &mut client,
        "/does-not-exist",
        Bytes::from_static(b"\"body\""),
        1,
    )
    .await;
    assert_eq!(resp.status, 404);

    server_handle.abort();
}

#[tokio::test]
async fn handler_can_return_created_201() {
    let mut router = Router::default();
    router.add_route(TypedRoute {
        method: Method::POST,
        path: PathBuf::from("/create"),
        handler: RouteHandler::Sync(|req: Request<String>| Response::<String> {
            status: Status::Created,
            headers: ResponseHeaders::default(),
            body: req.body,
        }),
    });

    let (addr, server_handle) = spawn_server(router).await;
    let (_conn, mut client) = connect_client(addr).await;

    let resp = send_post(&mut client, "/create", Bytes::from_static(b"\"item\""), 1).await;
    assert_eq!(resp.status, 201);
    assert_eq!(resp.body_bytes, b"\"item\"");

    server_handle.abort();
}

#[tokio::test]
async fn json_object_body_round_trips() {
    let mut router = Router::default();
    router.add_route(TypedRoute {
        method: Method::POST,
        path: PathBuf::from("/json"),
        handler: RouteHandler::Sync(|req: Request<serde_json::Value>| Response {
            status: Status::OK,
            headers: ResponseHeaders::default(),
            body: req.body,
        }),
    });

    let (addr, server_handle) = spawn_server(router).await;
    let (_conn, mut client) = connect_client(addr).await;

    let json_payload = br#"{"key":"value","num":42}"#;
    let resp = send_post(&mut client, "/json", Bytes::from_static(json_payload), 1).await;
    assert_eq!(resp.status, 200);

    let expected: serde_json::Value = serde_json::from_slice(json_payload).unwrap();
    let actual: serde_json::Value =
        serde_json::from_slice(&resp.body_bytes).expect("response body is not valid JSON");
    assert_eq!(expected, actual);

    server_handle.abort();
}

#[tokio::test]
async fn sequential_requests_on_same_connection() {
    let mut router = Router::default();
    router.add_route(TypedRoute {
        method: Method::POST,
        path: PathBuf::from("/echo"),
        handler: RouteHandler::Sync(|req: Request<String>| Response::<String> {
            status: Status::OK,
            headers: ResponseHeaders::default(),
            body: req.body,
        }),
    });

    let (addr, server_handle) = spawn_server(router).await;
    let (_conn, mut client) = connect_client(addr).await;

    let resp1 = send_post(&mut client, "/echo", Bytes::from_static(b"\"first\""), 1).await;
    assert_eq!(resp1.status, 200);
    assert_eq!(resp1.body_bytes, b"\"first\"");

    let resp2 = send_post(&mut client, "/echo", Bytes::from_static(b"\"second\""), 2).await;
    assert_eq!(resp2.status, 200);
    assert_eq!(resp2.body_bytes, b"\"second\"");

    server_handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn curl_http3_post_echo() {
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

    let mut router = Router::default();
    router.add_route(TypedRoute {
        method: Method::POST,
        path: PathBuf::from("/echo"),
        handler: RouteHandler::Sync(|req: Request<String>| Response::<String> {
            status: Status::OK,
            headers: ResponseHeaders::default(),
            body: req.body,
        }),
    });

    let (addr, server_handle) = spawn_server(router).await;

    let resolve = format!("localhost:{}:127.0.0.1", addr.port());
    let url = format!("https://localhost:{}/echo", addr.port());

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

    server_handle.abort();
}

#[tokio::test]
async fn async_handler_is_dispatched() {
    let mut router = Router::default();
    router.add_route(TypedRoute {
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

    let (addr, server_handle) = spawn_server(router).await;
    let (_conn, mut client) = connect_client(addr).await;

    let resp = send_post(
        &mut client,
        "/async-echo",
        Bytes::from_static(b"\"async body\""),
        1,
    )
    .await;
    assert_eq!(resp.status, 200);
    assert_eq!(resp.body_bytes, b"\"async body\"");

    server_handle.abort();
}
