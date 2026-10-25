#!/usr/bin/env python3
"""Validate and losslessly install a handoff into a new, self-contained asset pack."""
import argparse
import hashlib
import json
from pathlib import Path
import shutil
import struct
import subprocess
import sys
import tempfile
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def convert(source, target):
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(source, target.with_suffix('.png'))
    with Image.open(source) as image:
        rgba = image.convert('RGBA')
        target.write_bytes(b'SAHD' + struct.pack('<II', *rgba.size) + rgba.tobytes())


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--handoff', type=Path, required=True)
    parser.add_argument('--atlases', type=Path, required=True)
    parser.add_argument('--layout', type=Path, default=ROOT / 'exports/sprite-tilemaps')
    parser.add_argument('--output', type=Path, default=ROOT / 'assets/gemini-remaster')
    args = parser.parse_args()
    if args.output.exists():
        parser.error('Choose a new output directory; existing packs are preserved.')
    edited = args.handoff / 'edited'
    subprocess.run([sys.executable, str(ROOT/'tools/validate_remaster_edits.py'), '--pack', str(args.handoff), '--edited', str(edited)], check=True)
    manifest = json.loads((args.handoff/'manifest.json').read_text())
    original_layout = json.loads((args.layout/'manifest.json').read_text())
    args.output.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='.remaster-import-', dir=args.output.parent) as temp:
        temp = Path(temp); merged = temp/'atlases'; merged.mkdir(); pack = temp/'pack'; pack.mkdir()
        sources = []
        for sheet in original_layout['sheets']:
            stem = Path(sheet['source_sheet']).stem
            name = f'{stem}.png'
            override = edited/'sprite_tilemaps'/name
            source = override if override.exists() else args.atlases/name
            if not source.is_file():
                raise ValueError(f'Missing complete sprite atlas: {source}')
            shutil.copy2(source, merged/name)
            sources.append({'sheet': sheet['source_sheet'], 'source': str(source), 'sha256': digest(source), 'handoff_override': override.exists()})
        subprocess.run([sys.executable, str(ROOT/'tools/unpack_sprite_tilesets.py'), '--pack', str(args.layout), '--edited-dir', str(merged), '--output', str(pack/'sprites')], check=True)
        rows = ['background\tratio\twidth\theight\tx\ty\tstage_width\tstage_height\tfile']
        files = []
        for bg in manifest['backgrounds']:
            name = bg['name']
            source = edited/'backgrounds'/f'{name}.png'
            convert(source, pack/'backgrounds'/f'{name}.rgba')
            files.append({'file': f'backgrounds/{name}.png', 'source_sha256': digest(source)})
            for wide in bg['widescreen']:
                source = edited/wide['return_file']
                if not source.is_file():
                    raise ValueError(f'Missing widescreen deliverable: {source}')
                target = pack/'widescreen'/wide['ratio']/f'{name}.rgba'
                convert(source, target)
                w,h = wide['canvas_size'];x,y,sw,sh = wide['stage_rect_xywh']
                rows.append('\t'.join(map(str,[name,wide['ratio'],w,h,x,y,sw,sh,f"{wide['ratio']}/{name}.rgba"])))
                files.append({'file': target.with_suffix('.png').relative_to(pack).as_posix(), 'source_sha256': digest(source)})
        (pack/'widescreen/layouts.tsv').write_text('\n'.join(rows)+'\n')
        (pack/'audio').mkdir()
        for sound in manifest['audio']:
            source = edited/'audio'/sound['file']
            if not source.is_file():
                raise ValueError(f'Missing audio deliverable: {source}')
            shutil.copy2(source, pack/'audio'/sound['file'])
            files.append({'file': f"audio/{sound['file']}", 'source_sha256': digest(source)})
        shutil.copy2(edited/'notes.txt', pack/'NOTES.txt')
        (pack/'handoff-manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
        report = {'format':'screen-antics-complete-remaster-v1','handoff':str(args.handoff),'base_atlases':str(args.atlases),'sprites':sources,'files':files,'counts':{'sprite_sheets':len(sources),'sprite_frames':json.loads((pack/'sprites/import-manifest.json').read_text())['frames'],'sprite_overrides':sum(s['handoff_override'] for s in sources),'backgrounds':len(manifest['backgrounds']),'widescreen':len(rows)-1,'audio':len(manifest['audio'])}}
        (pack/'import-manifest.json').write_text(json.dumps(report,indent=2)+'\n')
        (pack/'.gitignore').write_text('# Derived runtime cache; PlayGemini.sh regenerates it from PNGs.\n**/*.rgba\n**/*.tmp\n')
        (pack/'README.md').write_text('# Complete Gemini remaster\n\nRun `./PlayGemini.sh` from the project root.\n\nAll sprite, background, panorama and audio files come from the validated user\nhandoff. PNG pixels and WAV bytes are preserved; SAHD files are derived runtime\ncaches. See import-manifest.json for source hashes and NOTES.txt for art details.\n\nH: original/remastered art. A: aspect mode. S: original/remastered audio.\nM: mute. F: fullscreen. Arrow keys: scenes.\n')
        pack.rename(args.output)
    print(json.dumps({'installed':str(args.output),**report['counts']},indent=2))


if __name__ == '__main__':
    main()
