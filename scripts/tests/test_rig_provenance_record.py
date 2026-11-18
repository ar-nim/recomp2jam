"""Guards for the committed t240 provenance record (#308).

The committed directory is a hash-pinned pointer: its hashes must stay tied
to the local, git-ignored pack and to the citation that pack accepted.
These seam tests use the committed JSON/citation text only; no copyrighted
capture bytes are copied into the test fixtures.
"""
from __future__ import annotations

import hashlib
import shutil
import subprocess
import tempfile
import unittest
from pathlib import Path

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
PROVENANCE_SCRIPT = SCRIPTS_DIR / "rig-provenance.sh"
COMMITTED_RECORD = SCRIPTS_DIR.parent / "docs" / "captures" / "t240"
RECORD_FILES = (
    "record-hashes.txt",
    "wine-provenance.json",
    "wine-manifest.json",
    "wine-citation.txt",
)
LOCAL_PACK_NAMES = {
    "provenance.json": "wine-provenance.json",
    "manifest.json": "wine-manifest.json",
    "citation.txt": "wine-citation.txt",
}


def check_record(record_dir: Path, *extra_args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["bash", str(PROVENANCE_SCRIPT), "check-record", str(record_dir), *extra_args],
        capture_output=True,
        text=True,
        check=False,
    )


def replace_bytes(path: Path, old: bytes, new: bytes) -> None:
    content = path.read_bytes()
    assert old in content
    path.write_bytes(content.replace(old, new, 1))


def repin_file(record_dir: Path, name: str) -> None:
    pins = record_dir / "record-hashes.txt"
    digest = hashlib.sha256((record_dir / name).read_bytes()).hexdigest()
    lines = [
        f"{digest}  {name}" if line.endswith(f"  {name}") else line
        for line in pins.read_text(encoding="utf-8").splitlines()
    ]
    pins.write_text("\n".join(lines) + "\n", encoding="utf-8")


class CommittedRecordCheckTest(unittest.TestCase):
    def setUp(self) -> None:
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.record = Path(temporary.name) / "record"
        self.record.mkdir()
        for name in RECORD_FILES:
            shutil.copyfile(COMMITTED_RECORD / name, self.record / name)

    def build_local_pack(self) -> Path:
        pack = self.record.parent / "local-pack"
        pack.mkdir()
        for local_name, record_name in LOCAL_PACK_NAMES.items():
            shutil.copyfile(self.record / record_name, pack / local_name)
        return pack

    def assert_record_ok(self, *extra_args: str) -> subprocess.CompletedProcess[str]:
        result = check_record(self.record, *extra_args)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        return result

    def assert_record_fails(self, *extra_args: str) -> subprocess.CompletedProcess[str]:
        result = check_record(self.record, *extra_args)
        self.assertNotEqual(result.returncode, 0, result.stdout)
        self.assertIn("RECORD CHECK FAILED", result.stderr)
        return result

    def test_committed_record_passes_without_the_local_pack(self) -> None:
        result = self.assert_record_ok()
        self.assertIn("citation quotes the pinned provenance", result.stdout)

    def test_one_corrupted_citation_byte_fails_loudly(self) -> None:
        replace_bytes(self.record / "wine-citation.txt", b"Provenance", b"Xrovenance")
        result = self.assert_record_fails()
        self.assertIn("wine-citation.txt sha256", result.stderr)

    def test_citation_drift_from_pinned_manifest_hash_fails_loudly(self) -> None:
        provenance = (self.record / "wine-provenance.json").read_text(encoding="utf-8")
        manifest_hash = provenance.split('"manifest_sha256": "')[1].split('"')[0]
        replace_bytes(
            self.record / "wine-citation.txt",
            manifest_hash.encode("ascii"),
            b"0" * len(manifest_hash),
        )
        repin_file(self.record, "wine-citation.txt")
        result = self.assert_record_fails()
        self.assertIn("citation does not quote the pinned manifest sha256", result.stderr)

    def test_local_pack_matching_the_record_passes(self) -> None:
        pack = self.build_local_pack()
        result = self.assert_record_ok("--local-pack", str(pack))
        self.assertIn("committed copies match the qualified local pack", result.stdout)

    def test_drifted_local_pack_citation_fails(self) -> None:
        pack = self.build_local_pack()
        replace_bytes(pack / "citation.txt", b"Provenance", b"Xrovenance")
        result = self.assert_record_fails("--local-pack", str(pack))
        self.assertIn("local pack citation", result.stderr)


if __name__ == "__main__":
    unittest.main()
