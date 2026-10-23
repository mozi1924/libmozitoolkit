#!/usr/bin/env python3
"""
Wheel build script for libmozitoolkit (libmtk_py + libmtk_blender_dna).

Builds abi3-compatible Python wheels for the core and standalone host adapter using maturin.
Compatible with Python 3.10+, Blender 4.2+ (Python 3.11), and Blender 5.x (Python 3.13).
"""

import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys

# Windows consoles/pipes default to the legacy ANSI code page, which cannot
# encode the emoji progress output. Force UTF-8 so build logs never crash.
for _stream in (sys.stdout, sys.stderr):
    if hasattr(_stream, "reconfigure"):
        _stream.reconfigure(encoding="utf-8", errors="replace")


def find_maturin_executable() -> list[str]:
    """Finds a working maturin executable or python -m maturin invocation."""
    repo_root = Path(__file__).resolve().parent.parent
    for venv_dir in [repo_root / ".venv", repo_root / "venv"]:
        candidate = venv_dir / "bin" / "maturin"
        if candidate.exists() and os.access(candidate, os.X_OK):
            return [str(candidate)]
        candidate_win = venv_dir / "Scripts" / "maturin.exe"
        if candidate_win.exists() and os.access(candidate_win, os.X_OK):
            return [str(candidate_win)]

    which_maturin = shutil.which("maturin")
    if which_maturin:
        return [which_maturin]

    cargo_maturin = Path.home() / ".cargo" / "bin" / "maturin"
    if cargo_maturin.exists() and os.access(cargo_maturin, os.X_OK):
        return [str(cargo_maturin)]

    python_candidates = [
        sys.executable,
        "/Applications/Blender.app/Contents/Resources/5.2/python/bin/python3.13",
        "/Applications/Blender.app/Contents/Resources/4.2/python/bin/python3.11",
        "/opt/blender-5.2.0-linux-x64/5.2/python/bin/python3.13",
        "/opt/homebrew/bin/python3",
        shutil.which("python3"),
    ]
    for py in python_candidates:
        if py and Path(py).exists():
            try:
                res = subprocess.run(
                    [py, "-m", "maturin", "--version"],
                    stdout=subprocess.PIPE,
                    stderr=subprocess.PIPE,
                    text=True,
                    check=False,
                )
                if res.returncode == 0:
                    return [py, "-m", "maturin"]
            except Exception:
                pass

    raise RuntimeError(
        "Could not find 'maturin'. Please install it via 'brew install maturin', 'pip install maturin', or 'cargo install maturin'."
    )


def build_wheel_for_manifest(
    manifest: Path,
    *,
    out_dir: Path,
    release: bool = True,
    target: str | None = None,
    sdist: bool = False,
) -> list[Path]:
    """Builds one maturin wheel from a manifest."""
    if not manifest.exists():
        raise FileNotFoundError(f"Missing Cargo.toml at {manifest}")

    out_dir = Path(out_dir).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    maturin_cmd = find_maturin_executable()
    print(f"📦 Using maturin: {' '.join(maturin_cmd)}")
    print(f"📁 Manifest path: {manifest}")
    print(f"🎯 Output directory: {out_dir}")

    cmd = list(maturin_cmd) + [
        "build",
        "-m",
        str(manifest),
        "--out",
        str(out_dir),
    ]
    if release:
        cmd.append("--release")
    if target:
        cmd.extend(["--target", target])
    if sdist:
        cmd.append("--sdist")

    print(f"🚀 Running: {' '.join(cmd)}")
    result = subprocess.run(cmd, cwd=str(manifest.parent.parent.parent), check=False)
    if result.returncode != 0:
        raise RuntimeError(f"maturin build failed with exit code {result.returncode}")
    return list(out_dir.glob("*.whl"))


def build_wheels(
    out_dir: Path | None = None,
    release: bool = True,
    target: str | None = None,
    sdist: bool = False,
) -> list[Path]:
    """Builds both the core and standalone host-adapter wheels."""
    repo_root = Path(__file__).resolve().parent.parent
    if out_dir is None:
        out_dir = repo_root / "target" / "wheels"
    out_dir = Path(out_dir).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    manifests = [
        repo_root / "bindings" / "mtk-py" / "Cargo.toml",
        repo_root / "adapters" / "blender" / "mtk-blender-dna-py" / "Cargo.toml",
    ]
    before = set(out_dir.glob("*.whl")) if out_dir.exists() else set()
    for manifest in manifests:
        build_wheel_for_manifest(
            manifest,
            out_dir=out_dir,
            release=release,
            target=target,
            sdist=sdist,
        )

    built_wheels = sorted(set(out_dir.glob("*.whl")) - before)
    if not built_wheels:
        built_wheels = sorted(out_dir.glob("*.whl"))
    print(f"\n✅ Build successful! Produced {len(built_wheels)} wheel(s):")
    for whl in built_wheels:
        print(f"  • {whl.name} ({whl.stat().st_size / 1024 / 1024:.2f} MB)")
    return built_wheels


TARGET_ALIASES = {
    "linux-x64": "x86_64-unknown-linux-gnu",
    "linux-arm64": "aarch64-unknown-linux-gnu",
    "windows-x64": "x86_64-pc-windows-msvc",
    "windows-arm64": "aarch64-pc-windows-msvc",
    "macos-x64": "x86_64-apple-darwin",
    "macos-arm64": "aarch64-apple-darwin",
}


def main():
    parser = argparse.ArgumentParser(description="Build libmtk_py Python wheels")
    parser.add_argument(
        "--out-dir",
        "-o",
        type=str,
        default=None,
        help="Destination directory for built wheels",
    )
    parser.add_argument(
        "--target",
        type=str,
        default=None,
        help="Target platform triple (or alias: linux-x64, linux-arm64, windows-x64, windows-arm64, macos-x64, macos-arm64)",
    )
    parser.add_argument(
        "--debug",
        action="store_true",
        help="Build in debug mode instead of release mode",
    )
    parser.add_argument(
        "--copy-to-mozitoolkit",
        action="store_true",
        help="Automatically copy built wheels to MoziToolKit/wheels/",
    )

    args = parser.parse_args()

    target_triple = TARGET_ALIASES.get(args.target, args.target) if args.target else None


    repo_root = Path(__file__).resolve().parent.parent
    mozi_wheels_dir = repo_root.parent / "MoziToolKit" / "wheels"

    out_path = Path(args.out_dir) if args.out_dir else None
    if args.copy_to_mozitoolkit and out_path is None:
        out_path = mozi_wheels_dir

    built = build_wheels(
        out_dir=out_path,
        release=not args.debug,
        target=target_triple,
    )

    if args.copy_to_mozitoolkit and out_path != mozi_wheels_dir:
        mozi_wheels_dir.mkdir(parents=True, exist_ok=True)
        for whl in built:
            dest = mozi_wheels_dir / whl.name
            shutil.copy2(whl, dest)
            print(f"📋 Copied to MoziToolKit wheels: {dest.name}")


if __name__ == "__main__":
    main()
