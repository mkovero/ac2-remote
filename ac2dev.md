# Handoff to ac2 developers

## Latest APK: connection screen without preview (2026-10-13)

The user confirmed that the corrected view cycling looks good on the phone.
Removed the Preview sample plot button from the first screen as requested;
it now contains rig scanning and connection setup. The desktop `--demo` harness
remains available for development. No measurement or gesture behavior changed.

Android build, signature, ZIP/ELF 16 KB alignment, static transport linkage,
formatting and diff whitespace checks pass. No new tests were needed for the
button removal. Latest artifact: `target/debug/apk/ac2-remote.apk`, 84,525,494
bytes; SHA-256:
`faa4e93607563d9909bf69ad0650d51ee5d0963a18729a3b05493ff264df6eff`.
The user validated this build on the phone, including the simplified connection
screen, on 2026-10-13.
No ADB operations were performed.

Upstream main was verified at `89a06b4a74d3e1b0e314985674882799d1772e75`.
New changes cover daemon validation/authentication, WAV imports and tests;
the protocol and viewer scene APIs are unchanged. The viewer keeps its known
deployed target `6d5a4275` (protocol 50/session 25) for this UI-only change.
The shared-live-visibility handoff and SPL-history browsing remain pending.

## G cycling across views corrected (2026-10-13, phone confirmed)

The user reported that long press only worked on SPL. The original tests checked
gesture recognition without verifying each view. Transfer lacked a G cycle;
preview ignored mode selection. Transfer now cycles magnitude, phase, coherence,
linear IR, log IR and ETC and subscribes to its IR stream. Preview and connected
plots share the same input action and scene builder. A stable plot interaction ID
also keeps changing status widgets from interrupting a hold.

All 24 regular tests pass, including touch-driven scene changes and complete
mode cycles for transfer, RTA, spectrum, SPL and sweep, plus IR subscription
reception. The encrypted fake-audio daemon test, clippy with warnings denied,
formatting and APK verification pass. Dependency `6d5a4275`, protocol 50/session
25 remain unchanged; upstream main rechecked at `aa2a64c3` only adds docs.
Latest artifact: `target/debug/apk/ac2-remote.apk`, 84,525,494 bytes; SHA-256:
`a01c8abc075d0a26d6893a487683d83739789ca691a846a4256a4bbde9d6a99f`.
No ADB operations were performed. The user subsequently confirmed the correction
looks good on the phone.
The shared-live-visibility handoff and SPL-history browsing remain pending.
An isolated Xvfb GPU preview check also confirmed that G changes all five
rendered measurement types. Captures: `target/g-cycle-{0..4}-{before,after}.png`.

## Initial long-press APK (2026-10-13, superseded)

Holding the connected measurement plot now advances its view once, through the
same action as a tap or G. The hold clears any pending tap and consumes the
swipe; release does not advance again or navigate. Pinch and drag suppress the
hold action. Two tests drive actual egui touch/pointer events to cover these
interactions. This cycles existing views; the separate SPL-history browsing
request remains pending.

All 23 regular tests, the encrypted fake-audio daemon test (frames, grids and
health), clippy with warnings denied, formatting and APK verification pass.
Protocol 50/session 25 and dependency revision `6d5a4275` are unchanged; upstream
main checked at `aa2a64c3f478dfe0624d98f8846316f903e112a9` only adds docs.
Latest artifact: `target/debug/apk/ac2-remote.apk`, 84,464,054 bytes; SHA-256:
`c841d114cfb8312d4be576ada6dd9847acc15cd2daa4faebdc66290cc2bb17c7`.
No ADB operations were performed. Physical-phone gesture validation remains
pending. The live-visibility handoff below remains open.

## Open handoff: spectrum live-curve visibility (2026-10-13)

The user reports more spectrum curves on the phone than on the desktop.
Protocol-50 source inspection found a visibility gap: desktop
`ac2-ui/src/scenes/spectrum_pane.rs::with_spectrum` skips `st.meas_hidden(m)`,
but `AppState.hidden_meas` is a local set of measurement names, saved in
`ui.toml` through `state_queries_layout.rs`. Toggling a live curve in
`state_display.rs::toggle_meas_hidden` sends no daemon command. Neither
`model::Measurement` nor `model::PaneLayout` exposes this visibility to another
client. The viewer therefore receives every spectrum/RTA/math measurement
without knowing which live curves the desktop has hidden. Running/stopped is
independent of hidden/shown; it must not be used as a substitute.

Requested upstream outcome: make the intended live-curve visibility available
through the mirrored daemon state, with a documented policy for desktop and
remote clients to follow. Prefer measurement IDs over display names; include
math channels and keep visibility separate from whether a job runs. Desktop
hide/show actions must publish the shared choice. Document session persistence,
defaults and how existing local hidden preferences are handled. Supply the
resulting immutable revision/protocol and tests for hide/show propagation,
rename, deletion and reconnect. No ac2 source was changed here.

Once available, the viewer should apply that state consistently when subscribing,
building curves, choosing the level scale, fitting axes and showing spectrograph
history. Until then it cannot automatically reproduce desktop-local live
visibility. No speculative viewer filter or APK rebuild was made for this
investigation.

Stored traces already use the shared `TraceEdit.visible` flag. The existing
integration test for hide/show/rename/delete and the spectrum scene test for
excluding hidden stored captures both pass. Confirmation of whether the user's
extra curves are live or stored was requested. Upstream main was checked directly
at `aa2a64c3f478dfe0624d98f8846316f903e112a9`, still protocol 50; changes since
the current `6d5a4275` pin only affect documentation.

## Protocol-50 dependency refresh (2026-10-13, before long-press change)

The rig now reports protocol 50. Upstream main was verified through GitHub at
`024e7757a05b8072e052e4811cbf3905632376a3`. Its deployment log identifies
`6d5a42758de7bf30b9eab3b5b09fbef859c69666` on all three rigs (protocol 50,
session format 25); that exact source commit was separately verified through
GitHub and selected for the viewer. The log is deployment evidence, not a live
query of the rigs.

Viewer-owned snapshot: `.tools/ac2-pins/6d5a4275`. Source tree:
`6f9ceb30bee3f30a4bf970e0481dfa12be1794c3`; archive SHA-256:
`70a9267f4046f4dbbaf5c0d8f14f252c3c33d088d50d2cee3cae6da8df29981c`.
It was created by read-only `git archive` from the exact locally available commit,
with provenance recorded in `.ac2-source.json` and snapshot write bits removed.
Every ac2 Cargo dependency and the plot adapter now uses that snapshot.
Neither `~/src/ac2` nor the existing snapshots were modified.

Protocol 50 adds daemon health to keepalives and client receive-byte accounting.
The viewer reuses the updated decoder/client; its encrypted fake-audio test now
requires daemon health as well as transfer frames and grids before passing.

All 21 regular tests, the encrypted fake-audio daemon test, clippy with warnings
denied, formatting and diff whitespace checks pass. The test daemon reports
build `0.0.0+6d5a42758de7`. The ARM64 APK passes signature, ZIP/ELF 16 KB
alignment and static transport linkage checks. Artifact:
`target/debug/apk/ac2-remote.apk`, 84,464,054 bytes; SHA-256:
`7f9ee5b70fdb727a935767aedb54766a70385d36b53efac51a6be4f5919fcd0d`.

Phone installation and live-rig validation remain pending. No ADB operations
were performed; provide installation commands only. The separate SPL-history
long-press request remains pending. No outstanding ac2 developer action for
this refresh.

## Previous build: protocol 49 (2026-10-13)

The user reports that the rig runs protocol 49. The earlier protocol-43 APK
could not connect to that rig; it has now been replaced by the protocol-49 build.
No outstanding ac2 developer action for this refresh.

Read-only upstream inspection identified main at
`0e5aff3f04169b9a54c7dd10902b36e3270b1861`, protocol 49, session format 25.
The protocol constant was verified through the GitHub contents API at that exact
commit. Local cached main also contains that commit; working-checkout state and
deployment notes must not substitute for checking upstream and the rig's reported
protocol. The rig's exact deployed commit has not been established.

The user approved viewer-owned immutable snapshots on 2026-10-13. The
protocol-49 target has been extracted into `.tools/ac2-pins/0e5aff3f` from a
read-only `git archive` of the GitHub-verified commit. Source tree:
`1d4990ee4bfa83fef6de379dabb0411db6aa4c87`; archive SHA-256:
`8ac3f810d54499102a726192c2b3df2f68da1cc7ad5ea2806f2ed8e324f2c247`.
The snapshot records these in `.ac2-source.json` and is read-only. AGENTS.md and
the refresh procedure now permit future viewer-owned snapshots without routine
confirmation; neither `~/src/ac2` nor existing `/work/ac2-pin` checkouts may be
changed. The previous request for a supplied pin is resolved by this approval.

All shared-crate paths and the plot adapter target the new snapshot. All 21
regular tests, the protocol-49 encrypted fake-audio daemon test (authorization,
frames and grids), clippy with warnings denied and formatting pass. The ARM64
APK passes signature, ZIP/ELF 16 KB alignment and static transport linkage checks.
Artifact: `target/debug/apk/ac2-remote.apk`, 84,332,982 bytes; SHA-256:
`16bfb7df4c34f4d4ab5b2d5c3d10d75f0c48a7cd7d0db94e02817af310b4d92b`.
The test daemon reports build `0.0.0+0e5aff3f0416` (set explicitly so an archived
source tree does not pick up the enclosing viewer repository's Git revision).

Phone installation and live-rig validation remain pending. The user requested
installation commands only; no ADB connection, installation or launch was made
for this rebuild. The separate SPL-history long-press request remains pending.

## Previous build: protocol 43 (2026-10-13)

No outstanding ac2 developer action for this refresh. The viewer now uses
`/work/ac2-pin/2c07efef`, full revision
`2c07efeff7703d3cd46bd127f7d2808e50a91cfd`, for all shared crates and the plot
adapter. Protocol 43, daemon session format 25. The intermediate pins below were
skipped in favor of the latest supplied target.

The ARM64 debug APK at `target/debug/apk/ac2-remote.apk` has been rebuilt and
verified. All 21 regular tests, the encrypted fake-audio daemon test
(authorization/retry, frames and grids), clippy, formatting, APK signature and
ZIP/ELF 16 KB alignment checks pass. Physical-phone validation against the
announced deployed rig remains pending; no production daemon was queried or
changed. Session persistence remains daemon-owned.

Viewer adaptations preserve tone/density units for live and stored spectra and
spectrograph history, use the shared packed-frame decoder, and follow the sweep
view chain through linear IR, IR in dB and ETC. The previously requested
SPL-history long press remains pending. Neither ac2 checkout was modified.

The requests, answers and deployment statements below are historical handoff notes.

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
- Correction (later on 2026-10-10): pupu's ac2d now runs `d08cc705`, which is
  `bdc1bec` plus an ac2-core harmonic-floor change (not used by the viewer).
  `bdc1bec` is on
  main 9 commits after `684d610`. It still uses protocol 35, so a viewer built on
  `684d610` connects. The only shared-crate changes between the two are in
  `ac2-scene` (`view.rs` and `ir.rs`). The pin stays at `684d610` unless ac2
  announces a new one.

## Viewer dependency refresh procedure (2026-10-10)

Clarification from the user: ac2 consistently bumps version numbers. The request
is for the remote agent to keep up automatically through a repeatable dependency
refresh procedure, rather than redesigning compatibility or weakening checks.

The standing procedure is in [docs/upstream-refresh.md](docs/upstream-refresh.md)
and is required by AGENTS.md. During viewer work, the remote agent checks the
current pin, the latest ac2 developer handoff and the local ac2 checkout read-only.
When a new target pin is supplied, it adapts the viewer, validates it and builds a
new APK without requesting routine confirmation. A missing target pin gets one
concrete handoff here; the remote agent continues independent viewer work.

Protocol numbers identify wire compatibility; session-format changes remain
owned by the daemon. Shared rendering/client changes can merit a refresh even
when the protocol stays the same. No new ac2 interface is requested by this note.
The existing source boundaries and immutable pin requirements remain in force.

## New pin from ac2 (2026-10-11)

- **Target revision: `6f0e5b57`**, source **`/work/ac2-pin/6f0e5b57`** (detached, read-only).
  Rolling out to pupu, ketunkolo and the Pi on 2026-10-11. Protocol **39**, session format **22**.
  Keep `/work/ac2-pin/684d610` until the viewer has switched.
- Wire change that affects the viewer: live frame payloads are now **packed**. Each frame's
  arrays are concatenated into one raw-deflate part (zlib wbits −15). Curves (mag, level, ir_etc)
  are i16 hundredths of a dB, phase is i16 hundredths of a degree, coherence is u16 1e-4. 16-bit
  arrays are sent as neighbour differences split into low- and high-byte planes. ir_linear,
  meters, leq and band_leq stay f32. Use `ac2-proto`'s decoder from the pin rather than parsing
  this by hand. The spec is in `docs/protocol.md` §5.2–5.7 and `docs/design/wire-slim.md`.
- The daemon now publishes at 60 fps on network transports.
- Other shared-crate changes since `684d610` include `ac2-scene` (transfer Magnitude view,
  colour shades, spectrograph slots) and `ac2-client`.

## Newer pin from ac2 (2026-10-11, later)

- **Target revision: `303d02a9`**, source **`/work/ac2-pin/303d02a9`**. All three hosts run it.
  Protocol **40**, session format **23**. It replaces `6f0e5b57`, which was deployed for only
  a few hours; skip that one.
- What changed since `6f0e5b57`:
  - A new command `meas.layout`.
  - A new field `Measurement.layout: Option<PaneLayout>`, defined in `ac2_proto::model::layout`.
  - Spectrum frames are now stamped at the end of their FFT frame.
  - `ac2-scene` has new glide pacing (`curve_ease::Pacing`) and a meas-list `live_selected` field.
- The frame payload format is the same as in `6f0e5b57`.

## Newer pin from ac2 (2026-10-11, night)

- **Target revision: `c7bd24b1`**, source **`/work/ac2-pin/c7bd24b1`**. All three hosts run it.
  Protocol **42**, session format **24**. It replaces `303d02a9`.
- What changed since `303d02a9`:
  - Spectrum frames can now carry multi-resolution bands: `SpectrumConfig.resolution` is
    `Fixed { fft_len } | Multi { grid }`, and `SpecMeta.bands` is new. Multi-resolution frames
    use a log grid.
  - `ir_linear` now goes on the wire as `bfp16`: one signed exponent byte, then i16 mantissas.
    Each value is mantissa · 2^e.
  - The client drops frames for topics it has unsubscribed from (`ac2-client` data.rs).
  - Use the pin's `ac2-proto` decoder. The spec is in `docs/protocol.md` §5 and
    `docs/design/wire-slim.md`.

## Newer pin from ac2 (2026-10-11, morning)

- **Target revision: `2c07efef`**, source **`/work/ac2-pin/2c07efef`**. All three hosts run it.
  Protocol **43**, session format **25**. It replaces `c7bd24b1`.
- What changed since `c7bd24b1`:
  - `SpectrumConfig.level: SpecLevel { Tone, Density }` is new. Fixed FFTs default to Tone;
    multi-resolution spectra default to Density.
  - Spectrum frames carry the level. Density frames use the new units `dbfs_per_hz` and
    `db_spl_per_hz`; label the axis `dBFS/Hz` or `dB SPL/Hz` (density).
  - `TraceKind::Spectrum { scale, level }`: stored traces record their level too.
  - The frame payload encoding is unchanged; use the pin's `ac2-proto` decoder. The spec is in
    `docs/protocol.md` and `docs/design/multires-spectrum.md` (Density section).
