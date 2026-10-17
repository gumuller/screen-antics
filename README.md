# Screen Antics — native Rust remaster

A native Linux player for your supplied Johnny Castaway / Screen Antics data.
The default mode uses a consistent 4× enhancement of every original sprite and
background, true-color material filtering and a display-sized framebuffer.
The renderer targets 60 Hz, without Wine, DOSBox, a browser or
Cargo dependencies. It links to the installed SDL2 library.

```sh
./Play.sh --fullscreen
./Play.sh --fullscreen --resolution 2560x1440
./Play.sh --fullscreen --resolution 3440x1440
./Play.sh --retro                 # original pixels and palette
./Play.sh --painted-art           # experimental generated pack; known pose mismatches
./Play.sh --scene MJREAD
./Play.sh --list
./Play.sh --seconds 8             # desktop FPS measurement
./Play.sh --headless --scene MJJOG --frames 140 --resolution 3440x1440 --capture /tmp/antics.ppm
```

Left/Right or Space changes scenes. R restarts, P pauses, F toggles fullscreen,
H switches remastered/original artwork, C toggles the retro palette treatment,
M mutes, and Escape exits. `--screensaver` enters fullscreen and exits on input.
It does not change your idle/lock configuration or provide a security lock.

The initial window respects the selected aspect ratio and fits the desktop.
Fullscreen and resizing use actual HiDPI output pixels; an explicit
`--resolution WIDTHxHEIGHT` fixes the internal framebuffer instead. Resolutions
from 320×200 through 7680×4320 are accepted. Uniform scene scaling preserves
character proportions. Widescreen scenery extends the outer sky/water strips
while the active stage keeps the original geometry and all actor coordinates.
Intentional black surrounds inside original office/title scenes remain.

## Artwork

All **116 sprite sheets / 2,391 frames** now use the same 4× contour-aware
processing and local RGB material filter. Clothing, gestures, separate props,
small marks and disconnected water effects follow the recovered frames. Colors
do not change merely because the next pose has a different frame size. Scenery
uses the same treatment and retains the exact tree, shore and prop geometry.
This is an enhancement of the original artwork, not a newly illustrated HD set.

The earlier generated pack is preserved behind `--painted-art`: ten painted
backgrounds, two panorama variants and 96 replacement frames. A complete visual
review found wrong book gestures, duplicate-cap risk, changed jogging clothes,
and missing tiny artwork despite matching dimensions. That pack is experimental
and is no longer the default. See the [complete sprite audit](evidence/sprite-review/REVIEW.md).

Assets, exact prompts and extraction manifests are saved in
[assets/remastered](assets/remastered/PROMPTS.md). Generation used built-in
ImageGen. Original and generated atlases are kept alongside runtime `.rgba`
packs; `tools/pack_image.py` performs lossless PNG-to-SAHD conversion, and
`tools/isolate_sprite_frames.py` reproduces technical sprite extraction.
The original publisher's assets remain in ignored `original/`; use `--data DIR`
for another supplied copy. The source-code license excludes original assets.
Git also excludes build outputs, captures and extracted original-art review
images. The review tools regenerate those local previews; their measurements
and written findings are tracked alongside the source and experimental art pack.

[Corrected reading sequence](evidence/scene-review/MJREAD.TTM.jpg) ·
[Corrected jogging sequence](evidence/scene-review/MJJOG.TTM.jpg) ·
[All scene review captures](evidence/scene-review/REVIEW.md)

## Animation and transitions

The player executes recovered ADS actor rules, instead of playing stored
TTM frames linearly. Actors keep separate bitmap/getput state; start/stop,
repetitions, GOTO loops, timed cuts and successor conditions control playback.
Held poses replay their entry drawing state; timed actors repeat until their
deadline; WHILE waits for the actor to finish; explicit layer order is retained.
Gulliver now selects a branch containing both recovered directional wipes. Held
poses do not restart timed transitions. The source's directional wipes animate at display cadence over their original
regions, and scene changes crossfade. A 350 ms duration replaces original
hardware-dependent wipe loops. Original rectangle capture restores opaque
pixels, while the persistent stored area uses its original transparency key.

HD composition tracks the source of every final pixel through background saves,
stored areas, captures and restores. It preserves the VM's actual draw order,
clipping and opaque black. It no longer guesses overlap from rectangular sprite
bounds or the color of the tree. Interactions are authored sprite placements,
not a simulated physics/collision engine.

The original Windows executable's global itinerary/day/night/holiday state
machine remains unverified. Scene browsing chooses an ADS segment starting
the requested animation. Four timelines reach a 2,400-source-tick browser cap;
four directly launched TTMs use bounded local loops. Random paths are seeded,
random delay ranges use their midpoint, and wipe staging approximates the
mid-frame original effect using the composed target. This improves the known
broken behavior without claiming complete original-runtime equivalence.
`--raw-scripts` retains linear traversal for diagnostics;
`--original-timing` selects the previous build's 50 ms tick interpretation.
Modern playback uses 60 script ticks per second with original pose holds.

## Build and verification

`Play.sh` rebuilds when Rust sources change. Rust is expected at
`~/.cargo/bin/cargo` and SDL2 at the system linker path.

```sh
~/.cargo/bin/cargo build --release --offline
~/.cargo/bin/cargo test --release --offline
~/.cargo/bin/cargo fmt --check
```

All 34 release tests pass. Native measured presentation is **59.86 FPS** at
1920×1080 and **59.66 FPS** at 3440×1440 (eight seconds each).

Validation includes complete playback of all 41 scene entries and all 6,631 raw
animation frames. An identity-art regression compares every pixel of every raw
frame between indexed and HD composition. Additional tests cover saved actors,
opaque restore, primitive/sprite ordering, mirrored/clipped sprites, 1080p/ultrawide/
4K anchors, every source pose, stable colors across frame sizes, actor holds,
timed repeats, WHILE continuations, layering and real Gulliver wipes.

`tools/review_scenes.rs` reproduces eight review captures per scene and the
[playback inventory](evidence/scene-review.tsv). All 30 original sprite contact
pages and all 96 generated replacements were visually inspected. The stills
check visible stages; they do not establish original-runtime equivalence.

See [executable findings](evidence/executable.md) and
[script findings and limits](evidence/scripts.md). Archive SHA-256:
`111c384aa44fc810c0453f524d8c02dee58ea3358ee788316b2fc1f2059afc56`.
REA inventory evidence:
`ev_0f67af210ae6921bfa2ba0253ce393a287174dbd819fdbbcb0b00ffe0e37c68e`.
The Win16 NE executable was inspected with local static tools after REA
reported that format unsupported; the original program was not executed.
The independent Rust implementation was informed by ScummVM's primary
[DGDS resource](https://github.com/scummvm/scummvm/blob/master/engines/dgds/resource.cpp),
[TTM](https://github.com/scummvm/scummvm/blob/master/engines/dgds/ttm.cpp), and
[ADS](https://github.com/scummvm/scummvm/blob/master/engines/dgds/ads.cpp) references.
