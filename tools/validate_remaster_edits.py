#!/usr/bin/env python3
"""Validate a Gemini handoff return folder without installing or changing assets."""
import argparse
import json
from pathlib import Path
import sys
import wave
from PIL import Image, ImageChops, ImageDraw


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pack', type=Path, required=True)
    parser.add_argument('--edited', type=Path, required=True)
    parser.add_argument('--allow-partial', action='store_true', help='Permit missing core backgrounds during iteration')
    args = parser.parse_args()
    manifest = json.loads((args.pack / 'manifest.json').read_text())
    errors, warnings = [], []
    checked = {'backgrounds': 0, 'widescreen': 0, 'sprite_tilemaps': 0, 'audio': 0}
    if not args.edited.is_dir():
        parser.error(f'No edited directory: {args.edited}')

    def image(path, size, opaque=False):
        try:
            with Image.open(path) as source:
                if source.format != 'PNG' or source.mode not in ('RGB', 'RGBA'):
                    raise ValueError('must be an RGB/RGBA PNG, not indexed, grayscale or JPEG')
                if source.size != tuple(size):
                    raise ValueError(f'expected {size[0]}x{size[1]}, got {source.width}x{source.height}')
                result = source.convert('RGBA')
                if opaque and result.getchannel('A').getextrema() != (255, 255):
                    raise ValueError('full backgrounds must be completely opaque')
                return result
        except (OSError, ValueError) as error:
            errors.append(f'{path.relative_to(args.edited)}: {error}')
            return None

    known = set()
    for bg in manifest['backgrounds']:
        rel = f"backgrounds/{bg['name']}.png"; known.add(rel)
        core_path = args.edited / rel
        core = None
        if core_path.exists():
            core = image(core_path, bg['runtime_size'], opaque=True)
            checked['backgrounds'] += core is not None
        elif not args.allow_partial:
            errors.append(f'Missing required {rel}')
        for layout in bg['widescreen']:
            rel = layout['return_file']; known.add(rel)
            path = args.edited / rel
            if not path.exists():
                continue
            wide = image(path, layout['canvas_size'], opaque=True)
            if core is None:
                errors.append(f'{rel}: supply its finished backgrounds/{bg["name"]}.png to verify registration')
            elif wide is not None:
                x, y, w, h = layout['stage_rect_xywh']
                if wide.crop((x, y, x+w, y+h)).tobytes() != core.tobytes():
                    errors.append(f'{rel}: fixed stage differs from the finished core background; paste the core at {x},{y} without resizing')
                else:
                    checked['widescreen'] += 1

    expected_atlases = {page['source']: page for page in manifest['sprite_style_atlases']}
    for name in expected_atlases:
        known.add(f'sprite_tilemaps/{name}')
    for path in sorted((args.edited / 'sprite_tilemaps').glob('*.png')):
        if path.name not in expected_atlases:
            errors.append(f'Unknown sprite atlas {path.name}')
            continue
        layout = json.loads((args.pack / 'context/sprite-manifests' / f'{path.stem}.json').read_text())
        page = layout['pages'][0]
        atlas = image(path, page['canvas_size'])
        if atlas is None:
            continue
        if atlas.getchannel('A').getextrema()[0] == 255:
            errors.append(f'{path.name}: sprite atlas needs actual alpha, not an opaque background')
        rectangles = Image.new('L', atlas.size, 0)
        d = ImageDraw.Draw(rectangles)
        for frame in page['frames']:
            x, y, w, h = frame['rect']
            d.rectangle((x, y, x+w-1, y+h-1), fill=255)
            empty = atlas.crop((x, y, x+w, y+h)).getchannel('A').getbbox() is None
            if empty != frame['empty']:
                errors.append(f'{path.name} frame {frame["index"]}: empty/nonempty status changed')
        outside = ImageChops.multiply(atlas.getchannel('A'), ImageChops.invert(rectangles))
        if outside.getbbox():
            errors.append(f'{path.name}: nontransparent pixels outside frame rectangles/gutters')
        checked['sprite_tilemaps'] += 1

    sounds = {s['file']: s for s in manifest['audio']}
    known.update(f'audio/{name}' for name in sounds)
    for path in sorted((args.edited / 'audio').glob('*.wav')):
        if path.name not in sounds:
            errors.append(f'Unknown audio filename {path.name}')
            continue
        try:
            with wave.open(str(path)) as sound:
                if (sound.getnchannels(), sound.getsampwidth(), sound.getcomptype()) != (1, 2, 'NONE') or sound.getframerate() not in (44100, 48000):
                    raise ValueError('expected mono 16-bit PCM WAV at 44100 or 48000 Hz')
                if sound.getnframes() == 0:
                    raise ValueError('empty audio')
                duration = sound.getnframes() / sound.getframerate()
                if abs(duration - sounds[path.name]['duration_seconds']) > .02:
                    warnings.append(f'{path.name}: duration changed by {duration - sounds[path.name]["duration_seconds"]:+.3f}s; review timing and notes')
                checked['audio'] += 1
        except (OSError, EOFError, wave.Error, ValueError) as error:
            errors.append(f'{path.name}: {error}')
    for path in args.edited.rglob('*'):
        if path.is_file() and path.suffix.lower() in ('.png', '.jpg', '.jpeg', '.webp', '.wav'):
            if path.relative_to(args.edited).as_posix() not in known:
                errors.append(f'Unexpected asset path: {path.relative_to(args.edited)}')
    if sum(checked.values()) == 0:
        errors.append('No valid returned assets found')
    print(json.dumps({'ok': not errors, 'checked': checked, 'warnings': warnings, 'errors': errors}, indent=2))
    return 1 if errors else 0


if __name__ == '__main__':
    sys.exit(main())
