# NATS Manager

A Linux and macOS desktop application for inspecting and administering existing NATS clusters, with or without JetStream.

## Initial scope

- Connect to existing NATS clusters and import connection configuration.
- Support password, NKey/JWT, `.creds`, TLS, and mTLS authentication.
- Store credentials only in the operating system keychain/secret service.
- Inspect cluster status and observability data.
- Manage JetStream streams and consumers, publish and inspect messages, and perform replay/purge operations.
- Require typing the exact resource name before destructive operations such as stream/consumer deletion or message purge.

## Development

This project uses Cargo for Rust dependencies and tooling. Start the desktop app with:

```sh
cargo run
```

The app currently includes connection profiles, secure credential storage/import, password and NATS credentials-file authentication, TLS/mTLS with PEM material stored in the system keychain, JetStream detection, stream and durable-consumer management, message publishing/inspection/replay/purge, and exact-name confirmation for destructive JetStream actions.

NATS integration tests can run against local servers by setting `NATS_URL` to a JetStream-enabled server and `NATS_NO_JS_URL` to a server without JetStream before running `cargo test --test nats_integration`. Without those variables the integration tests return without connecting.
