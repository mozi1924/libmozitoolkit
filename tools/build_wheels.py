#!/usr/bin/env python3
"""
Wheel build script for libmozitoolkit (libmtk_py).

Builds abi3-compatible Python wheels for libmtk_py using maturin.
Compatible with Python 3.10+, Blender 4.2+ (Python 3.11), and Blender 5.x (Python 3.13).
"""

import argparse
import os
from pathlib import Path
import shutil
import subprocess
import sys


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


def build_wheels(
    out_dir: Path | None = None,
    release: bool = True,
    target: str | None = None,
    sdist: bool = False,
) -> list[Path]:
    """Builds libmtk_py wheels using maturin."""
    repo_root = Path(__file__).resolve().parent.parent
    py_manifest = repo_root / "bindings" / "mtk-py" / "Cargo.toml"

    if not py_manifest.exists():
        raise FileNotFoundError(f"Missing Cargo.toml at {py_manifest}")

    if out_dir is None:
        out_dir = repo_root / "target" / "wheels"
    out_dir = Path(out_dir).resolve()
    out_dir.mkdir(parents=True, exist_ok=True)

    maturin_cmd = find_maturin_executable()
    print(f"📦 Using maturin: {' '.join(maturin_cmd)}")
    print(f"📁 Manifest path: {py_manifest}")
    print(f"🎯 Output directory: {out_dir}")

    cmd = list(maturin_cmd) + [
        "build",
        "-m",
        str(py_manifest),
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
    result = subprocess.run(cmd, cwd=str(repo_root), check=False)
    if result.returncode != 0:
        raise RuntimeError(f"maturin build failed with exit code {result.returncode}")

    built_wheels = list(out_dir.glob("libmtk_py*.whl"))
    print(f"\n✅ Build successful! Produced {len(built_wheels)} wheel(s):")
    for whl in built_wheels:
        print(f"  • {whl.name} ({whl.stat().st_size / 1024 / 1024:.2f} MB)")

    return built_wheels


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
        help="Target platform triple (e.g. aarch64-apple-darwin, x86_64-unknown-linux-gnu)",
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

    repo_root = Path(__file__).resolve().parent.parent
    mozi_wheels_dir = repo_root.parent / "MoziToolKit" / "wheels"

    out_path = Path(args.out_dir) if args.out_dir else None
    if args.copy_to_mozitoolkit and out_path is None:
        out_path = mozi_wheels_dir

    built = build_wheels(
        out_dir=out_path,
        release=not args.debug,
        target=args.target,
    )

    if args.copy_to_mozitoolkit and out_path != mozi_wheels_dir:
        mozi_wheels_dir.mkdir(parents=True, exist_ok=True)
        for whl in built:
            dest = mozi_wheels_dir / whl.name
            shutil.copy2(whl, dest)
            print(f"📋 Copied to MoziToolKit wheels: {dest.name}")


if __name__ == "__main__":
    main()
