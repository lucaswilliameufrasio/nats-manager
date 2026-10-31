# Visual QA findings — 2026-10-30

Status: recorded, not fixed. These are local findings, not published tracker issues.

## Reproduction environment

- Revision: `nats-manager` 0.1.2 (release build of the checkout at the time); eframe
  0.36.2, glow, native Wayland.
- Private KWin 6.8.1 `--virtual` session on Linux with the real NVIDIA GPU (EGL probe in
  the same session reported the NVIDIA driver). Fresh private HOME/XDG, no saved profiles.
- Server: disposable `nats:2.12-alpine --jetstream` container on `127.0.0.1:14222` with one
  synthetic in-memory stream `QA`.
- Frames were captured and inspected. Screenshots stay in the operator's private scratch
  area, not in this repository.

## VQA-NATS-001 — Sidebar overlaps the main content on every page

**Priority:** high. **State:** open; reproduced at the default size and maximized.

**Reproduce:** launch; open Connections, JetStream and Overview in turn.

**Observed:** the left edge of the page header and body sits under the sidebar. Text reads
`USTER ACCESS`, `onnections`, `nnect securely…`, `EAMING MESSAGING`, `etStream`,
`luster overview`. The status badge at the top right is clipped (`NOT CONNECTE`,
`CONNECTE`) and the JetStream `Refresh` button is clipped (`Refres`). Also reproduced
maximized at 1360 px wide, so it is not only a small default window.

**Context:** `src/main.rs` uses `eframe::NativeOptions::default()` (no initial or minimum
window size). The default window is 800x600 and the sidebar is wide for it. The exact
layout cause is not diagnosed.

**Expected:** a sidebar and a content panel that do not overlap; a minimum window size that
fits the layout.

**Acceptance:** at the default size and at a larger size, headers and body text are fully
visible, the right-hand badge and action buttons are not clipped, and focus order still works.

## VQA-NATS-002 — Footer label collides with the navigation list

**Priority:** low. **State:** open; default size.

**Observed:** `NATS CLUSTER WORKSPACE` is drawn directly under, and touching, the
`Maintenance` item with no spacing.

## VQA-NATS-003 — Icon-like glyphs render as empty squares

**Priority:** medium. **State:** open; same class as sql-manager `VQA-001`.

**Observed:** Overview, JetStream, Publish and other nav icons show as empty square glyphs;
only `⚠` and `↔` render. Reproduces on native Wayland as well as X11 in sibling apps, so
it is not specific to a virtual X server. Cause not diagnosed; likely Unicode symbols outside
the bundled egui font coverage.

## Passed

- Quick connect to `nats://127.0.0.1:14222`: Overview shows "Connection healthy", server
  2.12.15, 1 stream, 0 consumers.
- JetStream lists stream `QA` with subjects `qa.>` and message/storage/consumer counts.

## Not covered

- Publish, consumer creation, message inspection, Maintenance (destructive) actions.
- Credentials, TLS and the keyring path.
- Light theme (the app forces dark), macOS, other compositors.
