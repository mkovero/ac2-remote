# ac2-remote

An Android companion for [ac2](https://github.com/mkovero/ac2), bringing live measurement panes to a phone or tablet while the audio interface, DSP, and measurement session stay on the host computer.

Walk the room with a transfer function, spectrum, or SPL meter in your hand. The first version is a remote viewer with its own pane selection and layout, sharing measurements with the desktop client through `ac2d`.

## Status

ARM64 Android prototype builds successfully: NativeActivity entry point, automatic mDNS rig discovery,
fingerprint-confirmed pairing, remembered connection details, encrypted daemon connectivity,
landscape views for transfer (magnitude, phase and coherence), spectrum, RTA, SPL,
owned math curves, visible stored traces and the latest stored sweep result. Swipes move
between top-level measurements; slots and math channels stay within the measurement view.
The spectrum view combines live FFT/RTA curves and visible stored spectra/RTA traces.
Measurements fill the screen beneath
a small caption; swipe left/right to navigate, pinch to zoom, use two fingers to pan,
tap to cycle the host's G view modes, and double-tap to reset the axes. Back returns to connection setup. A desktop harness
uses the same application and renderer.

Desktop rendering, encrypted transfer reception from a real ac2d with fake audio,
integration tests and APK signature/16 KB alignment checks pass. The debug APK is
at `target/debug/apk/ac2-remote.apk`.

See [prototype setup and validation](docs/prototype.md) for build, pairing and device checks.
The user has confirmed discovery, pairing, live transfer, pinch and swipe navigation on
the phone. The landscape layout and additional measurement views await device validation.
The current viewer targets ac2 revision `684d610` (protocol 35, daemon session
format 20) via the read-only pinned checkout `/work/ac2-pin/684d610`. Automatic
pairing requires discovery TXT layout 2.

## Initial scope

- Connect to an existing remote-enabled `ac2d` on the local network.
- Use the existing encrypted connection and pinned-key pairing model.
- Show one live pane at a time: transfer function first, then spectrum/RTA and SPL.
- Select an existing measurement and use touch-friendly local plot controls.
- Clearly show connection status, stale data, and measurement faults.
- Support landscape plots and a readable portrait SPL meter.

Audio capture, DSP, session setup, calibration, stimulus control, and exact mirroring of the desktop's pane layout are outside the first milestone. Measurement results and daemon state are shared; pane layout is local to each client.

## Proposed architecture

```text
Audio interface → ac2d on host computer
                       ├── ac2 desktop UI
                       └── encrypted ZeroMQ over LAN → ac2-remote on Android
```

Start by investigating reuse of the existing Rust stack:

| Component in ac2 | Potential reuse |
| --- | --- |
| `ac2-proto` | Wire types, commands, events, measurement frames |
| `ac2-client` | Connection, subscriptions, state mirror, reconnect handling |
| `ac2-zmq` | ZeroMQ transport and CURVE encryption |
| `ac2-plot`, `ac2-scene` | Plot rendering and scene construction |
| `ac2-ui` | Existing pane rendering and application state where practical |

The current ac2 UI uses egui/eframe with wgpu. Its embedded daemon is optional. eframe exposes Android integration, but that alone does not establish that ac2's UI, rendering, native dependencies, discovery, or filesystem helpers work on Android.

Keep ac2 as the source of truth for protocol and measurement logic. During the prototype, use the documented read-only pinned ac2 checkout for path dependencies and the plot adapter. Before releases, choose a reproducible dependency strategy and pin compatible revisions: ac2 currently requires matching protocol versions rather than negotiating compatibility. If reuse needs refactoring, extract shared pane code in ac2 instead of copying large sections into this repository.

## High-level implementation steps

### 1. Prove the Android build and renderer

- Set up the Android SDK/NDK and Rust Android target; choose a minimum supported Android version.
- Create a minimal Android application entry point and APK packaging workflow.
- Run an egui/wgpu screen on a physical Android device.
- Exercise the existing plot renderer with fixed sample data, including rotation and touch input.

**Exit:** an installable APK draws an ac2-style plot on a real device. Record device, build commands, and any unsupported graphics paths.

### 2. Prove direct daemon connectivity

- Cross-compile ZeroMQ and libsodium with the NDK and integrate `ac2-client` without embedding `ac2d`.
- Adapt app-private paths for keys and preferences; audit desktop-specific assumptions in shared crates.
- Connect using a manually entered host address and the existing pairing flow.
- Verify protocol mismatch reporting and subscription to one existing transfer measurement.

**Exit:** the device receives live frames over an encrypted connection to a real daemon. This is the main feasibility checkpoint before expanding the UI.

### 3. Deliver one useful live pane

- Draw live transfer magnitude, phase, and coherence using shared plotting code where feasible.
- Add measurement selection, connection status, frame age, and clear stale/fault states.
- Add large touch controls and local zoom/pan/reset; keep a single-pane layout.
- Confirm a desktop client and the phone can observe the same daemon concurrently.

**Exit:** a usable transfer viewer for walking the room. No signal-generation or session-editing controls are required.

### 4. Make it reliable on a phone

- Handle app backgrounding, resume, screen lock, rotation, and Wi-Fi interruption.
- Reconnect and resynchronize daemon state; never present old frames as current readings.
- Preserve connection preferences and identity securely in app-private storage.
- Measure rendering cost, network traffic, and battery use; avoid drawing while hidden.
- Add an explicit keep-screen-awake option for measurement use.

**Exit:** repeated interruption and resume cycles recover cleanly on physical devices.

### 5. Expand the viewer

- Add spectrum/RTA and SPL panes, including a large portrait meter.
- Add a simple pane switcher and per-pane preferences.
- Integrate Android-compatible service discovery after manual connections work reliably.
- Consider stored trace viewing and tablet layouts based on actual use.

**Exit:** the core viewer works comfortably on both phone and tablet-sized screens.

### 6. Package and validate

- Add reproducible APK builds and CI checks for the chosen Android targets.
- Document daemon setup, pairing, networking, installation, and supported versions.
- Validate plots against the desktop using the same measurement data.
- Test with a real audio rig and imperfect Wi-Fi, then create a signed test release.

Publishing through an app store and adding remote measurement controls are later decisions. Any stimulus controls should preserve ac2's lease, arm/fire, stop, and disconnect behavior, with an explicit touch-oriented interaction design.

## First experiment

Build an APK that connects to a manually specified daemon and displays one live transfer pane. First verify Android graphics and native transport separately, then combine them. If the existing stack proves awkward, record the specific blocker before choosing a different UI shell or a host-side transport bridge.

## Planning estimate

Rough estimates for one experienced developer, subject to the Android build and transport experiments:

- Live-pane proof of concept: several days to two weeks.
- Useful personal remote viewer: approximately three to six weeks.
- Polished companion with extensive controls: a few months.

## References

- [ac2 repository](https://github.com/mkovero/ac2)
- [ac2 protocol](https://github.com/mkovero/ac2/blob/main/docs/protocol.md)
- [ac2 installation and remote setup](https://github.com/mkovero/ac2/blob/main/docs/install.md)
- [eframe 0.36.2 documentation](https://docs.rs/eframe/0.36.2/eframe/)
- [Rust Android platform support](https://doc.rust-lang.org/rustc/platform-support/android.html)
