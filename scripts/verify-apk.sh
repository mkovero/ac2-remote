#!/usr/bin/env bash
set -euo pipefail
source "$(dirname "$0")/android-env.sh"
cd "$ac2_remote_root"
ac2_apk="${1:-target/debug/apk/ac2-remote.apk}"
ac2_build_tools="$ANDROID_HOME/build-tools/35.0.0"
"$ac2_build_tools/apksigner" verify "$ac2_apk"
"$ac2_build_tools/zipalign" -c -P 16 4 "$ac2_apk"
python3 - "$ac2_apk" "$ANDROID_NDK_ROOT" <<'PY'
import pathlib
import re
import subprocess
import sys
import tempfile
import zipfile

apk, ndk = sys.argv[1:]
readelf = pathlib.Path(ndk) / "toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-readelf"
with tempfile.TemporaryDirectory(prefix="ac2-remote-apk-") as directory:
    with zipfile.ZipFile(apk) as archive:
        libraries = [name for name in archive.namelist() if name.endswith(".so")]
        assert libraries, "APK contains no native libraries"
        for name in libraries:
            path = pathlib.Path(directory) / pathlib.Path(name).name
            path.write_bytes(archive.read(name))
            headers = subprocess.check_output([readelf, "-l", path], text=True)
            aligns = [int(line.split()[-1], 16) for line in headers.splitlines() if line.strip().startswith("LOAD ")]
            assert aligns and min(aligns) >= 16384, f"{name}: not 16 KB aligned: {aligns}"
            dynamic = subprocess.check_output([readelf, "-d", path], text=True)
            needed = re.findall(r"Shared library: \[(.*?)\]", dynamic)
            forbidden = [lib for lib in needed if "zmq" in lib or "sodium" in lib]
            assert not forbidden, f"transport wasn't linked statically: {forbidden}"
            print(f"{name}: 16 KB ELF alignment; dependencies: {', '.join(needed)}")
print(f"Verified {apk}")
PY
