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

The initial implementation provides the desktop shell, an asynchronous NATS connectivity check, and an operating-system credential-store abstraction.
