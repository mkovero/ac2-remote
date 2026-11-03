# Repository boundaries

Treat `/home/mui/src/ac2` (`~/src/ac2`) as read-only. Working on `ac2-remote`, a general
instruction to proceed, or a technical dependency on `ac2` does not authorize
modifying that checkout.

Do not edit files, commit, switch branches, rebase, pull, fetch, or otherwise update
`ac2`. Read-only inspection is allowed. If changes or a dependency refresh are
needed there, leave a concrete handoff note in `ac2dev.md` in this repository for
the ac2 developers. Do not request an update to that checkout as the routine path
to completing remote-viewer work. Only a later explicit user instruction changing
this boundary can authorize modifications there.

This boundary was explicitly requested after an agent changed and committed
discovery code in an `ac2` checkout that was 92 commits behind main. The user has
their own rebase and deployment process; do not interfere with it.

# Pinned dependencies

The user authorized viewer-owned immutable ac2 source snapshots on 2026-10-13.
The current dependency target is revision
`6d5a42758de7bf30b9eab3b5b09fbef859c69666` in
`.tools/ac2-pins/6d5a4275` (protocol 50, daemon session format 25).
Cargo paths and the plot adapter include in `src/lib.rs` must use the same
snapshot. Record its full revision and provenance; never edit a snapshot in place.

For later refreshes, check upstream directly and select a revision compatible
with the rig's reported protocol. Create a new snapshot under this repository's
`.tools/ac2-pins` from that verified commit. A separate viewer-owned clone/cache
may be fetched to obtain source; `~/src/ac2` may only be inspected or read as an
object source, never fetched into or modified. Existing `/work/ac2-pin` checkouts
remain read-only. Record the selected target and validation in `ac2dev.md`;
request an upstream handoff only when the target or required changes are unclear.

# Automatic viewer maintenance

At the start of viewer implementation, debugging, or dependency maintenance,
follow [docs/upstream-refresh.md](docs/upstream-refresh.md). Check upstream state
read-only and handle a verified dependency refresh as part of the
work without waiting for the user to identify version changes or request routine
adaptations. A status-only or discussion request does not require a rebuild.

This procedure does not authorize changing the upstream ac2 checkout or existing
`/work/ac2-pin` checkouts, deploying daemons, or installing an APK on a device.
