# Requests for ac2 developers

## Protocol 35 / session format 20 dependency refresh

Inspected on 2026-10-10: the sibling checkout at `/home/mui/src/ac2` is still
`5e267e1` (protocol 33, session format 18). Its cached `origin/main` is `684d610`
(protocol 35, session format 20). These versions were read from
`crates/ac2-proto/src/lib.rs::PROTO_VERSION` and
`crates/ac2-traces/src/session.rs::VERSION`, respectively. No fetch or update of
the sibling checkout was performed.

Please provide a protocol-35 dependency checkout/revision, or update the sibling
checkout through the ac2 development process. The remote-viewer agent must leave
`~/src/ac2` untouched. Once the compatible sources are available, the viewer can
be adapted, tested, and rebuilt here. The latest rebuilt APK still uses protocol 33.

The changes include saved transfer impulse responses (`TraceData.ir`,
`TransferIr`, and `SweepIr` becoming `TraceIr`), per-measurement sweep/transfer
resolution settings, and unresolved-resolution metadata on frames/sweeps and
shared trace scenes. Check the viewer's struct literals, fixtures, and shared
scene calls when the updated dependencies are available. Use the upstream shared
types and version constant; do not merely change a local version number.

The viewer does not load or save session directories; session-format compatibility
is handled by ac2d. No local session migration is required in ac2-remote. After
refreshing the sources, run the viewer tests and clippy, rebuild the APK, verify
its signature and 16 KB alignment, and update the documented compatible revision
and protocol. Do not advertise protocol-35 APK compatibility until rebuilt.
