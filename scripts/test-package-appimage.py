#!/usr/bin/env python3
"""Exercise packaging with temporary fixtures; no network or Rust build runs."""
import hashlib
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


ROOT = Path(__file__).resolve().parent.parent
TOOL_NAME = "appimagetool-x86_64.AppImage"
TOOL_BYTES = b'''#!/usr/bin/env bash
printf '%s\n' "$@" > "$FIXTURE_EXECUTED"
: > "$3"
'''


class PackagingTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="packaging-test-")
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name) / "repo with spaces"
        for directory in ("scripts", "desktop", "assets", "target/release", "bin"):
            (self.root / directory).mkdir(parents=True)
        shutil.copy2(ROOT / "scripts/package-appimage.sh", self.root / "scripts")
        (self.root / "scripts/appimagetool.sha256").write_text(
            f"{hashlib.sha256(TOOL_BYTES).hexdigest()}  {TOOL_NAME}\n"
        )
        (self.root / "desktop/com.nodaysidle.Browser.desktop").write_text("fixture\n")
        (self.root / "assets/icon.svg").write_text("fixture\n")
        (self.root / "target/release/nodaysidle-browser").write_text("fixture\n")
        self.tool = self.root / "target/appimagetool-cache" / TOOL_NAME
        self.partial = self.tool.with_name(TOOL_NAME + ".partial")
        self.download = self.root / "download"
        self.download.write_bytes(TOOL_BYTES)
        self.executed = self.root / "executed"
        self.curl_log = self.root / "curl.log"
        self.output = self.root / "dist/nodaysidle-browser-x86_64.AppImage"
        self.env = dict(os.environ, PATH=str(self.root / "bin"),
                        FIXTURE_DOWNLOAD=str(self.download),
                        FIXTURE_EXECUTED=str(self.executed),
                        FIXTURE_CURL_LOG=str(self.curl_log), FIXTURE_CURL_MODE="ok")
        for command in ("bash", "dirname", "mkdir", "rm", "install", "ln",
                        "cat", "chmod", "awk", "sha256sum", "mv", "cp"):
            executable = shutil.which(command)
            self.assertIsNotNone(executable, command)
            (self.root / "bin" / command).symlink_to(executable)
        self.write_executable(self.root / "bin/cargo", b'''#!/usr/bin/env bash
[[ "$*" == "build --release --locked" ]]
''')
        self.write_executable(self.root / "bin/curl", b'''#!/usr/bin/env bash
set -euo pipefail
[[ "$1 $2 $3 $4" == "-L -s -f -o" ]]
[[ "$6" == "https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage" ]]
printf 'download\n' >> "$FIXTURE_CURL_LOG"
cp "$FIXTURE_DOWNLOAD" "$5"
case "$FIXTURE_CURL_MODE" in
  fail) exit 18 ;;
  term) kill -TERM "$PPID" ;;
esac
''')

    def write_executable(self, path, content):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(content)
        path.chmod(0o755)

    def run_package(self):
        return subprocess.run(
            [str(self.root / "bin/bash"), str(self.root / "scripts/package-appimage.sh")],
            cwd=self.temp.name, env=self.env, capture_output=True, text=True, timeout=10,
        )

    def assert_success(self, result):
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertEqual(self.executed.read_text().splitlines(), [
            "--appimage-extract-and-run", str(self.root / "target/AppDir"), str(self.output)
        ])
        self.assertTrue(self.output.exists())
        self.assertFalse(self.partial.exists())

    def assert_rejected(self, result):
        self.assertNotEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertFalse(self.executed.exists())
        self.assertFalse(self.output.exists())
        self.assertFalse(self.tool.exists())
        self.assertFalse(self.partial.exists())

    def test_valid_download(self):
        self.assert_success(self.run_package())
        self.assertEqual(self.tool.read_bytes(), TOOL_BYTES)
        self.assertTrue(os.access(self.tool, os.X_OK))

    def test_altered_download(self):
        self.download.write_bytes(TOOL_BYTES + b"# altered\n")
        self.assert_rejected(self.run_package())

    def test_matching_root_decoy_does_not_verify_altered_download(self):
        (self.root / TOOL_NAME).write_bytes(TOOL_BYTES)
        self.download.write_bytes(TOOL_BYTES + b"# altered\n")
        self.assert_rejected(self.run_package())

    def test_interrupted_transfer_is_removed_and_retry_verifies(self):
        self.env["FIXTURE_CURL_MODE"] = "fail"
        self.assert_rejected(self.run_package())
        self.env["FIXTURE_CURL_MODE"] = "ok"
        self.assert_success(self.run_package())

    def test_terminated_download_is_removed(self):
        self.env["FIXTURE_CURL_MODE"] = "term"
        result = self.run_package()
        self.assert_rejected(result)
        self.assertEqual(result.returncode, 143)

    def test_checksum_command_failure_rejects_download_and_cache(self):
        (self.root / "bin/sha256sum").unlink()
        self.write_executable(self.root / "bin/sha256sum", b"#!/usr/bin/env bash\nexit 1\n")
        for cached in (False, True):
            with self.subTest(cached=cached):
                if cached:
                    self.write_executable(self.tool, TOOL_BYTES)
                self.assert_rejected(self.run_package())

    def test_missing_checksum_manifest_rejects_download(self):
        (self.root / "scripts/appimagetool.sha256").unlink()
        self.assert_rejected(self.run_package())

    def test_stale_partial_is_replaced(self):
        self.write_executable(self.partial, TOOL_BYTES + b"# stale\n")
        self.assert_success(self.run_package())
        self.assertEqual(self.tool.read_bytes(), TOOL_BYTES)

    def test_valid_cache_precedes_unpinned_path_tool(self):
        self.write_executable(self.tool, TOOL_BYTES)
        self.write_executable(self.root / "bin/appimagetool", b"#!/usr/bin/env bash\nexit 99\n")
        self.assert_success(self.run_package())
        self.assertFalse(self.curl_log.exists())

    def test_invalid_cache_is_replaced_with_verified_download(self):
        self.write_executable(self.tool, TOOL_BYTES + b"# altered cache\n")
        self.assert_success(self.run_package())
        self.assertEqual(self.tool.read_bytes(), TOOL_BYTES)
        self.assertTrue(self.curl_log.exists())

    def test_invalid_cache_and_download_cannot_fall_back_to_path(self):
        self.write_executable(self.tool, TOOL_BYTES + b"# altered cache\n")
        self.download.write_bytes(TOOL_BYTES + b"# altered download\n")
        self.write_executable(self.root / "bin/appimagetool", TOOL_BYTES)
        self.assert_rejected(self.run_package())

    def test_invalid_cache_and_interrupted_download_leave_no_artifact(self):
        self.write_executable(self.tool, TOOL_BYTES + b"# altered cache\n")
        self.env["FIXTURE_CURL_MODE"] = "fail"
        self.assert_rejected(self.run_package())

    def test_path_tool_is_explicitly_trusted_without_pin(self):
        path_tool = self.root / "bin/appimagetool"
        self.write_executable(path_tool, TOOL_BYTES + b"# intentionally unpinned\n")
        result = self.run_package()
        self.assert_success(result)
        self.assertIn(f"Using system appimagetool at {path_tool} (not checksum-pinned).", result.stdout)
        self.assertFalse(self.tool.exists())
        self.assertFalse(self.curl_log.exists())


if __name__ == "__main__":
    unittest.main()
