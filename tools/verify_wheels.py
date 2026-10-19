#!/usr/bin/env python3
"""
Verify that a directory contains one abi3 wheel per supported platform.

Used by the wheel release workflow as a release gate, and useful locally:

    python tools/verify_wheels.py dist
    python tools/verify_wheels.py target/wheels --platforms linux-x64
    python tools/verify_wheels.py dist --prefix libmtk_py --allow-non-abi3

Exit codes:
    0  every requested platform has exactly one wheel
    1  a platform is missing, duplicated, or has an unexpected wheel
    2  invalid command-line usage
"""

from __future__ import annotations

import argparse
from pathlib import Path
import sys

DEFAULT_PLATFORMS = (
    "linux-x64",
    "linux-arm64",
    "windows-x64",
    "windows-arm64",
    "macos-x64",
    "macos-arm64",
)


def wheel_tags(filename: str) -> tuple[str, str, list[str]]:
    """Return (python_tag, abi_tag, platform_tags) parsed from a wheel filename.

    Wheel filename layout::

        {distribution}-{version}(-{build})?-{python}-{abi}-{platform}.whl

    The platform component may itself be a dot-separated list of compatible
    tags, e.g. ``manylinux_2_17_x86_64.manylinux2014_x86_64``.
    """
    if not filename.endswith(".whl"):
        raise ValueError(f"not a wheel: {filename}")
    parts = filename[: -len(".whl")].split("-")
    if len(parts) < 5:
        raise ValueError(f"malformed wheel filename: {filename}")
    return parts[-3], parts[-2], parts[-1].split(".")


def platforms_from_tags(tags: list[str]) -> set[str]:
    """Map a wheel's platform tags to Blender-style platform identifiers."""
    found: set[str] = set()
    for tag in tags:
        tag = tag.lower()
        if tag.startswith(("manylinux", "linux")):
            if tag.endswith("x86_64"):
                found.add("linux-x64")
            elif tag.endswith(("aarch64", "arm64")):
                found.add("linux-arm64")
        elif tag == "win_amd64":
            found.add("windows-x64")
        elif tag == "win_arm64":
            found.add("windows-arm64")
        elif tag.startswith("macosx"):
            if tag.endswith("universal2"):
                found.update(("macos-x64", "macos-arm64"))
            elif tag.endswith("arm64"):
                found.add("macos-arm64")
            elif tag.endswith("x86_64"):
                found.add("macos-x64")
    return found


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Verify a wheel directory covers every supported platform exactly once.",
    )
    parser.add_argument(
        "directory",
        nargs="?",
        default="dist",
        help="Directory containing the downloaded .whl files (default: dist)",
    )
    parser.add_argument(
        "--platforms",
        default=",".join(DEFAULT_PLATFORMS),
        help="Comma-separated platform ids to require (default: all six)",
    )
    parser.add_argument(
        "--prefix",
        default="libmtk_py",
        help="Only consider wheels whose filename starts with this prefix (default: libmtk_py)",
    )
    parser.add_argument(
        "--allow-non-abi3",
        action="store_true",
        help="Do not reject wheels that are not tagged as abi3",
    )
    args = parser.parse_args()

    platforms = [item.strip() for item in args.platforms.split(",") if item.strip()]
    unknown = sorted(set(platforms) - set(DEFAULT_PLATFORMS))
    if unknown:
        print(f"error: unknown platform id(s): {', '.join(unknown)}", file=sys.stderr)
        print(f"       supported: {', '.join(DEFAULT_PLATFORMS)}", file=sys.stderr)
        return 2

    wheel_dir = Path(args.directory)
    if not wheel_dir.is_dir():
        print(f"error: not a directory: {wheel_dir}", file=sys.stderr)
        return 1

    wheels = sorted(wheel_dir.glob("*.whl"))
    if not wheels:
        print(f"error: no .whl files found in {wheel_dir}", file=sys.stderr)
        return 1

    by_platform: dict[str, list[Path]] = {platform: [] for platform in platforms}
    ignored: list[Path] = []
    errors: list[str] = []

    for wheel in wheels:
        if args.prefix and not wheel.name.lower().startswith(args.prefix.lower()):
            ignored.append(wheel)
            continue

        try:
            python_tag, abi_tag, tags = wheel_tags(wheel.name)
        except ValueError as exc:
            errors.append(str(exc))
            continue

        if not args.allow_non_abi3 and abi_tag != "abi3":
            errors.append(
                f"{wheel.name}: expected an abi3 wheel "
                f"(python={python_tag}, abi={abi_tag})"
            )

        matched = platforms_from_tags(tags) & set(platforms)
        if not matched:
            errors.append(f"{wheel.name}: no supported platform tag in {tags}")
            continue

        for platform in matched:
            by_platform[platform].append(wheel)

    print(f"Checked {len(wheels)} wheel(s) in {wheel_dir}:")
    for platform in platforms:
        matches = by_platform[platform]
        if not matches:
            print(f"  MISSING   {platform}")
            errors.append(f"{platform}: no wheel")
        elif len(matches) > 1:
            names = ", ".join(wheel.name for wheel in matches)
            print(f"  DUPLICATE {platform}: {names}")
            errors.append(f"{platform}: {len(matches)} wheels")
        else:
            print(f"  OK        {platform:<13} {matches[0].name}")

    for wheel in ignored:
        print(f"  ignored   (unrelated wheel) {wheel.name}")

    if errors:
        print("\nWheel matrix is invalid:", file=sys.stderr)
        for error in errors:
            print(f"  - {error}", file=sys.stderr)
        return 1

    print(f"\n✅ Wheel matrix covers all {len(platforms)} platform(s).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
