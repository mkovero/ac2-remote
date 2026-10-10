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

The ac2 developer handoff specifies `/work/ac2-pin/684d610` at revision
`684d610815f9a75841a1ce365036b8882b95bd3b` as the viewer's dependency source
(protocol 35, daemon session format 20). Treat this pinned checkout as read-only
too. Cargo paths and the plot adapter include in `src/lib.rs` must target the same
pin. For a later target, request a new pinned checkout via `ac2dev.md`; do not
switch back to the moving `~/src/ac2` checkout.
