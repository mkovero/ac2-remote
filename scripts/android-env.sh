#!/usr/bin/env bash
# Source from another script so local tools and external SDK installs work alike.
ac2_remote_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
export ANDROID_HOME="${ANDROID_HOME:-$ac2_remote_root/.tools/android-sdk}"
export ANDROID_SDK_ROOT="$ANDROID_HOME"
export ANDROID_NDK_ROOT="${ANDROID_NDK_ROOT:-$ANDROID_HOME/ndk/27.2.12479018}"
export ANDROID_USER_HOME="${ANDROID_USER_HOME:-$ac2_remote_root/.tools/android-user}"
if [[ -z "${JAVA_HOME:-}" && -d "$ac2_remote_root/.tools/jdk" ]]; then
    export JAVA_HOME="$ac2_remote_root/.tools/jdk"
fi
export PATH="${JAVA_HOME:+$JAVA_HOME/bin:}$ac2_remote_root/.tools/cargo/bin:$ANDROID_HOME/platform-tools:$PATH"
