#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/android-env.sh"
cd "$ac2_remote_root"
for tool in cargo-apk java keytool; do
    command -v "$tool" >/dev/null || { echo "Missing $tool; see docs/prototype.md" >&2; exit 1; }
done
[[ -d "$ANDROID_NDK_ROOT" ]] || { echo "Missing NDK: $ANDROID_NDK_ROOT" >&2; exit 1; }
# Older APK packagers don't set the native 16 KB alignment by default.
if [[ -n "${CARGO_ENCODED_RUSTFLAGS:-}" ]]; then
    export CARGO_ENCODED_RUSTFLAGS="${CARGO_ENCODED_RUSTFLAGS}"$'\x1f''-Clink-arg=-Wl,-z,max-page-size=16384'
else
    export RUSTFLAGS="${RUSTFLAGS:-} -C link-arg=-Wl,-z,max-page-size=16384"
fi
# cargo-apk 0.10 doesn't forward --locked. Fetch the locked graph first, then
# package offline so the APK step cannot silently resolve newer dependencies.
cargo fetch --locked --target aarch64-linux-android
CARGO_NET_OFFLINE=true cargo apk build --lib "$@"
