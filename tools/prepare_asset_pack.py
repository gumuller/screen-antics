#!/usr/bin/env python3
"""Build missing/stale SAHD runtime files from an imported pack's PNG frames."""
import argparse
import os
from pathlib import Path
import struct


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pack", type=Path, help="Pack directory containing sprites/")
    args = parser.parse_args()
    if (args.pack / "scene-art.tsv").is_file():
        frames = sorted((args.pack / "clips").glob("*/*.png"))
        frames += sorted((args.pack / "backgrounds").glob("*/*.png"))
        frames += sorted((args.pack / "backdrops").glob("*.png"))
    else:
        frames = sorted((args.pack / "sprites").glob("*.BMP/*.png"))
        frames += sorted((args.pack / "backgrounds").glob("*.png"))
        frames += sorted((args.pack / "widescreen").glob("*/*.png"))
    if not frames:
        parser.error(f"No imported PNG frames in {args.pack}")
    rebuilt = 0
    for source in frames:
        target = source.with_suffix(".rgba")
        if target.exists() and target.stat().st_mtime_ns >= source.stat().st_mtime_ns:
            continue
        # Pillow is only needed when rebuilding the derived runtime cache.
        from PIL import Image
        with Image.open(source) as image:
            rgba = image.convert("RGBA")
            content = b"SAHD" + struct.pack("<II", *rgba.size) + rgba.tobytes()
        temporary = target.with_suffix(f".rgba.{os.getpid()}.tmp")
        try:
            temporary.write_bytes(content)
            temporary.replace(target)
        finally:
            temporary.unlink(missing_ok=True)
        rebuilt += 1
    if rebuilt:
        print(f"Prepared {rebuilt} runtime frames in {args.pack}")


if __name__ == "__main__":
    main()
