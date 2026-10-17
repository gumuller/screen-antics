#!/usr/bin/env python3
"""Losslessly convert generated PNG assets into the native SAHD RGBA container.
This performs format conversion only; artistic edits are made with ImageGen.
"""
import struct,sys
from pathlib import Path
from PIL import Image
src,dst=map(Path,sys.argv[1:3])
im=Image.open(src).convert('RGBA')
dst.parent.mkdir(parents=True,exist_ok=True)
dst.write_bytes(b'SAHD'+struct.pack('<II',*im.size)+im.tobytes())
print(f'{dst}: {im.width}x{im.height} RGBA')
