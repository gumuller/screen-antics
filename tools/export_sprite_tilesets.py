#!/usr/bin/env python3
"""Losslessly pack recovered sprites for external image editing; no repainting.
Requires Pillow and the SAHD exports produced by tools/audit_sprites.rs.
"""
from __future__ import annotations
import argparse
import csv
import hashlib
import html
import json
import math
import shutil
import struct
import zipfile
from collections import defaultdict
from pathlib import Path
from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parents[1]
SCALE = 4
PADDING = 16
LABEL_HEIGHT = 32
MAX_EDGE = 2048


def digest(data):
    return hashlib.sha256(data).hexdigest()


def read_frame(path):
    data = path.read_bytes()
    if len(data) < 12 or data[:4] != b'SAHD':
        raise ValueError(f'{path}: invalid SAHD header')
    w, h = struct.unpack('<II', data[4:12])
    if not w or not h or len(data) != 12 + w * h * 4:
        raise ValueError(f'{path}: invalid dimensions or RGBA data')
    return Image.frombytes('RGBA', (w, h), data[12:])


def font():
    for path in ('/usr/share/fonts/liberation/LiberationSans-Regular.ttf',
                 '/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf'):
        if Path(path).exists():
            return ImageFont.truetype(path, 14)
    return ImageFont.load_default()


def checkerboard(size):
    image = Image.new('RGBA', size, (39, 43, 50, 255))
    draw = ImageDraw.Draw(image)
    for y in range(0, size[1], 16):
        for x in range(0, size[0], 16):
            if (x // 16 + y // 16) % 2:
                draw.rectangle((x, y, x + 15, y + 15), fill=(49, 54, 62, 255))
    return image


def write_json(path, data):
    path.write_text(json.dumps(data, indent=2) + '\n')


def grid_layout(count, cellw, cellh, layout):
    if layout == 'atlas':
        # One complete source set per image. Minimize the longest canvas edge
        # without changing sprite scale or frame order; then minimize spare cells.
        cols = min(range(1, count + 1), key=lambda c: (
            max(c * cellw, math.ceil(count / c) * cellh),
            c * math.ceil(count / c),
            abs(c * cellw - math.ceil(count / c) * cellh)))
        return cols, math.ceil(count / cols)
    cols = min(4, MAX_EDGE // cellw, count)
    rows = min(4, MAX_EDGE // cellh, math.ceil(count / max(1, cols)))
    return cols, rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source', type=Path, default=ROOT / 'exports/.sprite-source')
    parser.add_argument('--layout', choices=('pages', 'atlas'), default='pages',
                        help='Small paginated sheets, or one complete tilemap per source set.')
    parser.add_argument('--output', type=Path)
    parser.add_argument('--archive', type=Path)
    args = parser.parse_args()
    atlas = args.layout == 'atlas'
    pack_name = 'sprite-tilemaps' if atlas else 'sprite-tilesets'
    args.output = args.output or ROOT / 'exports' / pack_name
    args.archive = args.archive or ROOT / 'exports' / f'screen-antics-{pack_name}.zip'
    if args.output.exists() and any(args.output.iterdir()):
        parser.error('Output folder must be empty; choose a new --output to preserve prior work.')
    if args.archive.exists():
        parser.error('Archive already exists; choose a new --archive to preserve prior work.')
    args.output.mkdir(parents=True, exist_ok=True)
    for part in ('editing', 'reference', 'native', 'manifests'):
        (args.output / part).mkdir()
    if atlas:
        (args.output / 'preview').mkdir()
    groups = defaultdict(list)
    with (args.source / 'frames.tsv').open() as f:
        for row in csv.DictReader(f, delimiter='\t'):
            row['index'] = int(row['index'])
            groups[row['sheet']].append(row)
    report = {
        'format': 'screen-antics-sprite-tilesets-v1', 'scale': SCALE, 'layout': args.layout,
        'source': 'Original decoded RESOURCE.001 sprites; no generated replacements or recoloring',
        'source_resource_sha256': digest((ROOT / 'original/RESOURCE.001').read_bytes()),
        'coordinate_convention': 'Zero-based frames; rects are x,y,width,height; origins are top-left; RGBA straight alpha',
        'sheets': [],
    }
    frame_inventory = []
    cards = []
    total_frames = total_pages = empty_frames = verified = largest_edge = 0
    typeface = font()
    for name, entries in sorted(groups.items()):
        entries.sort(key=lambda r: r['index'])
        if [r['index'] for r in entries] != list(range(len(entries))):
            raise ValueError(f'{name}: frame indices are not consecutive')
        frames = [read_frame(args.source / row['file']) for row in entries]
        stem = Path(name).stem
        maxw = max(im.width for im in frames)
        maxh = max(im.height for im in frames)
        cellw = math.ceil(max(192, maxw * SCALE + 2 * PADDING) / 8) * 8
        cellh = math.ceil(max(128, maxh * SCALE + 2 * PADDING + LABEL_HEIGHT) / 8) * 8
        cols, rows = grid_layout(len(frames), cellw, cellh, args.layout)
        if cols < 1 or rows < 1:
            raise ValueError(f'{name}: frame cannot fit a {MAX_EDGE}px editing sheet at {SCALE}x')
        capacity = cols * rows
        size = (cols * cellw, rows * cellh)
        largest_edge = max(largest_edge, *size)
        sheet_meta = {
            'source_sheet': name, 'frame_count': len(frames), 'scale': SCALE,
            'grid_columns': cols, 'grid_rows': rows, 'cell_size': [cellw, cellh],
            'padding': PADDING, 'label_band': LABEL_HEIGHT, 'canvas_size': list(size),
            'layout': args.layout, 'pages': [],
        }
        (args.output / 'native' / stem).mkdir()
        for start in range(0, len(frames), capacity):
            page_number = start // capacity + 1
            basename = f'{stem}.png' if atlas else f'{stem}__p{page_number:03}.png'
            clean = Image.new('RGBA', size, (0, 0, 0, 0))
            page = {
                'page': page_number, 'editing_file': 'editing/' + basename,
                'reference_file': 'reference/' + basename,
                'canvas_size': list(size), 'frames': [], 'unused_cells': [],
            }
            for slot in range(capacity):
                index = start + slot
                cx, cy = slot % cols * cellw, slot // cols * cellh
                if index >= len(frames):
                    page['unused_cells'].append([slot % cols, slot // cols])
                    continue
                im = frames[index]
                native_file = f'native/{stem}/{index:03}.png'
                im.save(args.output / native_file)
                enlarged = im.resize((im.width * SCALE, im.height * SCALE), Image.Resampling.NEAREST)
                x, y = cx + PADDING, cy + LABEL_HEIGHT + PADDING
                clean.paste(enlarged, (x, y))
                bbox = im.getchannel('A').getbbox()
                frame = {
                    'index': index, 'cell': [slot % cols, slot // cols],
                    'source_size': list(im.size), 'source_opaque_bbox_xyxy': list(bbox) if bbox else None,
                    'empty': bbox is None, 'native_file': native_file,
                    'source_rgba_sha256': digest(im.tobytes()),
                    'rect': [x, y, enlarged.width, enlarged.height],
                    'runtime_size': list(enlarged.size),
                    'runtime_file': f'{name}/{index:03}.rgba',
                }
                page['frames'].append(frame)
                frame_inventory.append({'sheet': name, 'frame': index,
                    'editing_file': page['editing_file'], 'native_file': native_file,
                    'x': x, 'y': y, 'width': enlarged.width, 'height': enlarged.height,
                    'source_width': im.width, 'source_height': im.height})
                empty_frames += int(bbox is None)
            clean.save(args.output / page['editing_file'])
            # Verify the actual saved PNG, not just the packing buffer.
            with Image.open(args.output / page['editing_file']) as saved:
                for frame in page['frames']:
                    x, y, w, h = frame['rect']
                    actual = saved.crop((x, y, x + w, y + h))
                    expected = frames[frame['index']].resize((w, h), Image.Resampling.NEAREST)
                    if actual.tobytes() != expected.tobytes():
                        raise ValueError(f'{name} #{frame["index"]}: packed RGBA mismatch')
                    with Image.open(args.output / frame['native_file']) as native:
                        if native.tobytes() != frames[frame['index']].tobytes():
                            raise ValueError(f'{name} #{frame["index"]}: native PNG mismatch')
                    verified += 1
            reference = checkerboard(size)
            reference.alpha_composite(clean)
            draw = ImageDraw.Draw(reference)
            for slot in range(capacity):
                cx, cy = slot % cols * cellw, slot // cols * cellh
                draw.rectangle((cx, cy, cx + cellw - 1, cy + cellh - 1), outline='#687588')
                draw.rectangle((cx + 1, cy + 1, cx + cellw - 2, cy + LABEL_HEIGHT - 1), fill='#171c24')
                index = start + slot
                if index < len(frames):
                    im = frames[index]
                    label = f'{stem}  #{index:03}  |  {im.width}x{im.height}'
                    draw.text((cx + 8, cy + 7), label, font=typeface, fill='#f1f5f9')
                    x, y, w, h = page['frames'][slot]['rect']
                    draw.rectangle((x - 1, y - 1, x + w, y + h), outline='#64748b')
                    if page['frames'][slot]['empty']:
                        draw.text((x, y), 'EMPTY SOURCE FRAME', font=typeface, fill='#f4c97b')
                else:
                    draw.text((cx + 8, cy + 7), 'UNUSED CELL', font=typeface, fill='#a0aab8')
            reference.convert('RGB').save(args.output / page['reference_file'])
            if atlas:
                # Lightweight whole-set reference for browsing or model context.
                # Editing masters keep their exact 4x crop coordinates.
                preview = reference.convert('RGB')
                preview.thumbnail((MAX_EDGE, MAX_EDGE), Image.Resampling.NEAREST)
                page['preview_file'] = f'preview/{basename}'
                preview.save(args.output / page['preview_file'])
            sheet_meta['pages'].append(page)
            total_pages += 1
        manifest_file = f'manifests/{stem}.json'
        write_json(args.output / manifest_file, sheet_meta)
        report['sheets'].append({'source_sheet': name, 'frame_count': len(frames),
            'page_count': len(sheet_meta['pages']), 'manifest_file': manifest_file})
        total_frames += len(frames)
        links = ' '.join(f'<a href="{p["editing_file"]}">{"Full tilemap" if atlas else "Edit " + str(p["page"])}</a> '
                        f'<a href="{p["reference_file"]}">{"Labeled reference" if atlas else "Labels " + str(p["page"])}</a>' for p in sheet_meta['pages'])
        cards.append(f'<article data-name="{html.escape(stem.lower())}"><h2>{html.escape(stem)}</h2>'
            f'<p>{len(frames)} frames · {len(sheet_meta["pages"])} pages · {size[0]} × {size[1]}</p>'
            f'<a href="{sheet_meta["pages"][0]["reference_file"]}"><img loading="lazy" '
            f'src="{sheet_meta["pages"][0].get("preview_file", sheet_meta["pages"][0]["reference_file"])}" alt="{html.escape(stem)} labeled sprites"></a>'
            f'<nav>{links}</nav><p><a href="{manifest_file}">Frame manifest</a></p></article>')
    report.update(sheet_count=len(groups), frame_count=total_frames,
                  page_count=total_pages, empty_frame_count=empty_frames)
    write_json(args.output / 'manifest.json', report)
    with (args.output / 'frames.csv').open('w', newline='') as f:
        writer = csv.DictWriter(f, fieldnames=list(frame_inventory[0]))
        writer.writeheader()
        writer.writerows(frame_inventory)
    prompt = '''Use the transparent editing PNG as the exact sprite-sheet layout. Use the labeled
reference PNG only to understand frame IDs and canvas bounds.

Redraw the existing sprites as detailed, colorful 24-bit 2D cartoon game art.
Preserve every source frame's exact pose, facing direction, hand position, gait
phase, clothing, expression and separate prop. The gray jogging tracksuit must
remain distinct from the everyday shirt and shorts. Keep raised hats and book
pages in their individual source positions; do not replace them with a repeated
resting pose. Retain tiny marks, disconnected splashes, shadows and partial bodies.

Keep the exact input canvas dimensions, grid, frame order and per-frame top-left
anchors. All artwork must stay inside its original frame rectangle shown in the
reference. Preserve transparent margins and empty cells. Never crop, recenter,
resize, rearrange, merge or omit a frame. Do not extend a foot, prop or shadow
into the neighboring frame. Do not add a background, ground plane or new props.

Return ONE RGBA PNG with a real transparent alpha channel and the SAME filename
and pixel dimensions as the editing input. Do not draw labels, guides, borders,
checkerboards, text or reference annotations in the output. Black clothing and
outlines are opaque artwork, not transparency.
'''
    if atlas:
        prompt = ('This image is ONE COMPLETE TILEMAP for one original sprite set. '
                  'Frames run left-to-right, then top-to-bottom. Keep all poses '
                  'in this single image and leave unused trailing cells empty.\n\n' + prompt)
    (args.output / 'MODEL_PROMPT.txt').write_text(prompt)
    readme = f'''# Screen Antics sprite tilesets

{len(groups)} original sprite sets, {total_frames:,} frames, {total_pages} editing pages.
These are recovered original sprites, not the failed generated replacements.
No poses have been repainted, recolored, cropped, recentered or component-filtered.

## Start here

Open index.html to browse and search all sprite sets. Start with JOHNWALK,
MJJOG1, MJJOG2 and MJREAD for the castaway's walking, jogging and reading poses.
Other sets contain secondary characters, props, scenery fragments and effects.

For each page, give your model the transparent PNG under editing/. The matching
PNG under reference/ has frame numbers, dimensions, tile guides and a checkerboard
for inspection. Reference images are guides only; edit the clean transparent image.
MODEL_PROMPT.txt contains a reusable instruction template.

- editing/: lossless {SCALE}x nearest-neighbor enlargement, real RGBA transparency,
  at most 16 frames/page and {MAX_EDGE}x{MAX_EDGE} pixels. These are editing templates,
  not newly created high-resolution detail.
- reference/: corresponding labeled review images, with identical canvas size.
- native/: every original frame separately at its exact native resolution.
- manifests/: exact frame IDs, grid positions, source sizes, alpha bounds,
  crop rectangles and intended runtime filenames.
- manifest.json and frames.csv: full inventory.

Every frame starts at its original top-left canvas origin within the tile.
Frames are NOT centered by visible body shape. Smaller frames keep their own
source-sized crop rectangle within a common tile. Source bitmap origins matter
because the animation commands position each frame independently.

## Returning edited pages

Keep each editing PNG's filename, pixel dimensions and alpha channel. Place edited
pages in a separate folder; keep this source pack intact. Return the clean PNGs,
not labeled reference copies. It is fine to edit one page or one set at a time.

The included unpack_sprite_tilesets.py validates page sizes, RGBA transparency
and empty gutters, then extracts frames using the manifest. It writes PNG and
SAHD runtime files to a NEW output directory; it does not install or enable them.
With Python and Pillow installed, from this pack directory:

    python unpack_sprite_tilesets.py --pack . --edited-dir /path/to/edited-pages --output /path/to/new-frame-pack

The animation player expects source dimensions multiplied by {SCALE}; this matches
these editing sheets exactly. AI tools sometimes change canvas size or spacing;
the unpacker rejects dimension changes rather than stretching the results.
Pose accuracy still needs visual review even if dimensions pass.

## Verification and provenance

All {verified:,} frames were reopened from saved sheets and compared byte-for-byte
against the original RGBA enlarged by exact nearest-neighbor {SCALE}x. Every native
PNG was also verified. All {empty_frames} fully transparent source frames and every
nonempty tiny/multipart frame were retained. Source index zero alone defines
transparency; the other black colors remain opaque. See validation.json.

The source resource hash and pixel-coordinate convention are in manifest.json.
The PNG export is a technical packing of the user's supplied original game data.
'''
    if atlas:
        readme = readme.replace('# Screen Antics sprite tilesets', '# Screen Antics complete sprite tilemaps')
        readme = readme.replace(f'{total_pages} editing pages.', f'{total_pages} complete tilemaps: exactly ONE PNG per original sprite set.')
        readme = readme.replace(f'  at most 16 frames/page and {MAX_EDGE}x{MAX_EDGE} pixels. These are editing templates,',
            f'  every frame from a set in ONE image, in left-to-right / top-to-bottom order.\n'
            f'  Grid dimensions differ between sets. The largest master edge is {largest_edge}px.\n'
            f'  These are editing templates,')
        readme = readme.replace('- native/:',
            f'- preview/: smaller labeled whole-set references, capped at {MAX_EDGE}px.\n'
            '  Use them for browsing/context; editing/ contains the full-size templates.\n- native/:')
    (args.output / 'README.md').write_text(readme)
    page_html = '''<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Screen Antics sprite tilesets</title>
<style>body{font:16px system-ui;background:#10151e;color:#e4eaf2;margin:32px}h1{margin-bottom:8px}a{color:#8ed3ff}input{font:inherit;padding:12px;width:min(440px,90%);background:#202a38;color:white;border:1px solid #637185;border-radius:8px;margin:16px 0}main{display:grid;grid-template-columns:repeat(auto-fill,minmax(300px,1fr));gap:20px}article{padding:16px;background:#1b2431;border-radius:12px}article[hidden]{display:none}h2{font-size:20px;margin:0}p{color:#b6c3d6}img{width:100%;height:230px;object-fit:contain;background:#111820}nav{display:flex;flex-wrap:wrap;gap:10px;margin-top:12px}</style>
'''
    page_html += f'<h1>Screen Antics {"complete tilemaps" if atlas else "sprite tilesets"}</h1><p>{len(groups)} sets · {total_frames:,} frames · {total_pages} {"complete tilemaps" if atlas else "pages"} · Original artwork</p>'
    page_html += '<p><a href="README.md">Instructions</a> · <a href="MODEL_PROMPT.txt">Model prompt</a> · <a href="frames.csv">Frame inventory</a></p><input id="search" placeholder="Find a set, e.g. JOHNWALK or MJREAD" aria-label="Filter sprite sets"><main>'
    page_html += ''.join(cards) + '</main><script>document.getElementById("search").addEventListener("input",e=>{for(const card of document.querySelectorAll("article"))card.hidden=!card.dataset.name.includes(e.target.value.toLowerCase())})</script></html>'
    (args.output / 'index.html').write_text(page_html)
    shutil.copy2(ROOT / 'tools/unpack_sprite_tilesets.py', args.output / 'unpack_sprite_tilesets.py')
    validation = {'source_sheets': len(groups), 'source_frames': total_frames,
        'editing_pages': total_pages, 'packed_frame_rgba_checks_passed': verified,
        'native_frame_rgba_checks_passed': verified, 'empty_frames_preserved': empty_frames,
        'maximum_canvas_edge': largest_edge, 'nearest_neighbor_scale': SCALE,
        'layout': args.layout, 'one_image_per_set': atlas,
        'missing_frames': 0, 'repainted_frames': 0}
    write_json(args.output / 'validation.json', validation)
    args.archive.parent.mkdir(parents=True, exist_ok=True)
    if args.archive.exists():
        raise FileExistsError(f'{args.archive} already exists; choose a new --archive')
    with zipfile.ZipFile(args.archive, 'w', compression=zipfile.ZIP_DEFLATED, compresslevel=6) as z:
        for path in sorted(args.output.rglob('*')):
            if path.is_file():
                z.write(path, Path(args.output.name) / path.relative_to(args.output))
    print(json.dumps(validation, indent=2))
    print(f'Folder: {args.output}\nZIP: {args.archive} ({args.archive.stat().st_size / 1024 / 1024:.1f} MiB)')


if __name__ == '__main__':
    main()
