#!/usr/bin/env python3
"""Exercise live controls on this script's own SDL window under Hyprland."""
import json
import pathlib
import subprocess
import time

from PIL import Image, ImageChops

root = pathlib.Path(__file__).resolve().parents[1]
out = root / 'captures/story-native-controls-final'
out.mkdir(parents=True, exist_ok=True)
proc = subprocess.Popen(
    [str(root / 'target/release/screen-antics'), '--style', 'hd2d', '--scene',
     'MJFIRE', '--aspect', '43:18', '--resolution', '3440x1440', '--fullscreen'],
    cwd=root, stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
)
lines = []
while True:
    line = proc.stdout.readline()
    lines.append(line)
    if 'Panoramas:' in line:
        break
    if not line:
        raise RuntimeError('Player failed before ready: ' + ''.join(lines))


def key(name):
    subprocess.run(['hyprctl', 'dispatch',
                    f'hl.dsp.send_shortcut({{mods="",key="{name}",window="pid:{proc.pid}"}})'],
                   check=True, stdout=subprocess.DEVNULL)
    time.sleep(0.7)


def wait_title(label):
    deadline = time.monotonic() + 15
    while time.monotonic() < deadline:
        windows = json.loads(subprocess.check_output(['hyprctl', 'clients', '-j']))
        own = next((w for w in windows if w['pid'] == proc.pid), None)
        if own and label in own['title']:
            return
        time.sleep(0.1)
    raise RuntimeError(f'Player did not reach title state {label!r}')


captures = []


def capture(name):
    windows = json.loads(subprocess.check_output(['hyprctl', 'clients', '-j']))
    own = next(w for w in windows if w['pid'] == proc.pid)
    x, y = own['at']
    w, h = own['size']
    raw = out / 'owned-window.png'
    subprocess.run(['grim', '-l', '0', '-g', f'{x},{y} {w}x{h}', str(raw)], check=True)
    image = Image.open(raw).convert('RGB')
    image.thumbnail((1280, 720))
    path = out / f'{name}.png'
    image.save(path, compress_level=3)
    captures.append({'name': name, 'path': str(path.relative_to(root)), 'title': own['title']})
    # Exclude desktop notification area from the pixel comparison.
    return image.crop((0, image.height // 10, image.width, image.height))


def identical(a, b):
    return a.size == b.size and ImageChops.difference(a, b).getbbox() is None


try:
    time.sleep(24)
    key('p')
    paused = capture('paused-hd2d')
    held = capture('paused-held')
    for name, style, label in [('1', 'original', '256 colors'),
                               ('2', 'remaster', ' · Remaster · '),
                               ('3', 'hd2d', 'HD-2D Neo-Retro')]:
        key(name)
        wait_title(label)
        roundtrip = capture('style-' + style)
    style_match = identical(paused, roundtrip)
    key('l')
    unlit = capture('atmosphere-off')
    key('l')
    relit = capture('atmosphere-on')
    for mode in ['auto', 'original', '16_9', '21_9', '43_18']:
        key('a')
        aspect_return = capture('aspect-' + mode)
    key('s')
    capture('audio-original')
    key('s')
    capture('audio-remastered')
    key('p')
    time.sleep(1.2)
    resumed = capture('resumed')
    key('Right')
    capture('next-scene')
    key('Left')
    capture('previous-scene')
    key('r')
    capture('restarted-scene')
    key('Escape')
    tail = proc.communicate(timeout=10)[0]
    (out / 'player.log').write_text(''.join(lines) + tail)
    titles = {c['name']: c['title'] for c in captures}
    result = {
        'scene': 'MJFIRE.TTM', 'exit': proc.returncode,
        'paused_frame_stable': identical(paused, held),
        'paused_style_roundtrip_identical': style_match,
        'atmosphere_changes_image': not identical(roundtrip, unlit),
        'atmosphere_roundtrip_identical': identical(roundtrip, relit),
        'aspect_roundtrip_identical': identical(relit, aspect_return),
        'resumed_frame_changes': not identical(aspect_return, resumed),
        'style_selection': all(label in titles['style-' + mode] for mode, label in
                               [('original', '256 colors'), ('remaster', 'Remaster'),
                                ('hd2d', 'HD-2D Neo-Retro')]),
        'aspect_selection': all(f' · {label} · ' in titles['aspect-' + mode] for mode, label in
                                [('auto', 'auto'), ('original', 'original'),
                                 ('16_9', '16:9'), ('21_9', '21:9'), ('43_18', '43:18')]),
        'audio_selection': 'original audio' in titles['audio-original']
                           and 'remastered audio' in titles['audio-remastered'],
        'scene_navigation': 'MJFISH.TTM' in titles['next-scene']
                            and 'MJFIRE.TTM' in titles['previous-scene']
                            and 'MJFIRE.TTM' in titles['restarted-scene'],
        'captures': captures,
    }
    result['ok'] = proc.returncode == 0 and all(
        result[k] for k in ['paused_frame_stable', 'paused_style_roundtrip_identical',
                           'atmosphere_changes_image', 'atmosphere_roundtrip_identical',
                           'aspect_roundtrip_identical', 'resumed_frame_changes',
                           'style_selection', 'aspect_selection', 'audio_selection',
                           'scene_navigation'])
    (out / 'results.json').write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps({k: v for k, v in result.items() if k != 'captures'}, indent=2))
    if not result['ok']:
        raise SystemExit(1)
finally:
    if proc.poll() is None:
        proc.terminate()
        proc.wait(timeout=10)
