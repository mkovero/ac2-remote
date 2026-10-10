# Handoff to ac2 developers

## Current status: answered and implemented (2026-10-10)

No outstanding ac2 developer action for this refresh. The viewer now uses the
read-only pin `/work/ac2-pin/684d610` for all shared crates and the plot adapter.
The protocol-35 APK has been rebuilt; 20 regular tests, the encrypted fake-audio
daemon test (authorization, frames and grids), clippy, and APK signature / 16 KB
alignment verification pass. Physical-phone validation against the deployed rig
is pending. Session format 20 remains daemon-owned.

Viewer adaptation moved SPL/sweep pane selection into local view state because
the desktop's shared axis state no longer owns those modes, and supplies
`LeqView.chrome`. Shared trace types, resolution metadata and rendering are reused
from the pin. Neither ac2 checkout was modified by the remote agent.

The original request and ac2 developer answer below are retained as handoff history;
the protocol-33 APK and checkout statements describe the state before this refresh.

## Request: identify the dependency sources matching the deployed daemon

The Android viewer needs updating from protocol 33 to protocol 35. Please confirm
the ac2 revision the viewer should target and the checkout path to use for its
shared Rust crates and UI plot adapter. If the intended source is `~/src/ac2`,
any update of that checkout belongs to the ac2 development process.

No ac2 code change or daemon deployment is requested by this handoff. The known
protocol and session-format changes have already landed upstream. The remote
agent must not modify, fetch into, or otherwise update `~/src/ac2`.

## Evidence checked on 2026-10-10

- `~/src/ac2` working checkout: `5e267e1`, protocol **33**, session format **18**.
- Cached `origin/main`: `6137b93`, protocol **35**, session format **20**.
  Its `STATUS.md` records deployment of `684d610` to pupu, ketunkolo and the Pi.
  This is the repository's deployment record, not a live check of those daemons.
- Versions were read from `crates/ac2-proto/src/lib.rs::PROTO_VERSION` and
  `crates/ac2-traces/src/session.rs::VERSION`. No fetch was performed.
- The latest APK built by the remote agent uses protocol **33** and cannot
  connect to a protocol-35 daemon.

`ac2-remote/Cargo.toml` currently uses sibling path dependencies under
`../ac2/crates/`; `src/lib.rs` also includes `ac2-ui/src/plot.rs` from that sibling.
Changing the dependency location therefore requires updating both references in
ac2-remote, rather than just selecting a different Cargo dependency revision.

## Work owned by ac2-remote after the dependency target is established

Adapt the viewer to the shared types and scene APIs from the agreed revision:

- Stored transfer IR: `TraceData.ir`, `TransferIr`, and `SweepIr` renamed to
  `TraceIr`; check trace fixtures and stored sweep/IR rendering.
- Transfer/sweep resolution settings and unresolved-resolution metadata: check
  measurement fixtures and use the shared rendering behavior.
- Plot chrome API changes: check scene calls and view literals, including the new
  `LeqView.chrome` field. Compile against the actual revision to find the complete
  set of required changes; this list is based on source inspection, not a build.

Then run viewer tests and clippy, rebuild and verify the APK (signature and 16 KB
alignment), update the documented compatible revision/protocol, and confirm a
connection to a matching daemon. Use the shared protocol constant; changing a
version number alone is insufficient.

The viewer does not read or write session directories. Session format 20 is
handled by ac2d; no viewer-side session migration or format bump is required.

The handoff is answered when the target revision and usable source path are
confirmed. APK compatibility remains a separate ac2-remote validation step.

## Answer from ac2 (2026-10-10)

- **Target revision: `684d610`** ("Resolution per TF/sweep measurement and a
  valid-resolution marker"). This is the daemon build deployed on pupu, ketunkolo
  and the Pi. Verified at that commit: `PROTO_VERSION = 35`, session `VERSION = 20`.
  `origin/main` (`6137b93`) adds only `STATUS.md` / `docs/rigs/pupu.md` on top of it.
- **Source path: `/work/ac2-pin/684d610`**, a detached git worktree of the ac2 repo
  pinned at `684d610`. Point the seven `path = "../ac2/crates/..."` entries in
  `Cargo.toml` and the `#[path = "../../ac2/crates/ac2-ui/src/plot.rs"]` in
  `src/lib.rs` at it. Treat it as read-only; for a later target, ac2 creates a new
  pinned worktree and you change the path.
- `~/src/ac2` has been fast-forwarded to `origin/main` (`6137b93`), so the existing
  sibling paths also see protocol 35 today. Don't rely on them: other agents work
  in that checkout and it will move. The pinned worktree is the contract.
- A git dependency with `rev = "684d610"` isn't enough on its own while
  `src/lib.rs` includes `plot.rs` from the filesystem.
