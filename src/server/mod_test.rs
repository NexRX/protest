use super::Server;
use crate::{Method, Request, Response, ResponseHeaders, Router, Status};
use std::net::SocketAddr;
use std::path::PathBuf;
use tokio_quiche::settings::CertificateKind;

#[test]
fn server_new_has_default_address() {
    let expected: SocketAddr = "0.0.0.0:4043".parse().unwrap();
    assert_eq!(Server::new().addr, expected);
}

#[test]
fn server_new_has_no_cert() {
    assert!(Server::new().cert.is_none());
}

#[test]
fn server_new_has_empty_router() {
    assert_eq!(Server::new().router.len(), 0);
}

#[test]
fn with_address_updates_addr() {
    let mut server = Server::new();
    let new_addr: SocketAddr = "127.0.0.1:8443".parse().unwrap();
    server.with_address(new_addr);
    assert_eq!(server.addr, new_addr);
}

#[test]
fn with_address_ipv6() {
    let mut server = Server::new();
    let addr: SocketAddr = "[::1]:4043".parse().unwrap();
    server.with_address(addr);
    assert_eq!(server.addr, addr);
}

#[test]
fn with_cert_stores_certificate_paths() {
    let mut server = Server::new();
    server.with_cert("key.pem", "cert.pem", CertificateKind::X509);
    let cert = server.cert.as_ref().expect("cert should be set");
    assert_eq!(cert.private_key, "key.pem");
    assert_eq!(cert.cert, "cert.pem");
    assert!(matches!(cert.kind, CertificateKind::X509));
}

#[test]
fn with_cert_overrides_previous_cert() {
    let mut server = Server::new();
    server.with_cert("old_key.pem", "old_cert.pem", CertificateKind::X509);
    server.with_cert("new_key.pem", "new_cert.pem", CertificateKind::X509);
    let cert = server.cert.as_ref().unwrap();
    assert_eq!(cert.private_key, "new_key.pem");
    assert_eq!(cert.cert, "new_cert.pem");
}

#[test]
fn add_route_increases_router_len() {
    use crate::{RouteHandler, TypedRoute};
    let mut router = Router::new(());
    router.add(TypedRoute {
        method: Method::GET,
        path: PathBuf::from("/"),
        handler: RouteHandler::Sync(|x: Request<String>| Response {
            status: Status::OK,
            headers: ResponseHeaders::default(),
            body: x.body,
        }),
    });

    let server = Server::new_with(router);
    assert_eq!(server.router.len(), 1);
}

#[test]
fn add_route_thats_async() {
    use crate::{RouteHandler, TypedRoute};
    let mut router = Router::new(());
    router.add(TypedRoute {
        method: Method::GET,
        path: PathBuf::from("/"),
        handler: RouteHandler::from(async |x: Request<String>| Response {
            status: Status::OK,
            headers: ResponseHeaders::default(),
            body: x.body,
        }),
    });

    let server = Server::new_with(router);
    assert_eq!(server.router.len(), 1);
}

#[test]
fn nest_routes_merges_router_into_server() {
    use crate::{RouteHandler, Router, TypedRoute};
    let mut server = Server::new();
    let mut extra = Router::new(());

    extra.add(TypedRoute {
        method: Method::POST,
        path: PathBuf::from("/submit"),
        handler: RouteHandler::Sync(|x: Request<String>| Response {
            status: Status::Created,
            headers: ResponseHeaders::default(),
            body: x.body,
        }),
    });
    extra.add(TypedRoute {
        method: Method::DELETE,
        path: PathBuf::from("/item"),
        handler: RouteHandler::Sync(|x: Request<String>| Response {
            status: Status::OK,
            headers: ResponseHeaders::default(),
            body: x.body,
        }),
    });

    server.routes(extra);
    assert_eq!(server.router.len(), 1);
    assert_eq!(server.router[0].len(), 2);
}
