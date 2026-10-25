#!/usr/bin/env python3
"""Package decoded backgrounds, registered widescreen canvases and remaster context.

Technical export only: uses Pillow for lossless packing and annotation; no art is repainted.
Run export_remaster_context.rs first. Requires Pillow and ffprobe.
"""
from __future__ import annotations
import argparse
from collections import defaultdict
import csv
import hashlib
import json
import math
from pathlib import Path
import shutil
import subprocess
import wave
import zipfile
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
PAIRED = ('BACKGRND', 'TRUNK', 'MJDIVE', 'MJBOTTLE', 'THNKBUBL', 'STNDLAY', 'CLOUDS', 'HOLIDAY')
PANORAMA = {'ISLETEMP.SCR', 'ISLAND2.SCR', 'OCEAN00.SCR', 'OCEAN01.SCR', 'OCEAN02.SCR', 'NIGHT.SCR'}
RATIOS = {'16_9': (16, 9), '21_9': (21, 9), '43_18': (43, 18)}


def dump(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, indent=2) + '\n')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def font(size=22):
    for path in ('/usr/share/fonts/TTF/DejaVuSans.ttf', '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf',
                 '/usr/share/fonts/liberation/LiberationSans-Regular.ttf'):
        if Path(path).exists():
            return ImageFont.truetype(path, size)
    return ImageFont.load_default(size=size)


def checker(size):
    tile = Image.new('RGB', (64, 64), '#262d39')
    d = ImageDraw.Draw(tile)
    d.rectangle((32, 0, 63, 31), fill='#364052')
    d.rectangle((0, 32, 31, 63), fill='#364052')
    out = Image.new('RGB', size)
    for y in range(0, size[1], 64):
        for x in range(0, size[0], 64):
            out.paste(tile, (x, y))
    return out


def caption(image, text, height=48):
    out = Image.new('RGB', (image.width, image.height + height), '#101923')
    out.paste(image, (0, height))
    ImageDraw.Draw(out).text((12, 10), text, font=font(20), fill='white')
    return out


def background_guide(image, source_size, rect, label):
    base = checker(image.size).convert('RGBA')
    base.alpha_composite(image.convert('RGBA'))
    layer = Image.new('RGBA', image.size)
    d = ImageDraw.Draw(layer)
    x0, y0, w, h = rect
    for x in range(0, source_size[0], 64):
        xx = x0 + x * 4
        d.line((xx, y0, xx, y0 + h - 1), fill=(255, 255, 255, 80), width=2)
        d.text((xx + 5, y0 + 7), f'x={x}', font=font(18), fill='white', stroke_width=2, stroke_fill='black')
    for y in range(64, source_size[1], 64):
        yy = y0 + y * 4
        d.line((x0, yy, x0 + w - 1, yy), fill=(255, 255, 255, 80), width=2)
        d.text((x0 + 5, yy + 4), f'y={y}', font=font(18), fill='white', stroke_width=2, stroke_fill='black')
    d.rectangle((x0, y0, x0 + w - 1, y0 + h - 1), outline='#ffbf47', width=7)
    base.alpha_composite(layer)
    return caption(base.convert('RGB'), label)


def write_csv(path, rows, fields):
    with path.open('w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=fields)
        writer.writeheader()
        writer.writerows(rows)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=ROOT / 'exports/.remaster-context-source')
    parser.add_argument('--atlases', type=Path, default=Path('/home/gmuller/.gemini/antigravity/scratch/screen-antics-remaster/edited_deterministic'))
    parser.add_argument('--output', type=Path, default=ROOT / 'exports/remaster-handoff')
    parser.add_argument('--archive', type=Path, default=ROOT / 'exports/screen-antics-remaster-handoff.zip')
    args = parser.parse_args()
    if args.output.exists() and any(args.output.iterdir()):
        parser.error('Choose an empty --output directory to preserve existing exports.')
    if args.archive.exists():
        parser.error('Choose an unused --archive path.')
    out, source = args.output, args.source
    backgrounds = json.loads((source / 'backgrounds.json').read_text())
    scenes = json.loads((source / 'scenes.json').read_text())
    sounds = json.loads((source / 'audio.json').read_text())
    imported = json.loads((ROOT / 'assets/gemini-deterministic/sprites/import-manifest.json').read_text())
    for page in imported['pages']:
        if sha(args.atlases / page['source']) != page['png_sha256']:
            parser.error(f"{page['source']} changed since the sprite import; previews and style references would disagree.")
    for folder in ('backgrounds/native', 'editing/backgrounds', 'editing/widescreen', 'guides/backgrounds',
                   'guides/widescreen', 'masks/widescreen', 'reference/stills', 'reference/storyboards',
                   'reference/placement-guides', 'reference/previews', 'reference/original-paired-atlases',
                   'style/current-sprite-atlases', 'context/sprite-manifests', 'context/runtime-source',
                   'context/findings', 'context/timelines', 'context/raw-scripts', 'audio/original', 'tools'):
        (out / folder).mkdir(parents=True, exist_ok=True)
    manifest = {
        'format': 'screen-antics-remaster-handoff-v1', 'scale': 4,
        'source_resource_sha256': sha(ROOT / 'original/RESOURCE.001'),
        'source_executable_sha256': sha(ROOT / 'original/SCRANTIC.EXE'),
        'source_sprite_import_manifest_sha256': sha(ROOT / 'assets/gemini-deterministic/sprites/import-manifest.json'),
        'coordinates': 'Top-left origin; rects x,y,width,height; clip rectangles x1,y1,x2,y2 with exclusive far edges; frame indices zero-based except timeline frame counters (one-based).',
        'preview': {'fps': 20, 'script_ticks_per_second': 60, 'size': [640, 360], 'audio': False,
                    'source': 'Current native Rust reconstruction, complete bounded scene-browser timelines',
                    'limits': 'Preview samples every 3 script ticks. Final VM compositions shown; display-only transitions/crossfades and their extra wall time are omitted. Saved layers appear in images even when absent from the current tick draw list. Not a recording of the original Windows program.'},
        'backgrounds': [], 'audio': sounds, 'sprite_style_atlases': imported['pages'],
        'runtime_support': {'backgrounds': 'Convert final 4x PNGs to SAHD and load backgrounds/NAME.SCR.rgba in a custom pack.',
                            'widescreen': 'Future manifest-aware renderer integration required; current renderer repeats edge scenery and does not use these canvases.',
                            'audio': 'Future external WAV loading/resampling required; current player reads the original EXE samples.'},
    }
    bg_cards = []
    for entry in backgrounds:
        name = entry['name']; size = (entry['width'], entry['height'])
        with Image.open(source / 'backgrounds' / f'{name}.ppm') as image:
            native = image.convert('RGB')
        native.save(out / 'backgrounds/native' / f'{name}.png')
        core = native.resize((size[0] * 4, size[1] * 4), Image.Resampling.NEAREST)
        core.save(out / 'editing/backgrounds' / f'{name}.png')
        background_guide(core, size, (0, 0, *core.size), f'{name} | native coordinate grid | retain source geometry').save(out / 'guides/backgrounds' / f'{name}.png')
        thumb = native.copy(); thumb.thumbnail((480, 300))
        thumb.save(out / 'reference/previews' / f'{name}.png')
        entry = {**entry, 'native_file': f'backgrounds/native/{name}.png', 'editing_file': f'editing/backgrounds/{name}.png',
                 'runtime_size': list(core.size), 'runtime_file': f'backgrounds/{name}.rgba',
                 'native_rgb_sha256': hashlib.sha256(native.tobytes()).hexdigest(),
                 'currently_extends_edges': name in PANORAMA, 'widescreen': []}
        for ratio, (rx, ry) in RATIOS.items():
            # Keep the core at exact 4x and center it on an exact-aspect canvas.
            # Eight-unit multiples keep dimensions and offsets integral/even.
            unit = math.ceil(max(core.width / rx, core.height / ry) / 8) * 8
            canvas_size = (rx * unit, ry * unit)
            x = (canvas_size[0] - core.width) // 2; y = (canvas_size[1] - core.height) // 2
            template = Image.new('RGBA', canvas_size, (0, 0, 0, 0))
            template.paste(core.convert('RGBA'), (x, y))
            mask = Image.new('L', canvas_size, 255)
            mask.paste(0, (x, y, x + core.width, y + core.height))
            for category in ('editing', 'guides', 'masks'):
                (out / category / 'widescreen' / ratio).mkdir(exist_ok=True)
            template.save(out / 'editing/widescreen' / ratio / f'{name}.png')
            mask.save(out / 'masks/widescreen' / ratio / f'{name}.png')
            background_guide(template, size, (x, y, *core.size), f'{name} | {rx}:{ry} | gold = fixed original stage; checkerboard = extension').save(out / 'guides/widescreen' / ratio / f'{name}.png')
            entry['widescreen'].append({'ratio': ratio, 'canvas_size': list(canvas_size), 'stage_rect_xywh': [x, y, *core.size],
                'source_to_canvas': {'scale': 4, 'offset': [x, y]},
                'editing_file': f'editing/widescreen/{ratio}/{name}.png',
                'mask_file': f'masks/widescreen/{ratio}/{name}.png', 'mask_convention': '255 = extension to paint; 0 = registered core',
                'return_file': f'widescreen/{ratio}/{name}.png'})
        manifest['backgrounds'].append(entry)
        links = ' '.join(f'<a href="editing/widescreen/{ratio}/{name}.png">{ratio.replace("_", ":")}</a>' for ratio in RATIOS)
        bg_cards.append(f'<article><h3>{name}</h3><img src="reference/previews/{name}.png" alt="{name}"><p>{size[0]}×{size[1]} source · {core.width}×{core.height} editing</p><p><a href="editing/backgrounds/{name}.png">Clean editing PNG</a> · <a href="guides/backgrounds/{name}.png">Coordinate guide</a></p><p>Wide templates: {links}</p></article>')
    print('Prepared 10 backgrounds, 30 registered widescreen templates and masks', flush=True)
    contact = Image.new('RGB', (1600, 600), '#101923'); cd = ImageDraw.Draw(contact)
    for i, bg in enumerate(manifest['backgrounds']):
        with Image.open(out / bg['native_file']) as image:
            image.thumbnail((320, 240))
            x, y = (i % 5) * 320, (i // 5) * 300
            contact.paste(image, (x, y))
            cd.text((x+8, y+250), bg['name'], font=font(16), fill='white')
            cd.text((x+8, y+274), f"{bg['width']} x {bg['height']} native", font=font(14), fill='#aabbd0')
    contact.save(out / 'reference/background-contact.png')

    # Reference pack stays self-contained: all imported atlas PNGs and original manifests.
    for page in imported['pages']:
        name = page['source']; stem = Path(name).stem
        shutil.copy2(args.atlases / name, out / 'style/current-sprite-atlases' / name)
        shutil.copy2(ROOT / 'exports/sprite-tilemaps/manifests' / f'{stem}.json', out / 'context/sprite-manifests' / f'{stem}.json')
    for stem in PAIRED:
        shutil.copy2(ROOT / 'exports/sprite-tilemaps/editing' / f'{stem}.png', out / 'reference/original-paired-atlases' / f'{stem}.png')
    dump(out / 'style/import-manifest.json', imported)
    palette = json.loads((source / 'palette.json').read_text())
    dump(out / 'context/original-palette.json', palette)
    swatch = Image.new('RGB', (768, 768))
    d = ImageDraw.Draw(swatch)
    for i, color in enumerate(palette):
        x, y = (i % 16) * 48, (i // 16) * 48
        d.rectangle((x, y, x + 47, y + 47), fill=tuple(color))
        d.text((x + 3, y + 3), str(i), font=font(12), fill='white' if sum(color) < 384 else 'black')
    swatch.save(out / 'reference/original-palette.png')

    scene_cards = []; placements = {}; sound_events = []; frame_count = 0; sample_count = 0
    sound_by_id = {s['sfx_number']: s for s in sounds}
    for scene in scenes:
        stem = scene['scene']
        shutil.copy2(source / 'videos' / f'{stem}.mp4', out / 'reference/previews' / f'{stem}.mp4')
        shutil.copy2(source / 'timelines' / f'{stem}.jsonl', out / 'context/timelines' / f'{stem}.jsonl')
        shutil.copy2(source / 'raw-scripts' / f'{stem}.TTM.jsonl', out / 'context/raw-scripts' / f'{stem}.TTM.jsonl')
        shot_numbers = {s['frame'] for s in scene['stills']}; shot_rows = {}
        for line in (source / 'timelines' / f'{stem}.jsonl').open():
            row = json.loads(line); frame_count += 1
            if row['frame'] in shot_numbers:
                shot_rows[row['frame']] = row
            for number in row['sfx_requests']:
                sound_events.append({'scene': stem, 'frame': row['frame'], 'tick': row['tick'], 'time_seconds': row['time_seconds'],
                                     'sfx_number': number, 'wav': sound_by_id.get(number, {}).get('file', 'UNMAPPED')})
            for draw in row['draws']:
                key = (row['background'], draw['sheet'], draw['frame'], draw['x'], draw['y'], draw['width'], draw['height'],
                       draw['flip_x'], draw['flip_y'], tuple(draw['clip_xyxy_exclusive']))
                if key not in placements:
                    placements[key] = {'background': row['background'], **draw, 'scenes': {}, 'occurrences': 0}
                    placements[key].pop('order')
                placement = placements[key]; placement['occurrences'] += 1
                usage = placement['scenes'].setdefault(stem, {'first_frame': row['frame'], 'last_frame': row['frame']})
                usage['last_frame'] = row['frame']
        board = Image.new('RGB', (960, 4 * 308), '#101923'); bd = ImageDraw.Draw(board)
        best = max(shot_rows.values(), key=lambda r: len(r['draws']))
        for i, shot in enumerate(scene['stills']):
            with Image.open(source / 'stills' / shot['file']) as image:
                image = image.convert('RGB')
            filename = Path(shot['file']).with_suffix('.png').name
            image.save(out / 'reference/stills' / filename)
            x, y = (i % 2) * 480, (i // 2) * 308
            board.paste(image.resize((480, 270), Image.Resampling.LANCZOS), (x, y))
            bd.text((x + 8, y + 276), f"{stem} · frame {shot['frame']} · {shot['tick']/60:.2f}s", font=font(16), fill='white')
            if shot['frame'] == best['frame']:
                annotated = image.copy(); ad = ImageDraw.Draw(annotated)
                sw, sh = best['logical_size']; scale = min(960 / sw, 540 / sh)
                ox, oy = (960 - sw * scale) / 2, (540 - sh * scale) / 2
                colors = ['#ffbf47', '#82e9ff', '#ff8fb9', '#91ed90', '#c5a7ff', '#ff9c71', '#a6d9ff', '#e2e885']
                for order, dr in enumerate(best['draws'][:16]):
                    x0, y0 = ox + dr['x'] * scale, oy + dr['y'] * scale
                    ad.rectangle((x0, y0, x0 + dr['width'] * scale, y0 + dr['height'] * scale), outline=colors[order % len(colors)], width=2)
                # Keep names out of the artwork: many water-effect rectangles overlap.
                legend_rows = math.ceil(min(16, len(best['draws'])) / 2)
                guide = Image.new('RGB', (960, 540 + 70 + max(1, legend_rows) * 30), '#101923')
                guide.paste(annotated, (0, 0)); gd = ImageDraw.Draw(guide)
                gd.text((12, 550), f"{stem} | frame {best['frame']} | draw order / native x,y / native size", font=font(18), fill='white')
                for order, dr in enumerate(best['draws'][:16]):
                    xx, yy = 12 + (order % 2) * 480, 580 + (order // 2) * 30
                    gd.rectangle((xx, yy+3, xx+12, yy+16), fill=colors[order % len(colors)])
                    gd.text((xx+20, yy), f"{order}: {dr['sheet']} #{dr['frame']}  ({dr['x']},{dr['y']})  {dr['width']}x{dr['height']}", font=font(14), fill='white')
                gd.text((12, guide.height-22), 'Executed draw extents, not collision masks. Saved layers may persist. Complete list in the paired JSON.', font=font(13), fill='#aabbd0')
                guide.save(out / 'reference/placement-guides' / f'{stem}.png')
                dump(out / 'reference/placement-guides' / f'{stem}.json', best)
            shot['file'] = f'reference/stills/{filename}'
        board.save(out / 'reference/storyboards' / f'{stem}.jpg', quality=90)
        scene_cards.append(f'<article><h3>{stem}</h3><video controls preload="none" poster="{scene["stills"][0]["file"]}" src="reference/previews/{stem}.mp4"></video><p>{scene["frames"]} ticks · {scene["duration_seconds"]:.2f}s · silent 20 FPS reference</p><p><a href="reference/storyboards/{stem}.jpg">8-frame storyboard</a> · <a href="reference/placement-guides/{stem}.png">Placement guide</a> · <a href="context/timelines/{stem}.jsonl">Every tick / draw / clip / transition</a></p></article>')
    dump(out / 'context/scene-inventory.json', scenes)
    dump(out / 'context/placement-index.json', {'note': 'Executed draw positions, not collision or semantic contact geometry. Per-tick order is in timelines. Saved layers are visible in stills/videos.', 'placements': list(placements.values())})
    write_csv(out / 'audio/events.csv', sound_events, ['scene', 'frame', 'tick', 'time_seconds', 'sfx_number', 'wav'])
    audio_cards = []
    for sound in sounds:
        shutil.copy2(source / 'audio' / sound['file'], out / 'audio/original' / sound['file'])
        with wave.open(str(out / 'audio/original' / sound['file'])) as wav:
            assert (wav.getnchannels(), wav.getsampwidth(), wav.getframerate(), wav.getnframes()) == (1, 1, 11025, sound['samples'])
            pcm = wav.readframes(wav.getnframes()); sound['pcm_sha256'] = hashlib.sha256(pcm).hexdigest()
            sample_count += len(pcm)
        audio_cards.append(f'<article><h3>{sound["file"]}</h3><p>SFX {sound["sfx_number"]} · resource {sound["resource_id"]} · {sound["duration_seconds"]:.3f}s</p><audio controls preload="none" src="audio/original/{sound["file"]}"></audio></article>')
    dump(out / 'audio/manifest.json', sounds)
    for filename in ('animation.rs', 'scheduler.rs', 'hd.rs', 'remaster.rs', 'audio.rs', 'resources.rs', 'effects.rs', 'timing.rs', 'main.rs'):
        shutil.copy2(ROOT / 'src' / filename, out / 'context/runtime-source' / filename)
    for filename in ('scripts.md', 'executable.md', 'rendering-review.md', 'gemini-import.json'):
        shutil.copy2(ROOT / 'evidence' / filename, out / 'context/findings' / filename)
    shutil.copy2(ROOT / 'evidence/sprite-review/REVIEW.md', out / 'context/findings/sprite-review.md')
    for filename in ('pack_image.py', 'unpack_sprite_tilesets.py', 'validate_remaster_edits.py'):
        shutil.copy2(ROOT / 'tools' / filename, out / 'tools' / filename)
    # The existing sprite importer expects these paths at its pack root.
    dump(out / 'context/sprite-tilemap-layout.json', json.loads((ROOT / 'exports/sprite-tilemaps/manifest.json').read_text()))
    manifest['counts'] = {'backgrounds': len(backgrounds), 'widescreen_templates': len(backgrounds) * len(RATIOS),
                          'sprite_style_atlases': len(imported['pages']), 'scene_previews': len(scenes),
                          'scene_ticks': frame_count, 'reference_stills': sum(len(s['stills']) for s in scenes),
                          'audio_files': len(sounds), 'audio_samples': sample_count, 'unique_scripted_placements': len(placements)}
    dump(out / 'manifest.json', manifest)
    readme = '''# Screen Antics: Gemini remaster handoff

Start with `GEMINI_PROMPT.txt` and open `index.html` to browse locally. This pack
contains technical exports and references, not newly generated artwork.

## What to work on

1. Redraw the 10 PNGs in `editing/backgrounds/`. These are clean 4x nearest-neighbor
   originals. Retain their dimensions, composition, horizon, tree/ground geometry,
   contact locations, framing and all intentional black areas. Use full RGB color.
2. Match the existing character/effect style in `style/current-sprite-atlases/`.
   All 116 imported atlases are included unchanged. Eight original scenery-related
   sheets in `reference/original-paired-atlases/` explain the old geometry.
3. After finishing each core background, paste that exact finished PNG into the
   corresponding widescreen stage rectangle. Paint only the exterior extension.
   Return complete opaque widescreen PNGs. The 30 templates cover exact 16:9, 21:9,
   and 43:18 (the ratio of 3440x1440). Read canvas dimensions and stage offsets from
   `manifest.json`; do not resize, crop, recenter, stretch or move the active stage.
   Black areas inside the stage are intentional source content, not empty masks.
4. If a background patch sprite needs matching colors, return an edited atlas with
   the same name, layout, size and alpha conventions. Do not change character poses.
5. Optionally remaster the 23 WAV effects, preserving identity, onset and timing.

`guides/` contains annotations for reference only. Do not paint labels or grids
into final images. Widescreen templates have a real transparent extension;
`masks/widescreen/` uses white=paint extension, black=retain registered core.
Never bake a checkerboard into the image. All source background pixels are opaque,
including black. The source palette is a reference, not a 256-color restriction.

## How to use the context

- `reference/previews/*.mp4`: every complete bounded scene-browser timeline,
  sampled at 20 FPS using the imported sprites. Silent. Display-only fades/wipes
  and their extra wall time are omitted; their events are recorded in metadata.
- `reference/storyboards/`: eight frames per scene, for a quick visual review.
- `reference/stills/`: full 960x540 checkpoints with current composition.
- `reference/placement-guides/`: draw rectangles plus their exact metadata.
- `context/timelines/`: every compiled tick, native coordinates, executed sprite
  draws, mirroring, exclusive clipping rectangles, sounds, transitions and commands.
  Draws are in execution order. Captured/stored artwork can survive later ticks
  without another draw; use the rendered references for the final visible result.
- `context/placement-index.json`: deduplicated placements grouped by background,
  with scene/frame usage. These are not collision masks or named contact points.
- `context/raw-scripts/`: recovered raw TTM commands, distinct from scheduled ticks.
- `context/sprite-manifests/`: exact atlas frame rectangles and registration.
  `native_file` entries describe the earlier original sprite export; those separate
  native PNGs are not duplicated here. Current complete atlases are under `style/`.
- `audio/events.csv`: observed sound triggers in the compiled preview timelines.
  WAVESFX suffixes differ from numeric executable resource IDs; keep supplied names.
- `context/runtime-source/` and `context/findings/`: implementation and known limits.

These previews describe the current Rust reconstruction, not proven original
Windows behavior. Scene selection is deterministic and bounded; several timelines
stop at 2400 ticks, some raw scripts use bounded local loops, random choices are
seeded and random delays use their midpoint. Global itinerary/day/night/holiday
logic is not fully verified. Original raw background resources may appear less
often than composited scenery sprites. Do not infer that an unused backdrop is junk.

Important paired artwork: BACKGRND shoreline/tree fragments; TRUNK trunk/crown;
MJDIVE frames 7,8,24,25,26 tree fragments; MJBOTTLE frames 0,1,2 water patches;
THNKBUBL and STNDLAY composed mini-scenes. Frame numbers here are zero-based.
These are source artwork, not extraction damage. Preserve opaque black outlines,
tiny detached highlights, spray, props and intentionally empty frames.

## Return layout

```
edited/
  backgrounds/ISLETEMP.SCR.png       # all 10; exact 4x sizes in manifest.json
  widescreen/16_9/ISLETEMP.SCR.png    # complete opaque canvas, exact pasted core
  widescreen/21_9/ISLETEMP.SCR.png
  widescreen/43_18/ISLETEMP.SCR.png
  sprite_tilemaps/BACKGRND.png       # only if scenery patch matching needs changes
  audio/WAVESFX01.wav               # optional; 16-bit PCM, mono, 44100 or 48000 Hz
  notes.txt                        # changes, consistency decisions and exceptions
```

Other files use their exact source names from `manifest.json`. No JPEG final art.
Audio semantic names are deliberately not guessed. Preserve sample onset and
approximately original duration; document any deliberate timing change in notes.
The still/video references contain actors and effects: never flatten them into
the static background. Palette indices are not transparency keys for RGB art.

Validate the return folder:

```
python tools/validate_remaster_edits.py --pack . --edited /path/to/edited
```

Use `--allow-partial` while working on a subset. Validation checks dimensions,
opacity, wide/core registration, optional sprite layouts/gutters/empty frames,
and audio format. It cannot judge visual consistency or detect painted-in labels.

The native renderer already accepts standard background SAHD replacements through
`--asset-pack`. `tools/pack_image.py input.png output.rgba` converts them losslessly.
The new widescreen canvases need manifest-aware renderer integration; they are not
currently used by the edge-extension renderer. New WAVs similarly need an external
audio loading/resampling path. Nothing in this handoff installs or enables edits.

Original inputs, the imported sprites and the running application are unchanged.
The inventory in `SHA256SUMS` covers every included file except itself.
'''
    (out / 'README.md').write_text(readme)
    (out / 'GEMINI_PROMPT.txt').write_text('''You are remastering the artwork of Screen Antics / Johnny Castaway for an existing native Rust player.
Read README.md and manifest.json before editing. Open index.html for the complete reference gallery.

Your main deliverables are the 10 full-color, high-resolution background PNGs and 30 widescreen variants.
Match the already remastered sprite style in style/current-sprite-atlases/. Preserve the original cartoon identity,
while replacing the old palette/dithering with coherent modern color, shading and clean contours.

Work from editing/backgrounds/ for the source composition. Do not use screenshots as base artwork: those
contain separately animated actors, shadows, water effects and props that must remain separate.
Preserve exact canvas dimensions and geometry: tree position/width/crown, ground/contact surfaces, shoreline,
horizon, intentional black regions and embedded lettering/layout. Do not add a static Johnny, clouds or props
where the original backdrop lacks them. These can be supplied by separate sprite draws.

Check the paired scenery sprites in BACKGRND, TRUNK, MJDIVE, MJBOTTLE, THNKBUBL and STNDLAY.
Background and animated fragments must share the same material colors and registration. If necessary,
return matching edited sprite_tilemaps as well, retaining frame order, poses, exact layout, real alpha,
opaque black, tiny disconnected effects and empty frames. Do not repaint the other characters unnecessarily.

Finish each core background first. Paste that exact finished core into its stage_rect_xywh in every
widescreen template, then outpaint only the surrounding region. The core must be byte-identical across
variants. Final full-background and widescreen images must be opaque RGB/RGBA PNGs. Use the masks;
never bake guides, frame labels, a transparency checkerboard or black replacement bars into final art.
For interior/title scenes, preserve intentional interior black and respect the original framing; outer
extensions may use restrained scene-appropriate treatment instead of inventing more active stage.

Use videos, storyboards, native coordinates, per-tick clipping and draw ordering to understand the interactions.
Treat these as current-port reference behavior with documented limitations, not proof of original Windows logic.
Do not move interactions or redesign the timing to compensate for artwork alignment mistakes.

Optionally remaster audio/original/*.wav into 16-bit mono PCM at 44100 or 48000 Hz, retaining exact filenames,
effect identity, onset and approximate duration. Listen before assigning meanings. Do not create background
music or insert long silence. Document any duration changes.

Return the edited/ directory layout in README.md, run the included validator, and report any exceptions
in notes.txt. Do not alter runtime source or install the assets as part of this artwork pass.
''')
    style = '''body{background:#101923;color:#e7edf5;font:16px system-ui;margin:0 auto;padding:28px;max-width:1500px}a{color:#7edbff;margin-right:8px}h1{font-size:32px}h2{margin-top:40px}.grid{display:grid;grid-template-columns:repeat(auto-fit,minmax(330px,1fr));gap:18px}article{background:#1c2939;border:1px solid #3c5068;border-radius:12px;padding:16px}img,video{width:100%;height:auto;max-height:300px;object-fit:contain}audio{width:100%}p{line-height:1.5}.notice{padding:18px;background:#253b50;border-radius:10px}input{font:inherit;padding:10px;background:#24374c;color:white;border:1px solid #567;width:min(600px,90%)}'''
    document = f'''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Screen Antics — Gemini remaster handoff</title><style>{style}</style><h1>Screen Antics · remaster handoff</h1><p>10 backgrounds · 30 widescreen templates · 116 sprite style atlases · 41 animation previews · 23 sounds</p><p><a href="GEMINI_PROMPT.txt">Start: Gemini prompt</a><a href="README.md">Instructions</a><a href="manifest.json">Exact layouts</a><a href="context/placement-index.json">Placement index</a><a href="style/current-sprite-atlases/">Current sprite style</a></p><p class="notice">Edit clean PNGs in editing/. Guides and previews are reference only. Videos show the current Rust port with the new sprites and old scenery; silent, sampled at 20 FPS, display transitions omitted. Backgrounds and widescreen extensions must keep the registered stage fixed.</p><input id="search" placeholder="Filter backgrounds, scenes or sounds" aria-label="Filter assets"><h2>Backgrounds</h2><div class="grid">{''.join(bg_cards)}</div><h2>Animation and placement references</h2><div class="grid">{''.join(scene_cards)}</div><h2>Original sound effects</h2><p>Names preserve executable mapping. <a href="audio/events.csv">Scene trigger times</a><a href="audio/manifest.json">Audio metadata</a></p><div class="grid">{''.join(audio_cards)}</div><script>document.getElementById('search').addEventListener('input',e=>{{let q=e.target.value.toLowerCase();document.querySelectorAll('article').forEach(c=>c.hidden=!c.textContent.toLowerCase().includes(q))}})</script></html>'''
    (out / 'index.html').write_text(document)

    # Check final on-disk templates, original colors and videos before archiving.
    for bg in manifest['backgrounds']:
        with Image.open(out / bg['native_file']) as native, Image.open(out / bg['editing_file']) as core:
            assert core.size == tuple(bg['runtime_size']) and core.mode == 'RGB'
            assert core.tobytes() == native.resize(core.size, Image.Resampling.NEAREST).tobytes()
            for wide in bg['widescreen']:
                with Image.open(out / wide['editing_file']) as image, Image.open(out / wide['mask_file']) as mask:
                    assert image.size == mask.size == tuple(wide['canvas_size'])
                    x, y, w, h = wide['stage_rect_xywh']
                    assert image.crop((x,y,x+w,y+h)).tobytes() == core.convert('RGBA').tobytes()
                    assert Image.eval(image.getchannel('A'), lambda a: 255-a).tobytes() == mask.tobytes()
    video_checks = []
    for scene in scenes:
        path = out / 'reference/previews' / f'{scene["scene"]}.mp4'
        probe = json.loads(subprocess.check_output(['ffprobe','-v','error','-select_streams','v:0','-show_entries','stream=width,height,nb_frames,avg_frame_rate','-of','json',str(path)]))['streams'][0]
        assert (probe['width'], probe['height'], probe['avg_frame_rate'], int(probe['nb_frames'])) == (640,360,'20/1',scene['video_frames'])
        video_checks.append({'scene':scene['scene'], **probe})
    dump(out / 'validation.json', {'counts': manifest['counts'], 'background_pixels_exact': True, 'widescreen_core_pixels_exact': True,
                                   'widescreen_masks_exact': True, 'sprite_atlas_hashes_match_import': True,
                                   'audio_format_and_sample_counts_verified': True, 'video_streams': video_checks})
    files = sorted(p for p in out.rglob('*') if p.is_file())
    (out / 'SHA256SUMS').write_text(''.join(f'{sha(p)}  {p.relative_to(out).as_posix()}\n' for p in files))
    args.archive.parent.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(args.archive, 'x', compression=zipfile.ZIP_DEFLATED, compresslevel=6) as archive:
        for path in sorted(out.rglob('*')):
            if path.is_file():
                archive.write(path, Path(out.name) / path.relative_to(out))
    with zipfile.ZipFile(args.archive) as archive:
        assert archive.testzip() is None
    print(json.dumps({'output': str(out), 'archive': str(args.archive), **manifest['counts'], 'archive_bytes': args.archive.stat().st_size}, indent=2))


if __name__ == '__main__':
    main()
