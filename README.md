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

Build a release binary with `cargo build --release --locked`. To create a native bundle on the target OS, install the Cargo packaging tool with `cargo install cargo-bundle`, then run `cargo bundle --release`.

The app currently includes connection profiles, NATS CLI context and `.creds` import, password/token/NKey/JWT authentication, TLS/mTLS with imported PEM material stored in the system keychain, JetStream detection, stream and durable-consumer management, message publishing/inspection/replay/purge, and exact-name confirmation for destructive JetStream actions. Context imports do not execute external resolvers such as `env://`, `op://`, or `nsc://`; resolve those values before importing.

NATS integration tests can run against local servers by setting `NATS_URL` to a JetStream-enabled server and `NATS_NO_JS_URL` to a server without JetStream before running `cargo test --test nats_integration`. Without those variables the integration tests return without connecting.

GitHub Actions checks formatting, compilation, Clippy, and tests on Linux and macOS. The Linux job also runs the integration suite against local JetStream-enabled and core-only NATS servers.

## Releases

Use **Actions → Prepare Release → Run workflow** on `main` and enter a SemVer version. The workflow updates the changelog and Cargo version, then opens a release-preparation PR. After that PR is merged, create and push the matching `vX.Y.Z` tag. The `Release` workflow publishes cargo-dist archives for x86_64/ARM64 Linux and macOS, plus native Linux `.AppImage`/`.deb` and a universal macOS `.app.zip`/`.dmg`, each with checksums. For an existing release, **Actions → Bundle release installers** can attach the native installer files to its tag. If the tag does not yet have a GitHub Release, rerun **Release** for that tag first, then run **Bundle release installers**.

The macOS app is currently unsigned and not notarized; Gatekeeper may report it as damaged on first launch. See [macOS Gatekeeper and signing](docs/macos-gatekeeper.md) for the one-time workaround and the future signing setup.
