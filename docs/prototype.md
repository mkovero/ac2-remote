# Measurement viewer prototype

This prototype shares ac2's wire protocol, client, scenes and GPU renderer. It sends
hello, state, frequency-grid and stored-trace requests and subscribes to the selected
measurement group's live streams. Transfer views include owned math and visible stored
curves; spectrum views combine live FFT/RTA and visible stored spectra/RTA. Sweep measurements
show their latest visible stored result. Stored trace metadata changes refresh the cache;
hidden or deleted comparison traces are removed. Sweep results remain available to the
distortion pane even when hidden as comparison curves, matching the host: prefer the newest
run of the current sweep measurement, otherwise use the newest stored sweep on the rig. Session,
measurement and stimulus setup stay in ac2's host application. It does not capture audio.

## Source and tools

Keep checkouts at `~/src/ac2` and `~/src/ac2-remote`: Cargo uses sibling path dependencies,
including the desktop plot callback adapter. The tested ac2 revision is
`5e267e1` (protocol 33; protocol versions must match).
This revision includes TXT layout 2, which advertises the public server key for pairing.
Build and run a protocol-matching daemon on the host. Treat the ac2 checkout as read-only;
updates to it require explicit discussion with the user (see AGENTS.md).
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

On the phone, choose **Preview sample plot** first. Android uses either landscape orientation
and hides the status bar. Swipe left/right through transfer, RTA, spectrum, SPL and sweep
examples. Pinch to zoom, drag with two fingers to pan frequency, and double-tap to reset.
When connected, swipes navigate top-level measurements, including stopped measurements.
Tap the measurement to cycle its G views: spectrum → split spectrograph → spectrograph;
SPL meter → Leq → both → bands (when configured); sweep response/distortion → IR → room.
The caption names the active mode. Spectrograph history builds while that mode is shown.
Double-tap resets the axes without cycling the mode.
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
cargo build --locked --manifest-path ../ac2/Cargo.toml -p ac2d --target-dir target/ac2-daemon
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
| ARM64 debug APK, API 26 minimum / API 35 target | Built and signed; approximately 72 MB |
| APK alignment and both native libraries' ELF alignment | 16 KB checks pass |
| Native entry point and Internet permission | Present in packaged artifact |
| ZeroMQ/libsodium dependency | Linked statically; packaged C++ runtime present |
| Desktop sample plot under Xvfb/Mesa | All three plot sections render; `target/prototype.png` |
| Viewer integration tests | Frame/grid retrieval, switching, silence, cancellation pass |
| Real daemon with fake audio | CURVE handshake, live transfer frame and grid pass (protocol 25) |
| No-key-entry authorization | Refused phone key approved live; retry receives frames without daemon restart |
| mDNS discovery | Public key resolves over loopback UDP; malformed/versioned adverts rejected |
| Saved pairing | Restart, DHCP change, mismatched fingerprints and replacement keys tested |
| Clippy, formatting and diff whitespace | Pass |
| Physical Android device | User confirms discovery, protocol 33 connection, live transfer and button zoom; new gestures await device validation |

The installed older ac2d binary was rejected with protocol 14 versus client 25.
The encrypted test passed after building current source into `target/ac2-daemon`.
The sibling ac2 checkout now includes the updated discovery advert, daemon publisher,
discovery test fixtures and protocol documentation. No audio/DSP behavior changed.
