use crate::*;
use std::path::PathBuf;

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

    let mut router = Router::default();
    router.add_route(TypedRoute {
        method: Method::POST,
        path: PathBuf::from("/"),
        handler: RouteHandler::Sync(|x: Request<String>| Response::<String> {
            status: Status::OK,
            headers: ResponseHeaders::default(),
            body: x.body,
        }),
    });
    let mut server = Server::new();
    server.nest_routes(router);
    server.start().await
}
