#!/usr/bin/env python3
"""Package the lm_node native binary and config template."""

from __future__ import annotations

import argparse
import hashlib
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True, help="Rust target triple used for cargo build")
    parser.add_argument("--package-name", required=True, help="Release artifact base name")
    parser.add_argument("--out-dir", default="dist", help="Directory for archives and checksum files")
    parser.add_argument("--cargo-features", default="", help="Comma/space separated Cargo features used for this build")
    parser.add_argument("--repo-root", default=".", help="Repository root")
    parser.add_argument(
        "--archive-format",
        choices=("auto", "tar.gz", "zip"),
        default="auto",
        help="Archive format; auto uses zip for Windows and tar.gz otherwise",
    )
    args = parser.parse_args()

    repo = Path(args.repo_root).resolve()
    out_dir = (repo / args.out_dir).resolve()
    is_windows_target = "pc-windows" in args.target or "windows" in args.target
    binary_name = "lm_node.exe" if is_windows_target else "lm_node"
    binary_path = repo / "target" / args.target / "release" / binary_name
    if not binary_path.is_file():
        print(f"error: built binary not found: {binary_path}", file=sys.stderr)
        return 2

    archive_format = args.archive_format
    if archive_format == "auto":
        archive_format = "zip" if is_windows_target else "tar.gz"

    out_dir.mkdir(parents=True, exist_ok=True)

    with tempfile.TemporaryDirectory(prefix="lm-node-release-") as tmp:
        staging = Path(tmp) / args.package_name
        staging.mkdir(parents=True)
        staged_binary = staging / binary_name
        shutil.copy2(binary_path, staged_binary)
        if not is_windows_target:
            mode = staged_binary.stat().st_mode
            staged_binary.chmod(mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)

        example_config = repo / "docs/examples/lm-node.config.example.json"
        if example_config.is_file():
            shutil.copy2(example_config, staging / "node.config.example.json")

        if archive_format == "zip":
            archive = out_dir / f"{args.package_name}.zip"
            with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as zipf:
                for path in sorted(staging.iterdir()):
                    zipf.write(path, f"{args.package_name}/{path.name}")
        else:
            archive = out_dir / f"{args.package_name}.tar.gz"
            with tarfile.open(archive, "w:gz") as tar:
                for path in sorted(staging.iterdir()):
                    tar.add(path, arcname=f"{args.package_name}/{path.name}", recursive=False)

    archive_sha = sha256_file(archive)
    checksum_file = archive.with_suffix(archive.suffix + ".sha256")
    if archive.name.endswith(".tar.gz"):
        checksum_file = archive.with_name(archive.name + ".sha256")
    checksum_file.write_text(f"{archive_sha}  {archive.name}\n", encoding="utf-8")

    print(archive)
    print(checksum_file)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
