# Gemini deterministic sprite pack

Imported on 2026-10-24 from the user-supplied PNGs in:
`/home/gmuller/.gemini/antigravity/scratch/screen-antics-remaster/edited_deterministic`

All 116 tilemaps match the exported atlas manifests. They provide all 2,391
sprite frames at exactly 4× native canvas dimensions. Frame order, offsets,
straight RGBA values and partial transparency are preserved without recoloring,
resizing or trimming. `sprites/import-manifest.json` records source PNG SHA-256
hashes and frame membership; each frame has a PNG and a runtime SAHD `.rgba` file.
The approximately 432 MiB runtime cache is ignored by Git. Rebuild missing cache
files with `python tools/prepare_asset_pack.py assets/gemini-deterministic`
(Python 3 and Pillow required). The PNG pack is approximately 46 MiB.

Run from the project root:

```sh
./Play.sh --asset-pack assets/gemini-deterministic
./Play.sh --asset-pack assets/gemini-deterministic --fullscreen
./Play.sh --asset-pack assets/gemini-deterministic --resolution 3440x1440
```

H switches between this pack and original indexed artwork. The complete new remaster is selected by `PlayGemini.sh`. The ordinary
`Play.sh` still selects the existing enhanced-original mode. The supplied
directory contains no complete `.SCR` background replacements, so this pack
uses the existing enhanced-original full backgrounds and widescreen extension.
Scenery sprites, including BACKGRND and TRUNK, use the supplied new artwork.

To reproduce the import, use a fresh output directory:

```sh
python tools/unpack_sprite_tilesets.py \
  --pack exports/sprite-tilemaps \
  --edited-dir /home/gmuller/.gemini/antigravity/scratch/screen-antics-remaster/edited_deterministic \
  --output assets/gemini-deterministic/sprites
```

The importer validates dimensions, gutters, alpha, and original frame occupancy
before writing. The runtime accepts other converted packs with
`./Play.sh --asset-pack /path/to/pack`, where the directory contains `sprites/`
and optionally `backgrounds/` in the existing SAHD format.
