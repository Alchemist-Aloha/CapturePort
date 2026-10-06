#!/usr/bin/env python3
"""Offline installer checks; never touch the real home or GitHub."""
import hashlib
import os
from pathlib import Path
import subprocess
import sys
import tempfile

INSTALLER = Path(__file__).with_name("install.sh").resolve()
BINARY = b"#!/bin/sh\necho captureport-fixture\n"

with tempfile.TemporaryDirectory(prefix="captureport-install-test-") as directory:
    root = Path(directory)
    tools = root / "tools"
    tools.mkdir()
    curl = tools / "curl"
    curl.write_text(f"#!{sys.executable}\n" + '''
import hashlib
import os
from pathlib import Path
import sys
args = sys.argv[1:]
url = next(arg for arg in args if arg.startswith("https://"))
output = Path(args[args.index("-o") + 1])
base = "https://github.com/Alchemist-Aloha/CapturePort/releases/"
mode = os.environ["TEST_MODE"]
binary = b"#!/bin/sh\\necho captureport-fixture\\n"
if url == base + "latest":
    print(base + "tag/v1.2.3", end="")
elif url == base + "download/v1.2.3/SHA256SUMS":
    checksum = hashlib.sha256(binary).hexdigest()
    name = "other-file" if mode == "missing" else "captureport-linux-x86_64"
    output.write_text(f"{checksum}  {name}\\n")
elif url == base + "download/v1.2.3/captureport-linux-x86_64":
    if mode == "download-failure":
        sys.exit(22)
    output.write_bytes(b"corrupt" if mode == "corrupt" else binary)
else:
    raise AssertionError(url)
''')
    curl.chmod(0o755)
    uname = tools / "uname"
    uname.write_text('#!/bin/sh\ncase "$1" in\n-s) echo Linux;;\n-m) echo "${TEST_ARCH:-x86_64}";;\nesac\n')
    uname.chmod(0o755)

    for mode in ("success", "corrupt", "missing", "download-failure", "unsupported"):
        home = root / mode
        bin_dir = home / ".local/bin"
        bin_dir.mkdir(parents=True)
        installed = bin_dir / "captureport"
        installed.write_bytes(b"previous installation")
        env = dict(os.environ, HOME=str(home), TEST_MODE=mode,
                   PATH=f"{tools}:{os.environ['PATH']}",
                   TEST_ARCH="aarch64" if mode == "unsupported" else "x86_64")
        result = subprocess.run(["sh", str(INSTALLER)], env=env,
                                capture_output=True, text=True)
        if mode == "success":
            assert result.returncode == 0, result.stderr
            assert installed.read_bytes() == BINARY
            assert installed.stat().st_mode & 0o777 == 0o755
            assert 'export PATH="$HOME/.local/bin:$PATH"' in result.stdout
            # Updating an already installed executable succeeds too.
            result = subprocess.run(["sh", str(INSTALLER)], env=env,
                                    capture_output=True, text=True)
            assert result.returncode == 0, result.stderr
            assert installed.read_bytes() == BINARY
        else:
            assert result.returncode != 0, mode
            assert installed.read_bytes() == b"previous installation", mode
        assert not list(bin_dir.glob(".captureport.*")), mode

print("Installer checks passed (install, update, checksum, download, architecture).")
