# Keeping the viewer current with ac2

Run this procedure at the start of viewer implementation, debugging, or dependency
maintenance, and after a protocol mismatch is reported. It runs during repository
work, not as a scheduled background service. Status/design questions do not
require a rebuild. On 2026-10-13 the user authorized viewer-owned immutable source
snapshots, removing the requirement to wait for an ac2 developer to supply a pin.

## Establish the actual target

1. Read AGENTS.md, the latest notes in ac2dev.md and working-tree status. Preserve
   existing user changes. Read the rig protocol from the user's report or available
   read-only connection/discovery information. Do not install or deploy anything
   merely to determine a version.
2. Check actual upstream main through the GitHub API, then read protocol/session
   constants at the returned full commit. For example:

   ```sh
   gh api repos/mkovero/ac2/commits/main --jq .sha
   gh api 'repos/mkovero/ac2/contents/crates/ac2-proto/src/lib.rs?ref=FULL_COMMIT' --jq '.content | @base64d'
   ```

   Local HEAD, cached refs, deployment notes and a previous handoff may all lag.
   Inspect `~/src/ac2` read-only for useful source/diffs, but never fetch, switch,
   update, or edit that checkout. A cached object matching the verified full
   upstream commit can be used as the read-only source for `git archive`.
3. Select an immutable revision compatible with the rig's reported protocol.
   If upstream main has moved to another protocol, identify the appropriate
   revision rather than blindly selecting main. A matching protocol does not
   establish the rig's exact build identity; record what is actually known.
   If the rig target is unknown, report that limitation and resolve it before
   claiming compatibility. Keep strict protocol checks.
4. Review protocol, client, discovery, transport, scene, plot and included UI
   adapter changes and their dependencies. Session persistence is daemon-owned;
   a session-format change alone does not require a viewer refresh.

## Prepare a reproducible source snapshot

- Create each snapshot under `.tools/ac2-pins/<commit-prefix>` in ac2-remote.
  Never edit or overwrite an existing snapshot. All ac2 Cargo paths, including
  dev dependencies, and the plot adapter must use the same snapshot.
- Obtain the exact verified commit through `git archive` from an existing local
  object, or a separate viewer-owned clone/cache under `.tools`. That cache may
  be fetched; neither `~/src/ac2` nor `/work/ac2-pin` may be changed. Network
  operations still follow sandbox escalation requirements.
- Record repository URL, full commit, source tree, archive SHA-256, protocol and
  session format in `.ac2-source.json` inside the snapshot. Extract only beneath
  the intended destination and make the extracted snapshot read-only. Preserve
  executable bits. Record the current full target and provenance in ac2dev.md.
- Relative Cargo paths start at this repository; the `#[path]` in src/lib.rs
  starts at src, so its snapshot path begins `../.tools/ac2-pins/…`.
- If the source or required upstream changes cannot be obtained, leave one
  concrete handoff in ac2dev.md and continue independent viewer work. Creating
  a viewer-owned snapshot no longer requires routine user confirmation.

## Adapt and validate

1. Update Cargo paths, the plot adapter and the current AGENTS.md target together.
   Adapt viewer code and fixtures to the actual APIs. Never alter a version
   constant to bypass checks. Resolve the lockfile only if necessary, then use
   locked builds.
2. Run formatting, `cargo test --locked --all-targets` and
   `cargo clippy --locked --all-targets -- -D warnings`. Local socket tests may
   require sandbox escalation. Verify units and measurement semantics where
   shared APIs changed, not just compilation.
3. Build a matching fake-audio daemon into this repository's target directory:

   ```sh
   AC2_BUILD_ID=0.0.0+FULL_COMMIT cargo build --locked --manifest-path .tools/ac2-pins/COMMIT_PREFIX/Cargo.toml -p ac2d --target-dir target/ac2-daemon
   AC2D_BIN="$PWD/target/ac2-daemon/debug/ac2d" cargo test --locked --test daemon -- --ignored --nocapture
   ```

   Set AC2_BUILD_ID explicitly: a source archive nested in ac2-remote would
   otherwise discover the viewer's enclosing Git repository and report its SHA.
   The test uses fake audio, temporary keys and temporary session/config data;
   do not operate or deploy a production daemon for dependency maintenance.
4. Run `bash scripts/build-android.sh` and `bash scripts/verify-apk.sh`. Verify
   that packaging succeeded at the expected path and record its SHA-256.
5. Update current compatibility/build records in README.md, docs/prototype.md,
   AGENTS.md and ac2dev.md. Preserve historical handoffs; distinguish them from
   the current target. Record actual checks and APK location. Physical-device
   validation stays pending until performed; a fake-audio test does not prove
   connectivity to a differently built deployed daemon.
6. Review `git diff --check` and the final changes. Report the target, checks and
   remaining limitations. Provide installation commands when asked; do not
   connect to or install on the phone unless the user requests that action.
