// #[test_context(IntegrationTest)]
// #[tokio::test]
// async fn post_echo_returns_same_body(test: &mut IntegrationTest) {
//     let mut router = Router::<()>::default();
//     router.add(TypedRoute {
//         method: Method::POST,
//         path: PathBuf::from("/echo"),
//         handler: RouteHandler::Sync(|req: Request<String>| Response::<String> {
//             status: Status::OK,
//             headers: ResponseHeaders::default(),
//             body: req.body,
//         }),
//     });
//     test.server().routes(router);

//     let resp = test
//         .send(
//             Method::POST,
//             "/echo",
//             Some(Bytes::from_static(b"\"hello world\"")),
//             1,
//         )
//         .await;

//     assert_eq!(resp.status, 200);
//     assert_eq!(resp.body_bytes, b"\"hello world\"");
// }

// #[test_context(IntegrationTest)]
// #[tokio::test]
// async fn unknown_route_returns_404(test: &mut IntegrationTest) {
//     let mut router = Router::<()>::default();
//     router.add(TypedRoute {
//         method: Method::POST,
//         path: PathBuf::from("/exists"),
//         handler: RouteHandler::Sync(|_req: Request<String>| Response::<String> {
//             status: Status::OK,
//             headers: ResponseHeaders::default(),
//             body: "ok".to_string(),
//         }),
//     });

//     let resp = test
//         .send(
//             Method::POST,
//             "/does-not-exist",
//             Some(Bytes::from_static(b"\"body\"")),
//             1,
//         )
//         .await;
//     assert_eq!(resp.status, 404);
// }

// #[test_context(IntegrationTest)]
// #[tokio::test]
// async fn handler_can_return_created_201(test: &mut IntegrationTest) {
//     let mut router = Router::<()>::default();
//     router.add(TypedRoute {
//         method: Method::POST,
//         path: PathBuf::from("/create"),
//         handler: RouteHandler::Sync(|req: Request<String>| Response::<String> {
//             status: Status::Created,
//             headers: ResponseHeaders::default(),
//             body: req.body,
//         }),
//     });
//     test.server().routes(router);

//     let resp = test
//         .send(
//             Method::POST,
//             "/create",
//             Some(Bytes::from_static(b"\"item\"")),
//             1,
//         )
//         .await;
//     assert_eq!(resp.status, 201);
//     assert_eq!(resp.body_bytes, b"\"item\"");
// }

// #[test_context(IntegrationTest)]
// #[tokio::test]
// async fn json_object_body_round_trips(test: &mut IntegrationTest) {
//     let mut router = Router::<()>::default();
//     router.add(TypedRoute {
//         method: Method::POST,
//         path: PathBuf::from("/json"),
//         handler: RouteHandler::Sync(|req: Request<serde_json::Value>| Response {
//             status: Status::OK,
//             headers: ResponseHeaders::default(),
//             body: req.body,
//         }),
//     });
//     test.server().routes(router);

//     let json_payload = br#"{"key":"value","num":42}"#;
//     let resp = test
//         .send(
//             Method::POST,
//             "/json",
//             Some(Bytes::from_static(json_payload)),
//             1,
//         )
//         .await;
//     assert_eq!(resp.status, 200);

//     let expected: serde_json::Value = serde_json::from_slice(json_payload).unwrap();
//     let actual: serde_json::Value =
//         serde_json::from_slice(&resp.body_bytes).expect("response body is not valid JSON");
//     assert_eq!(expected, actual);
// }

// #[test_context(IntegrationTest)]
// #[tokio::test]
// async fn sequential_requests_on_same_connection(test: &mut IntegrationTest) {
//     let mut router = Router::<()>::default();
//     router.add(TypedRoute {
//         method: Method::POST,
//         path: PathBuf::from("/echo"),
//         handler: RouteHandler::Sync(|req: Request<String>| Response::<String> {
//             status: Status::OK,
//             headers: ResponseHeaders::default(),
//             body: req.body,
//         }),
//     });
//     test.server().routes(router);

//     let resp1 = test
//         .send(
//             Method::POST,
//             "/echo",
//             Some(Bytes::from_static(b"\"first\"")),
//             1,
//         )
//         .await;
//     assert_eq!(resp1.status, 200);
//     assert_eq!(resp1.body_bytes, b"\"first\"");

//     let resp2 = test
//         .send(
//             Method::POST,
//             "/echo",
//             Some(Bytes::from_static(b"\"second\"")),
//             2,
//         )
//         .await;
//     assert_eq!(resp2.status, 200);
//     assert_eq!(resp2.body_bytes, b"\"second\"");
// }

// #[test_context(IntegrationTest)]
// #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
// async fn curl_http3_post_echo(test: &mut IntegrationTest) {
//     let _ = tracing_subscriber::fmt()
//         .with_env_filter(tracing_subscriber::EnvFilter::new("protest=debug"))
//         .try_init();

//     let ver_out = tokio::process::Command::new("curl")
//         .arg("--version")
//         .output()
//         .await;
//     let Ok(ver_out) = ver_out else {
//         eprintln!("curl not found – skipping curl integration test");
//         return;
//     };

//     let ver_str = String::from_utf8_lossy(&ver_out.stdout);
//     if !ver_str.contains("HTTP3") {
//         eprintln!("curl has no HTTP/3 support – skipping (features: {ver_str})");
//         return;
//     }

//     let mut router = Router::<()>::default();
//     router.add(TypedRoute {
//         method: Method::POST,
//         path: PathBuf::from("/echo"),
//         handler: RouteHandler::Sync(|req: Request<String>| Response::<String> {
//             status: Status::OK,
//             headers: ResponseHeaders::default(),
//             body: req.body,
//         }),
//     });

//     test.start().await;

//     let resolve = format!("localhost:{}:127.0.0.1", test.addr().port());
//     let url = format!("https://localhost:{}/echo", test.addr().port());

//     let out = tokio::process::Command::new("curl")
//         .args([
//             "--http3-only",
//             "--insecure",
//             "--verbose",
//             "--fail",
//             "--max-time",
//             "10",
//             "--request",
//             "POST",
//             "--data-raw",
//             r#""hello curl""#,
//             "--header",
//             "content-type: application/json",
//             "--resolve",
//             &resolve,
//             &url,
//         ])
//         .output()
//         .await
//         .expect("failed to spawn curl");

//     assert!(
//         out.status.success(),
//         "curl exited with {}\nstdout: {}\nstderr: {}",
//         out.status,
//         String::from_utf8_lossy(&out.stdout),
//         String::from_utf8_lossy(&out.stderr),
//     );

//     let body = String::from_utf8_lossy(&out.stdout);
//     assert_eq!(body.trim(), r#""hello curl""#);
// }

// #[test_context(IntegrationTest)]
// #[tokio::test]
// async fn async_handler_is_dispatched(test: &mut IntegrationTest) {
//     let mut router = Router::<()>::default();
//     router.add(TypedRoute {
//         method: Method::POST,
//         path: PathBuf::from("/async-echo"),
//         handler: RouteHandler::Async(Box::new(|req: Request<String>| {
//             Box::pin(async move {
//                 tokio::time::sleep(Duration::from_millis(1)).await;
//                 Response::<String> {
//                     status: Status::OK,
//                     headers: ResponseHeaders::default(),
//                     body: req.body,
//                 }
//             })
//         })),
//     });
//     test.server().routes(router);

//     let resp = test
//         .send(Method::POST, "/async-echo", Some("\"async body\""), 1)
//         .await;
//     assert_eq!(resp.status, 200);
//     assert_eq!(resp.body_bytes, b"\"async body\"");
// }

// /// Starts a server for manual testing and does nothing if not specifically ran
// #[tokio::test]
// async fn start_server() -> Result<(), Box<dyn std::error::Error>> {
//     // skip test if it wasnt specifically ran (starts real server)
//     let is_targeted = std::env::args().any(|a| a == "start_server");
//     if !is_targeted {
//         return Ok(());
//     }

//     tracing_subscriber::fmt()
//         .with_env_filter(tracing_subscriber::EnvFilter::new("protest=trace"))
//         .with_target(true)
//         .init();

//     let mut router = Router::<()>::default();
//     router.add(TypedRoute {
//         method: Method::POST,
//         path: PathBuf::from("/"),
//         handler: RouteHandler::Sync(|x: Request<String>| Response::<String> {
//             status: Status::OK,
//             headers: ResponseHeaders::default(),
//             body: x.body,
//         }),
//     });
//     let mut server = Server::new();
//     server.routes(router);
//     server.start().await
// }
