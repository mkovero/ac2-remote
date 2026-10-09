# Repository boundaries

Treat `/home/mui/src/ac2` (`~/src/ac2`) as read-only unless the user and agent
explicitly discuss and agree on changes there. Working on `ac2-remote`, a general
instruction to proceed, or a technical dependency on `ac2` does not authorize
modifying that checkout.

Before proposing changes to `ac2`, inspect its branch, revision, working-tree
state, and relationship to main, then discuss the intended work with the user.
Do not edit files, commit, switch branches, rebase, pull, or otherwise update
`ac2` without that dialogue and agreement. Read-only inspection is allowed.

This boundary was explicitly requested after an agent changed and committed
discovery code in an `ac2` checkout that was 92 commits behind main. The user has
their own rebase and deployment process; do not interfere with it.
