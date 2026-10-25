#!/usr/bin/env python3
"""Extract externally edited PNG tilesets to a separate frame pack. Never installs art."""
import argparse
import hashlib
import json
import struct
from pathlib import Path
from PIL import Image, ImageChops, ImageDraw


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pack', type=Path, required=True)
    parser.add_argument('--edited-dir', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    if args.output.exists():
        parser.error('Choose a new output directory; existing work will not be overwritten.')
    inventory = json.loads((args.pack / 'manifest.json').read_text())
    if inventory.get('format') != 'screen-antics-sprite-tilesets-v1':
        parser.error('Unsupported pack manifest.')
    jobs = []
    known = set()
    for record in inventory['sheets']:
        sheet = json.loads((args.pack / record['manifest_file']).read_text())
        for page in sheet['pages']:
            basename = Path(page['editing_file']).name
            known.add(basename)
            path = args.edited_dir / basename
            if not path.exists():
                continue
            with Image.open(path) as opened:
                if opened.size != tuple(page['canvas_size']):
                    parser.error(f'{basename}: expected {page["canvas_size"]}, got {opened.size}; preserve the original sheet dimensions.')
                if 'A' not in opened.getbands() and 'transparency' not in opened.info:
                    parser.error(f'{basename}: a real transparent alpha channel is required; reference images cannot be imported.')
                im = opened.convert('RGBA')
            allowed = Image.new('L', im.size, 0)
            mask = ImageDraw.Draw(allowed)
            for frame in page['frames']:
                x, y, w, h = frame['rect']
                if not frame['empty']:
                    mask.rectangle((x, y, x + w - 1, y + h - 1), fill=255)
            stray = ImageChops.multiply(im.getchannel('A'), ImageChops.invert(allowed))
            if stray.getbbox():
                parser.error(f'{basename}: opaque pixels outside the original frame rectangles (labels, backdrop, moved sprites or nonempty blank cells).')
            for frame in page['frames']:
                x, y, w, h = frame['rect']
                crop = im.crop((x, y, x + w, y + h))
                if not frame['empty'] and crop.getchannel('A').getbbox() is None:
                    parser.error(f'{basename}: source frame {frame["index"]} became empty.')
            jobs.append((path, record['source_sheet'], page))
    unknown = sorted(p.name for p in args.edited_dir.glob('*.png') if p.name not in known)
    if unknown:
        parser.error('Unrecognized page filenames: ' + ', '.join(unknown[:8]))
    if not jobs:
        parser.error('No matching edited page PNGs found directly inside --edited-dir.')
    # Validate ALL requested sheets before creating output or writing any frames.
    args.output.mkdir(parents=True)
    report = {'format': 'screen-antics-edited-frame-pack-v1', 'pages': [], 'frames': 0}
    for path, sheet, page in jobs:
        with Image.open(path) as opened:
            im = opened.convert('RGBA')
        report['pages'].append({'source': path.name,
            'png_sha256': hashlib.sha256(path.read_bytes()).hexdigest(),
            'source_sheet': sheet, 'frames': [f['index'] for f in page['frames']]})
        destination = args.output / sheet
        destination.mkdir(exist_ok=True)
        for frame in page['frames']:
            x, y, w, h = frame['rect']
            crop = im.crop((x, y, x + w, y + h))
            crop.save(destination / f'{frame["index"]:03}.png')
            (destination / f'{frame["index"]:03}.rgba').write_bytes(b'SAHD' + struct.pack('<II', w, h) + crop.tobytes())
            report['frames'] += 1
    (args.output / 'import-manifest.json').write_text(json.dumps(report, indent=2) + '\n')
    print(f'Extracted {report["frames"]} frames from {len(jobs)} pages to {args.output}. No production assets changed.')


if __name__ == '__main__':
    main()
