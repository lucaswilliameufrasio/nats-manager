# Changelog

All notable changes to NATS Manager will be documented in this file.

## [0.1.2] - 2026-09-25

### Features

- *(ui)* Introduce capability-based navigation
## [0.1.1] - 2026-09-24

### Bug Fixes

- *(release)* Locate universal macOS bundles

### CI / Build

- *(release)* Publish native desktop installers

### Documentation

- *(macos)* Document Gatekeeper warning and signing

### Features

- *(branding)* Add NATS Manager logo and bundle icon
## [0.1.0] - 2026-09-23

### Bug Fixes

- *(jetstream)* Preserve headers when replaying messages

### CI / Build

- Add Linux and macOS packaging checks
- *(release)* Build Linux and macOS release artifacts

### Features

- Initialize NATS Manager desktop shell
- *(profiles)* Persist connection profiles and keychain secrets
- *(connection)* Authenticate profiles and detect JetStream
- *(jetstream)* Manage streams and durable consumers
- *(jetstream)* Publish inspect replay and purge messages
- *(jetstream)* Edit stream and consumer settings
- *(connection)* Secure TLS identities in system keychain
- *(auth)* Import NATS contexts and auth methods
- *(jetstream)* Honor context API prefixes and domains
