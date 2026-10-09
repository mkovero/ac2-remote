#!/usr/bin/env python3
"""Capture the sample plot under X11 (e.g. xvfb-run -a python3 ...)."""
import argparse
import pathlib
import subprocess
import time

from PIL import ImageGrab

parser = argparse.ArgumentParser()
parser.add_argument("--output", default="target/prototype.png")
parser.add_argument("--connection", action="store_true", help="capture discovery/pairing instead of the sample plot")
args = parser.parse_args()
root = pathlib.Path(__file__).resolve().parent.parent
output = root / args.output
output.parent.mkdir(parents=True, exist_ok=True)
with (output.parent / "desktop-preview.log").open("w") as log:
    command = [str(root / "target/debug/ac2-remote")]
    if not args.connection:
        command.append("--demo")
    process = subprocess.Popen(command, cwd=root, stdout=log, stderr=log)
    try:
        time.sleep(5)
        if process.poll() is not None:
            raise RuntimeError(f"Viewer exited with {process.returncode}; see {log.name}")
        ImageGrab.grab().save(output)
        print(output)
    finally:
        process.terminate()
        try:
            process.wait(timeout=5)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()
