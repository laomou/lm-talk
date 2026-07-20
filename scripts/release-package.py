#!/usr/bin/env python3
"""Package the lm_node release binary with provenance and checksums."""

from __future__ import annotations

import argparse
import datetime as dt
import hashlib
import os
import shutil
import stat
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from pathlib import Path


def run_text(args: list[str], cwd: Path, required: bool = False) -> str:
    try:
        completed = subprocess.run(
            args,
            cwd=cwd,
            check=required,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
        )
        if completed.returncode != 0:
            return "unknown"
        return completed.stdout.strip() or "unknown"
    except (OSError, subprocess.CalledProcessError):
        return "unknown"


def sha256_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def add_tree_to_tar(tar: tarfile.TarFile, root: Path, arc_root: str) -> None:
    for path in sorted(root.rglob("*")):
        arcname = str(Path(arc_root) / path.relative_to(root))
        tar.add(path, arcname=arcname, recursive=False)


def add_tree_to_zip(zipf: zipfile.ZipFile, root: Path, arc_root: str) -> None:
    for path in sorted(root.rglob("*")):
        arcname = str(Path(arc_root) / path.relative_to(root)).replace(os.sep, "/")
        zipf.write(path, arcname=arcname)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True, help="Rust target triple used for cargo build")
    parser.add_argument("--package-name", required=True, help="Release artifact base name")
    parser.add_argument("--out-dir", default="dist", help="Directory for archives and checksum files")
    parser.add_argument("--cargo-features", default="", help="Comma/space separated Cargo features used for this build")
    parser.add_argument("--repo-root", default=".", help="Repository root")
    parser.add_argument(
        "--web-admin-zip",
        default="",
        help="Optional path to node_admin.zip to bundle into the archive",
    )
    parser.add_argument(
        "--archive-format",
        choices=("auto", "tar.gz", "zip"),
        default="auto",
        help="Archive format; auto uses zip for Windows and tar.gz otherwise",
    )
    args = parser.parse_args()

    repo = Path(args.repo_root).resolve()
    out_dir = (repo / args.out_dir).resolve()
    status = run_text(["git", "status", "--porcelain"], repo)
    dirty = "unknown" if status == "unknown" else str(bool(status)).lower()

    is_windows_target = "pc-windows" in args.target or "windows" in args.target
    binary_name = "lm_node.exe" if is_windows_target else "lm_node"
    binary_path = repo / "target" / args.target / "release" / binary_name
    if not binary_path.is_file():
        print(f"error: built binary not found: {binary_path}", file=sys.stderr)
        return 2

    archive_format = args.archive_format
    if archive_format == "auto":
        archive_format = "zip" if is_windows_target else "tar.gz"

    commit = run_text(["git", "rev-parse", "HEAD"], repo)
    out_dir.mkdir(parents=True, exist_ok=True)

    build_time = dt.datetime.now(dt.timezone.utc).replace(microsecond=0).isoformat().replace("+00:00", "Z")
    binary_sha = sha256_file(binary_path)
    rustc = run_text(["rustc", "-Vv"], repo)
    cargo = run_text(["cargo", "-V"], repo)
    cargo_features = " ".join(args.cargo_features.replace(",", " ").split())
    cargo_features_display = cargo_features or "default"

    with tempfile.TemporaryDirectory(prefix="lm-node-release-") as tmp:
        staging = Path(tmp) / args.package_name
        staging.mkdir(parents=True)
        staged_binary = staging / binary_name
        shutil.copy2(binary_path, staged_binary)
        if not is_windows_target:
            mode = staged_binary.stat().st_mode
            staged_binary.chmod(mode | stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH)

        docs_to_include = [
            "README.md",
            "README_zh.md",
            "LICENSE",
            "SECURITY.md",
            "SECURITY_zh.md",
            "docs/README.md",
            "docs/deploy/NODE_CONFIG.md",
            "docs/overview/DEV_WORKFLOW.md",
            "docs/release/RELEASE_CHECKLIST.md",
            "docs/release/RELEASE_SIGNING.md",
        ]
        for relative in docs_to_include:
            source = repo / relative
            if source.is_file():
                destination = staging / relative
                destination.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(source, destination)

        example_config = repo / "docs/examples/lm-node.config.example.json"
        if example_config.is_file():
            shutil.copy2(example_config, staging / "node.config.example.json")

        if args.web_admin_zip:
            admin_zip = Path(args.web_admin_zip)
            if not admin_zip.is_absolute():
                admin_zip = (repo / admin_zip).resolve()
            if admin_zip.is_file():
                shutil.copy2(admin_zip, staging / "node_admin.zip")
            else:
                raise SystemExit(f"--web-admin-zip not found: {admin_zip}")

        release_info = staging / "RELEASE_INFO.txt"
        release_info.write_text(
            "LM Talk native node release artifact\n"
            f"package={args.package_name}\n"
            f"target={args.target}\n"
            f"binary={binary_name}\n"
            f"binary_sha256={binary_sha}\n"
            f"cargo_features={cargo_features_display}\n"
            f"source_commit={commit}\n"
            f"source_dirty={dirty}\n"
            f"build_time_utc={build_time}\n"
            "\n[rustc -Vv]\n"
            f"{rustc}\n"
            "\n[cargo -V]\n"
            f"{cargo}\n",
            encoding="utf-8",
        )

        if archive_format == "zip":
            archive = out_dir / f"{args.package_name}.zip"
            with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED, compresslevel=9) as zipf:
                add_tree_to_zip(zipf, staging, args.package_name)
        else:
            archive = out_dir / f"{args.package_name}.tar.gz"
            with tarfile.open(archive, "w:gz") as tar:
                add_tree_to_tar(tar, staging, args.package_name)

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
