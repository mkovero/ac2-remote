# Measurement viewer prototype

This prototype shares ac2's wire protocol, client, scenes and GPU renderer. It sends
hello, state, frequency-grid and stored-trace requests and subscribes to the selected
measurement group's live streams. Transfer views include owned math and visible stored
curves; spectrum views combine live FFT/RTA and visible stored spectra/RTA. Sweep measurements
show their latest stored result. Stored trace metadata changes refresh the cache;
hidden or deleted comparison traces are removed. Sweep results remain available to the
distortion pane even when hidden as comparison curves, matching the host: prefer the newest
run of the current sweep measurement, otherwise use the newest stored sweep on the rig. Session,
measurement and stimulus setup stay in ac2's host application. It does not capture audio.

## Source and tools

Cargo dependencies and the desktop plot callback adapter use the read-only pinned
ac2 checkout at `.tools/ac2-pins/6d5a4275`, revision
`6d5a42758de7bf30b9eab3b5b09fbef859c69666` (protocol 50, daemon session format 25).
That path must exist to build this repository. Viewer-owned snapshots are now
authorized; see [the refresh procedure](upstream-refresh.md) for verifying upstream
and selecting an immutable target. Update Cargo paths and the plot adapter together.
Keep the snapshot, existing `/work/ac2-pin` checkouts and `~/src/ac2` read-only.
The viewer does not read or write session directories; ac2d handles session format.
Discovery uses TXT layout 2, which advertises the public server key for pairing.
Build and run a protocol-matching daemon on the host.
The path adapter is temporary; an upstream shared renderer crate would remove this
source-file dependency before release.

The prototype targets ARM64 Android 8.0/API 26 or later, using Rust 1.95,
eframe 0.36.2, cargo-apk 0.10.0, JDK 17, SDK platform/build tools 35 and
NDK 27.2.12479018. The APK uses Vulkan through wgpu; device support must be checked.

For a preinstalled SDK/JDK, set `ANDROID_HOME`, `ANDROID_NDK_ROOT` and `JAVA_HOME`.
For the local layout used here, put the JDK at `.tools/jdk` and SDK at `.tools/android-sdk`.
Download Android's command-line tools and unpack them into
`.tools/android-sdk/cmdline-tools/latest`. Then run:

```sh
export JAVA_HOME="$PWD/.tools/jdk"
.tools/android-sdk/cmdline-tools/latest/bin/sdkmanager --sdk_root="$PWD/.tools/android-sdk" \
  'platform-tools' 'platforms;android-35' 'build-tools;35.0.0' 'ndk;27.2.12479018'
# Review and accept the SDK license when prompted.
rustup target add aarch64-linux-android --toolchain 1.95
cargo install cargo-apk --version 0.10.0 --locked --root .tools/cargo
```

All local tools, debug signing keys, build outputs and desktop client keys are ignored by Git.
The build script sets `ANDROID_USER_HOME` locally, so debug signing does not write into
your global Android configuration. This is a test APK, signed with a development key.

## Build and install

```sh
bash scripts/build-android.sh
bash scripts/verify-apk.sh
source scripts/android-env.sh
adb devices
adb install -r target/debug/apk/ac2-remote.apk
adb shell am start -n fi.mui.ac2remote/android.app.NativeActivity
```

The script fetches the locked ARM64 dependency graph and then packages offline. It sets
16 KB native library alignment for recent Android devices. The verification script checks
signing, APK alignment, each library's ELF alignment and static transport linkage.

On the phone, scan for a rig and connect from the first screen. Android uses either
landscape orientation and hides the status bar. Pinch to zoom, drag with two fingers
to pan frequency, and double-tap to reset. Sample plots are available in the desktop
harness with `--demo`.
When connected, swipes navigate top-level measurements, including stopped measurements.
Tap or long-press the measurement plot to cycle its G views in both preview and
connected mode: transfer magnitude → phase → coherence → linear IR → IR in dB → ETC;
spectrum/RTA → split spectrograph → spectrograph;
SPL meter → Leq → both → bands (when configured); sweep response/distortion → linear IR → IR in dB → ETC → room.
The caption names the active mode. Spectrograph history builds while that mode is shown.
Long press advances once per hold, without a tap or swipe on release. Pinch and
drag gestures do not trigger it. Double-tap resets the axes without cycling the mode.
Owned math curves and stored slots are drawn inside the measurement view, not separate pages.
The only persistent UI above a measurement is its name, type, position
and running/stale state. Back returns to connection setup (Escape in the desktop harness).
The SPL number follows the shared desktop default display hold: 0.5 seconds for Fast/Impulse
and 1 second for Slow. The level bar and freshness continue to use the newest frame;
measurement weighting and acoustic averaging stay on the daemon.
The sample data is visibly marked DEMO and never
connects to a daemon.

## Connect to ac2d

1. Start a network-enabled daemon on the host:
   `ac2d --listen tcp://0.0.0.0 --name "FOH rack"`. For a no-audio development rig,
   use ac2's fake backend. Configure and start an existing transfer measurement in ac2.
2. Put phone and host on the same reachable LAN; allow TCP ports 47820 and 47821.
3. Choose the rig under **Find a rig**. The app discovers its address, control port and
   public server key through mDNS; nothing needs to be typed on the phone.
4. Compare the displayed server fingerprint with the host's **Connection** settings,
   then tick **The server fingerprint matches**. Advertisements alone never establish trust.
5. Tap **Connect**. On the host's Connection page, select the phone under **Refused keys**,
   compare its fingerprint with the phone, press **A**, enter a short name such as `phone`
   and press Enter. The host authorizes the key immediately; no daemon restart is needed.
   The phone keeps retrying until the encrypted connection succeeds.
6. Select the measurement. The viewer renders magnitude, phase
   and coherence using the same scene builder and GPU code as ac2's desktop app.

An unauthorized key behaves like an unreachable host because CURVE rejects it silently.
Check authorization, fingerprints, address and firewall when the connection keeps retrying.
Address and verified public key are saved in app-private storage; the client identity and
server pins survive app upgrades. A recognized public key remains trusted after a DHCP
address change. A host advertising a replacement key needs fresh fingerprint verification.
**Scan again** refreshes discovery; **Disconnect / change host** returns to rig selection.

Android holds a Wi-Fi multicast lock only during discovery and releases it before connecting
or leaving the app. The APK declares `CHANGE_WIFI_MULTICAST_STATE`. Wireless discovery still
depends on the access point allowing multicast; use **Manual connection** when it does not.
That section accepts an address/key and shows the client authorization line for older hosts.
A custom `host:port` specifies the control port; data uses the next port. Older ac2d adverts
(TXT layout 1) are reported as unsupported: update the daemon for automatic pairing.

Reinstall from your laptop using `adb install -r ac2-remote.apk`; this keeps app data and keys.

Frame age and STALE remain visible during data interruption. Stopped measurements are
distinguished from running measurements whose frames stopped arriving. ac2's protection
and timing banners remain present; stopped-audio banner timestamps currently use UTC.

## Desktop and automated checks

```sh
cargo run --locked -- --demo
cargo run --locked                     # discover / pair / connect
cargo test --locked --all-targets
cargo clippy --locked --all-targets -- -D warnings
cargo fmt --all --check
```

The integration tests use ac2's fake daemon over real local ZeroMQ sockets. They verify
transfer reception and grid retrieval, measurement subscription switching, stale readings,
daemon silence and cancellation while the handshake is pending. They check the daemon's
request log for session/stimulus mutations. The fake-daemon transport is unencrypted;
these tests do not establish Android or encrypted end-to-end connectivity.

An additional opt-in test starts a real network-mode ac2d with **fake audio only**,
temporary server/client keys and separate config/data directories. A setup client opens
the simulated session and starts a transfer measurement. The initially unauthorized phone
appears under Refused keys; the setup client grants it access after the first handshake
times out. The retrying viewer then receives its frame and grid over CURVE, without
restarting the daemon. Build a protocol-matching daemon and run:

```sh
AC2_BUILD_ID=0.0.0+6d5a42758de7 cargo build --locked --manifest-path .tools/ac2-pins/6d5a4275/Cargo.toml -p ac2d --target-dir target/ac2-daemon
AC2D_BIN="$PWD/target/ac2-daemon/debug/ac2d" cargo test --locked --test daemon -- --ignored --nocapture
```

For a Linux software-rendered preview (Xvfb and Python Pillow installed):

```sh
cargo build --locked --bin ac2-remote
xvfb-run -a -s '-screen 0 960x640x24' python3 scripts/capture-desktop.py
```

## Physical-device acceptance

Record device/Android version, GPU, ac2 revision and daemon protocol version.

- Install and launch, then verify the fixed plot and touch controls in both orientations.
- Pair with a real daemon; compare the same measurement's curves with the desktop.
- Keep desktop and phone connected concurrently and change the phone's measurement.
- Stop the measurement: the phone should show stopped data. Resume it: frames advance.
- Interrupt Wi-Fi: the phone must show disconnected/stale, then recover after Wi-Fi returns.
- Background/resume and lock/unlock repeatedly. Record renderer or reconnection failures.
- Check memory and battery use before calling the viewer ready for field use.

An APK build is not evidence of successful rendering or connectivity on a physical phone.
The milestone remains open until these device checks pass.

## Validation recorded 2026-10-09

| Check | Result |
| --- | --- |
| ARM64 debug APK, API 26 minimum / API 35 target | Built and signed; approximately 79 MiB |
| APK alignment and both native libraries' ELF alignment | 16 KB checks pass |
| Native entry point and Internet permission | Present in packaged artifact |
| ZeroMQ/libsodium dependency | Linked statically; packaged C++ runtime present |
| Desktop sample plot under Xvfb/Mesa | All three plot sections render; `target/prototype.png` |
| Viewer integration tests | Frame/grid retrieval, switching, silence, cancellation pass |
| Real daemon with fake audio | CURVE authorization/retry, live transfer frame and grid pass (protocol 35) |
| No-key-entry authorization | Refused phone key approved live; retry receives frames without daemon restart |
| mDNS discovery | Public key resolves over loopback UDP; malformed/versioned adverts rejected |
| Saved pairing | Restart, DHCP change, mismatched fingerprints and replacement keys tested |
| Clippy, formatting and diff whitespace | Pass |
| Physical Android device | User confirmed protocol 33 discovery, connection, transfer, pinch and swipe; protocol 35 awaits device validation |

The 2026-10-10 encrypted test used a protocol-35 daemon built from the read-only
`684d610` pin into `target/ac2-daemon`, with fake audio and temporary state only.
All 20 regular tests and this opt-in daemon test passed then. Protocol-35 physical-phone
validation was pending; the earlier physical-device checks used protocol 33.
Neither ac2 source checkout is changed by the viewer build.


## Refresh recorded 2026-10-13

The viewer now uses the read-only pin `/work/ac2-pin/2c07efef`, full revision
`2c07efeff7703d3cd46bd127f7d2808e50a91cfd`, protocol 43, daemon session format 25.
The ac2 handoff and deployment notes identify this as the daemon target on all
three rigs; this refresh did not query or deploy those rigs.

The shared decoder handles compressed/quantized frames and BFP16 impulse
responses. Shared scenes render multi-resolution spectra and carry tone/density
units into stored spectra and spectrograph history. Sweep navigation includes
linear IR, IR in dB and ETC. SPL-history long press remains pending.

- 21 regular tests pass, including a new live/stored/spectrograph density-unit check.
- The real protocol-43 daemon with fake audio passes CURVE authorization/retry,
  transfer-frame reception and grid retrieval with temporary keys and state.
- Clippy with warnings denied, formatting and diff whitespace checks pass.
- Desktop density-spectrum preview renders with per-hertz labels and
  multi-resolution window markers: `target/protocol-43-spectrum.png`.
- ARM64 debug APK built; signature, ZIP/ELF 16 KB alignment and static transport
  linkage checks pass. Artifact: `target/debug/apk/ac2-remote.apk` (83,820,982 bytes).
- APK SHA-256: `da57487af3874fb666c4384275216cf3bde3dc49b1c4ded2c4cdf180c56a0a10`.
- Physical-phone rendering and connectivity against protocol 43 remain pending.

Both ac2 checkouts were kept read-only. Session persistence remains daemon-owned.


## Protocol-49 refresh recorded 2026-10-13 (previous build)

That APK used the viewer-owned read-only source snapshot
`.tools/ac2-pins/0e5aff3f`, revision
`0e5aff3f04169b9a54c7dd10902b36e3270b1861` (protocol 49, session format 25).
The user reports protocol 49 on the rig. Upstream was checked directly through
GitHub at the selected commit; the older supplied pin and deployment notes had
lagged. The rig's exact deployed commit remains unverified.

The user authorized future viewer-owned snapshots; see AGENTS.md and the refresh
procedure. Snapshot provenance is in `.ac2-source.json` inside the snapshot and
ac2dev.md. All 1,113 archived files were checked against the source archive;
snapshot files are read-only. All seven Cargo paths and the plot adapter use this
same source. Neither `~/src/ac2` nor the existing `/work/ac2-pin` checkouts changed.

- All 21 regular tests and clippy with warnings denied pass.
- The matching fake-audio daemon passes encrypted authorization/retry and frame/
  grid reception. Its explicit build identity is `0.0.0+0e5aff3f0416`.
- ARM64 debug APK build, signature, ZIP/ELF 16 KB alignment and static transport
  linkage checks pass. Formatting and diff whitespace checks pass.
- Artifact: `target/debug/apk/ac2-remote.apk`, 84,332,982 bytes.
- SHA-256: `16bfb7df4c34f4d4ab5b2d5c3d10d75f0c48a7cd7d0db94e02817af310b4d92b`.
- Phone installation and live-rig validation remain pending. No ADB operations
  were performed for this rebuild; provide installation commands to the user.
- SPL-history long press remains pending.

## Protocol-50 refresh recorded 2026-10-13 (before long-press change)

That APK used `.tools/ac2-pins/6d5a4275`, full revision
`6d5a42758de7bf30b9eab3b5b09fbef859c69666` (protocol 50, session format 25).
GitHub-verified upstream main at `024e7757a05b8072e052e4811cbf3905632376a3`
records this source commit as deployed on all three rigs. This is deployment-log
evidence rather than a live query; the user also reports protocol 50.

The immutable viewer-owned snapshot records its revision, tree and archive hash
in `.ac2-source.json`; provenance is also recorded in ac2dev.md. All seven Cargo
paths and the plot adapter use this source. Neither `~/src/ac2` nor existing pins
were modified. Protocol 50 adds daemon health to keepalives; the shared decoder
and client handle the new fields.

- All 21 regular tests and clippy with warnings denied pass.
- The matching fake-audio daemon passes encrypted authorization/retry, frame,
  grid and daemon-health reception. Its build identity is `0.0.0+6d5a42758de7`.
- ARM64 debug APK build, signature, ZIP/ELF 16 KB alignment and static transport
  linkage checks pass. Formatting and diff whitespace checks pass.
- Artifact: `target/debug/apk/ac2-remote.apk`, 84,464,054 bytes.
- SHA-256: `7f9ee5b70fdb727a935767aedb54766a70385d36b53efac51a6be4f5919fcd0d`.
- Phone installation and live-rig validation remain pending. No ADB operations
  were performed for this rebuild; provide installation commands to the user.
- SPL-history long press remains pending.

## Initial long-press G recorded 2026-10-13 (superseded)

Long-pressing the connected plot invokes the existing view-cycle action once per
hold, alongside tap and keyboard G. It consumes pending tap/swipe input; pinch
and drag do not invoke it. The dependency remains `6d5a4275`, protocol 50,
session format 25. Upstream main `aa2a64c3` only adds documentation.

- All 23 regular tests pass, including two egui touch/pointer regression tests
  for hold/release, once-per-hold behavior, swipe consumption and pinch/drag.
- Clippy with warnings denied, formatting, diff whitespace checks and the
  encrypted fake-audio daemon test pass.
- APK signature, ZIP/ELF 16 KB alignment and static transport linkage pass.
- Artifact: `target/debug/apk/ac2-remote.apk`, 84,464,054 bytes.
- SHA-256: `c841d114cfb8312d4be576ada6dd9847acc15cd2daa4faebdc66290cc2bb17c7`.
- No ADB operations were performed; physical-phone gesture validation is pending.
- SPL-history browsing and shared desktop live-curve visibility remain pending.

## G cycling across views corrected 2026-10-13 (phone confirmed)

The initial gesture tests only checked recognition. Transfer had no G action,
and preview ignored mode changes. Transfer now cycles magnitude, phase,
coherence, linear IR, log IR and ETC; the viewer subscribes to its IR stream.
Preview and connected plots now use the same input action and scene builder.
The plot has a stable interaction ID, so status widgets changing above it cannot
interrupt a hold. All modes remain selected when axes are reset.

- All 24 regular tests pass. The touch regression walks every mode of transfer,
  RTA, spectrum, SPL and sweep, checking visible scene changes, repeated holds,
  release and status changes during a hold. The worker test also verifies IR
  reception over its added subscription.
- Clippy with warnings denied, formatting, diff whitespace checks and the
  encrypted fake-audio daemon test pass.
- Protocol 50, session format 25 and the `6d5a4275` dependency remain unchanged;
  upstream main was rechecked at `aa2a64c3`.
- APK signature, ZIP/ELF 16 KB alignment and static transport linkage pass.
- Artifact: `target/debug/apk/ac2-remote.apk`, 84,525,494 bytes.
- SHA-256: `a01c8abc075d0a26d6893a487683d83739789ca691a846a4256a4bbde9d6a99f`.
- No ADB operations were performed. Physical-phone validation remains pending.
- SPL-history browsing and shared desktop live-curve visibility remain pending.
- Xvfb GPU previews confirm G changes all five measurement types; captures are
  `target/g-cycle-{0..4}-{before,after}.png`.

## Connection screen simplified 2026-10-13 (latest APK)

The user confirmed the corrected view cycling looks good on the phone, then
requested removal of the first screen's Preview sample plot button. That screen
now provides rig scanning and connection setup. Desktop `--demo` remains
available for development.

The user validated this APK on the phone on 2026-10-13, confirming the simplified
connection screen after the earlier confirmation of corrected view cycling.

- Android build, signature, ZIP/ELF 16 KB alignment, static transport linkage,
  formatting and diff whitespace checks pass.
- Artifact: `target/debug/apk/ac2-remote.apk`, 84,525,494 bytes.
- SHA-256: `faa4e93607563d9909bf69ad0650d51ee5d0963a18729a3b05493ff264df6eff`.
- Dependency remains `6d5a4275`, protocol 50/session 25. Upstream checked at
  `89a06b4a`; subsequent changes do not alter the protocol or viewer scene APIs.
- No ADB operations were performed.
