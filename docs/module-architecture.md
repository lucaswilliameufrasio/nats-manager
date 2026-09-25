# Application module architecture

NATS Manager is organized around user-facing capabilities, not around individual
egui widgets or NATS API calls. New product work should deepen one capability at
a time and keep its user intent, state, operations, and failure feedback together.

## Capability map

| Capability | User intent | Current domain implementation |
| --- | --- | --- |
| Connections | Connect to clusters, create/import profiles, configure authentication and TLS | `profile`, `connection`, `context_import`, `credentials` |
| Cluster overview | Inspect connection health, server identity, and JetStream availability/usage | `connection::JetStreamStatus` and server information from `async_nats` |
| JetStream | Browse streams and consumers, inspect configuration and stored messages, create or edit resources | `jetstream` |
| Messaging | Publish core NATS or JetStream messages and replay inspected messages | `jetstream::publish` and `jetstream::replay_message` |
| Maintenance | Delete streams/consumers or purge stored messages | `jetstream` operations guarded by `safety::confirms_resource_name` |

The desktop shell gives each capability a distinct navigation destination. Keep
routine operations and read-only inspection separate from destructive maintenance;
the maintenance screen must retain exact-name confirmation for every destructive
action.

## Module contract for future work

Before adding a capability, record:

- the domain entities it owns and the commands/queries it supports;
- the state and prerequisites required to perform each operation;
- dependencies on connection, credentials, or other capabilities;
- validation, authorization/safety checks, and user-visible failure states;
- the observable behavior to verify in unit and NATS integration tests.

Keep NATS protocol behavior in domain modules and UI state/rendering in the
application layer. Parse and validate imported data before storing it. Keep secret
material in the OS credential store, never in profile serialization or logs.
Long-running NATS operations remain asynchronous and must report a clear pending,
success, or failure state to the user. Do not add a generic NATS request proxy;
expose explicit domain operations instead.

## Current extraction status

`src/app.rs` currently owns app state, operation dispatch, and rendering for all
capabilities. Its navigation and page topology now separate connection setup,
overview, JetStream work, publishing, and maintenance. As capabilities grow,
extract their app-side state and view/intent handling into `src/app/` modules while
keeping the existing domain modules as the protocol-facing implementation. Avoid
moving code into files without a coherent capability interface; use the deletion
test to ensure an extracted module actually concentrates behavior and tests.
