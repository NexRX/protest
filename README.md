# Protest 
*Dead simple HTTP/3 framework*

## Roadmap
- **Fluent Endpoint Definitions**
  - [ ] Path Params
  - [ ] Query Params
  - [ ] Headers
  - [ ] Request Body
  - [ ] Response Body
  - [ ] Errors *(Results)*
- **HTTP Routing**
  - [ ] Route Context *(Db Pools, Resources, Etc)*
  - [ ] Multiplexing
  - [ ] Middleware
  - [ ] Error Handling *(500 Fallback, 404 No Route, Endpoint Errors)*
- **Data Handling**
  - [ ] Streamed Request/Response Body
  - [ ] Buffered Request/Response Body
  - [ ] Typesafe Request Body/Params
  - [ ] Typesafe Response Body
- **Typesafe Client Generation**
  - [ ] JS/TS
  - [ ] Rust

## Testing

All tests:
```sh
cargo test --all-features
```

with logs set `TEST_LOG=1` environment variable:

```sh
TEST_LOG=1 cargo test --all-features
```

To run a specific test from `tests/macros`:

```sh
cargo test --test <filename> <test_name> --features test
# e.g.
cargo test --test router_codegen integration_test --features test
```

Protocol:
- Everything defined by code, models, APIs, everything.
- Transparent client interface. What the server returns is what the client gets. (No wrapping)
- Typesafety.
- HTTP/3 ?
