# Screen Antics — native Rust remaster

A native Linux player for your supplied Johnny Castaway / Screen Antics data.
Three visual styles are available: Original, the Castaway Animated Storybook
Remaster, and Modern HD-2D Neo-Retro. The default is the Storybook remaster.
Original and Storybook retain the recovered staging; HD-2D uses new artwork,
choreography and contact positions for its painted world.
The renderer targets 60 Hz, without Wine, DOSBox, a browser or
Cargo dependencies. It links to the installed SDL2 library.

Install Rust, SDL2, Python 3 and Pillow. Put your supplied `RESOURCE.001` and
`SCRANTIC.EXE` in `original/`, or pass `--data /path/to/game/files` to a launcher.
The original game files are not included. Launchers rebuild derived image caches
from the included PNG artwork on first use.

```sh
./PlayHD2D.sh --fullscreen         # third style: new art + atmospheric effects
./Play.sh --style original        # option 1: original pixels
./PlayGemini.sh --fullscreen      # option 2: Storybook remaster
./Play.sh --style hd2d --no-effects # option 3: new art without runtime effects
./PlayGemini.sh --aspect 21:9 --audio original
./PlayGemini.sh --screensaver
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
H cycles the three art styles, **1/2/3** selects a style directly, **L** toggles
HD-2D atmosphere, and C toggles the retro palette treatment,
**A** cycles aspect modes (auto, original, 16:9, 21:9, 43:18), **S** switches
remastered/original audio, M mutes, and Escape exits. `--screensaver` enters fullscreen and exits on input.
It does not change your idle/lock configuration or provide a security lock.

The initial window respects the selected aspect ratio and fits the desktop.
Fullscreen and resizing use actual HiDPI output pixels; an explicit
`--resolution WIDTHxHEIGHT` fixes the internal framebuffer instead. Resolutions
from 320×200 through 7680×4320 are accepted. Uniform scene scaling preserves
character proportions. Widescreen scenery extends the outer sky/water strips
while the active stage keeps the original geometry and all actor coordinates.
Intentional black surrounds inside original office/title scenes remain.

## HD-2D third style

`./PlayHD2D.sh` or `./Play.sh --style hd2d` selects the standalone renderer and
new sprite catalog at `assets/hd2d-v2`. All original bitmap families have explicit
new-art or scene-effect roles; no old sprite textures are drawn in this mode.
The new island, curved palm, shoreline, ships, beach and office have their own
contact positions. Original and Storybook remain selectable with 1 and 2.

The new catalog has 253 clips/poses and 614 transparent frames. All 41 browser
entries now use authored action sequences, including complete fishing, cooking,
rescue, mermaid, raft, office and ending stories. Complete poses replace the
original layered body parts; this is not a one-for-one redraw of 2,391 source frames.

The new island, curved palm, shoreline and props use measured contact positions.
Continuous panoramic masters supply the day, night and underwater backgrounds;
every aspect crop retains identical stage pixels. There are 10 core backgrounds,
30 panorama variants and 23 remastered sound effects. L toggles lighting, caustics,
reflections, particles, bloom and ACES grading.
[Implementation and controls](docs/HD2D.md) · [Scenario review](docs/HD2D_STORY_REVIEW.md).

## Artwork

The complete **Gemini remaster** is available through `PlayGemini.sh` or
`./Play.sh --asset-pack assets/gemini-remaster`. The current Castaway Animated Storybook return
includes all 116 refreshed sprite atlases / 2,391 frames, including the paired
scenery updates, ten remastered
backgrounds, 30 registered panorama canvases and 23 remastered sound effects.
The hybrid AI-painted sprite pack remains a separate supplied alternative.
Custom packs use their actual
alpha, including smoothed edges and coverage beyond the old pixel silhouettes;
translucent pixels blend against the underlying scenery and sprites. Press 1
for the original indexed artwork. See the
[complete pack and source hashes](assets/gemini-remaster/README.md).
`PlayGemini.sh` rebuilds its ignored runtime cache from PNGs when needed (Python
3 and Pillow); the PNGs and per-atlas source hashes are the durable asset files.
The earlier sprite-only pack is retained at `assets/gemini-deterministic` and can
still be selected explicitly with `--asset-pack`.

Auto aspect selects the nearest available 16:9, 21:9 or 43:18 panorama; classic
4:3/portrait windows retain the core framing. A forced aspect fits its canvas
uniformly inside the current window. The core stage stays at the manifest's
registered offset, and sprites, clips and wipe regions use that same transform.
Only the active panorama is loaded into memory. Panoramas preserve intentional
black framing in office/title scenes. Packs without registered panoramas retain
the earlier edge-extension fallback.

Audio uses a fixed 44.1 kHz signed 16-bit SDL queue. Remastered samples are
decoded from `audio/WAVESFXnn.wav`; original samples are converted from the EXE,
and optional 48 kHz replacements are resampled without changing duration. The
verified SFX suffix mapping is preserved. S switches the source for subsequent
sound triggers and clears the current queue; M separately controls mute.

The fallback enhancement (used when no imported pack is installed) gives all
**116 sprite sheets / 2,391 frames** the same 4× contour-aware
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

Sprite replay is idempotent for the same original pose, including identical
frames stored in different resources. Saved antialiased poses and overlapping
parts do not gain opacity on held redraws. HD sampling uses source-pixel scales
and mirrors selected texels, keeping its phase stable when frame padding changes.
See [flicker investigation and regressions](evidence/flicker-fix.json).

The player executes recovered ADS actor rules, instead of playing stored
TTM frames linearly. Actors keep separate bitmap/getput state; start/stop,
repetitions, GOTO loops, timed cuts and successor conditions control playback.
Held poses replay their entry drawing state; timed actors repeat until their
deadline; WHILE waits for the actor to finish; explicit layer order is retained.
Gulliver selects a branch covering the ship's arrival and departure. Its
`A050/A060` commands save and restore a clean composition rectangle; they are
not visual wipes in the shipped Windows executable. The older `4210/A600`
get/put commands are stubs in that build. Buffer 1 is composition and buffer 2
is opaque stored backing, initialized from the scene picture. Matching these
native semantics prevents the parked ship from remaining behind as it departs.
See [the original executable evidence](evidence/native-buffer-analysis.md) and
[verification of the fix](evidence/native-buffer-fix.json): 53 release tests,
55,014 scheduled ticks across 41 scenes, and no HD changes during identical poses.

Panoramic scenes retain sprite pixels beyond both original stage edges, including
transparent layers and stored/captured poses. Stage-edge clips and saved regions
extend into those side areas; explicit interior masks retain their authored bounds.
Retro and Original-aspect presentation keep the original framing.

HD composition tracks the source of every final pixel through background saves,
stored areas, captures and restores. It preserves the VM's actual draw order,
clipping and opaque black. It no longer guesses overlap from rectangular sprite
bounds or the color of the tree. Interactions are authored sprite placements,
not a simulated physics/collision engine.

The original Windows executable's global itinerary/day/night/holiday state
machine remains unverified. Scene browsing chooses an ADS segment starting
the requested animation. Four timelines reach a 2,400-source-tick browser cap;
four directly launched TTMs use bounded local loops. Random paths are seeded,
random delay ranges use their midpoint. The Rust player restores the entire
stored backing each tick, while Windows uses dirty rectangles; native frame
scheduling has not been fully verified. Scene changes use a modern crossfade.
`--raw-scripts` retains linear traversal for diagnostics;
`--original-timing` selects the previous build's 50 ms tick interpretation.
Modern playback uses 60 script ticks per second with original pose holds.

## Build and verification

### Sprite tilesets for external artwork editing

The export tools pack all 116 original sprite sets / 2,391 frames into transparent
4× PNG editing sheets, with labeled reference copies, individual native PNGs,
frame manifests, a searchable offline index and a model prompt. Each page has
at most 16 frames and fits within 2048×2048. Original canvas anchors and alpha
are retained; these templates contain no generated or repainted artwork.

```sh
mkdir -p target/tools exports
~/.cargo/bin/rustc --edition=2021 -O -A dead_code tools/audit_sprites.rs -o target/tools/export-original-sprites
target/tools/export-original-sprites original exports/.sprite-source
python tools/export_sprite_tilesets.py
```

The pack is written to `exports/sprite-tilesets/` and its ZIP alongside it. Open
`index.html` to browse, and send PNGs from `editing/` to the image model. Matching
`reference/` PNGs carry labels and guides. Return edited pages with the same
filename, dimensions, frame positions and real transparency. Exported original
artwork stays local under ignored `exports/`.

Use a new `--output` directory and `--archive` path when repeating the export.
`tools/unpack_sprite_tilesets.py` (also included in the pack) validates edited
pages and extracts frames into a separate output folder; it never installs
or enables them automatically. See the pack's README for its command.

For **one complete tilemap image per sprite set**, use:

```sh
python tools/export_sprite_tilesets.py --layout atlas
```

This creates `exports/sprite-tilemaps/` and
`exports/screen-antics-sprite-tilemaps.zip`: 116 transparent PNG atlases,
each containing every frame from its source set in a uniform grid. Labeled
references and smaller previews are included. Full editing images retain exact
4× source dimensions; the largest combined atlas is 6272×7128. The original
smaller paginated pack remains available for models with lower image limits.

### Background, widescreen and audio handoff for Gemini

The companion `exports/remaster-handoff/` pack contains all 10 original backgrounds
and clean 4× editing PNGs, 30 registered widescreen templates (16:9, 21:9 and 43:18),
coordinate guides and extension masks. It also includes all 116 imported sprite
atlases as style references, 41 silent scene videos, 328 stills, per-tick drawing/
clipping/transition metadata, 23 original WAV effects and their observed triggers.
Open its `index.html` and start with `GEMINI_PROMPT.txt`. Exact return paths and
dimensions are in its README and manifest. A ZIP is written alongside the folder.

```sh
~/.cargo/bin/rustc --edition=2021 -O -A dead_code tools/export_remaster_context.rs -o target/tools/export-remaster-context
target/tools/export-remaster-context original assets/gemini-deterministic exports/.remaster-context-source
python tools/export_remaster_handoff.py
python tools/validate_remaster_edits.py --pack exports/remaster-handoff --edited /path/to/edited
```

Outputs must be empty/new; use explicit output arguments to preserve earlier
exports. The packer requires the earlier sprite tilemap manifests and original
editing sheets, Python/Pillow and ffprobe; the Rust exporter uses ffmpeg for video.
Previews sample the current port at 20 FPS and omit display-only transitions;
they are not recordings of the original Windows executable. The full metadata
retains every compiled tick. No export changes runtime artwork or configuration.
Import a complete returned pack into a new output directory:

```sh
python tools/import_remaster_handoff.py \
  --handoff /path/to/remaster-handoff \
  --atlases /path/to/edited_deterministic \
  --output assets/gemini-remaster
```

The importer validates the return folder, overlays its scenery sheets onto the
complete sprite set, preserves PNG/WAV data, and writes registered panorama
metadata to `widescreen/layouts.tsv`. Runtime support for these backgrounds,
widescreen canvases and remastered WAVs is now implemented. No edit is installed
from an external folder automatically; the player uses its local imported pack.

### Rust build

`Play.sh` rebuilds when Rust sources change. Rust is expected at
`~/.cargo/bin/cargo` and SDL2 at the system linker path.

```sh
~/.cargo/bin/cargo build --release --offline
~/.cargo/bin/cargo test --release --offline
~/.cargo/bin/cargo fmt --check
```

All 84 release tests pass. The HD-2D rewrite was reviewed across all 41 scenarios,
65,883 timeline ticks, 860 action checkpoints and 40 background/aspect compositions.
See the [HD-2D review](docs/HD2D_STORY_REVIEW.md) and
[final verification report](evidence/hd2d-story-final.json).

The earlier complete Gemini pack's scene-browser verification
covers all 41 timelines (55,014 ticks), unchanged indexed output, 328 ultrawide
captures and 572 mapped sound requests. Every one of the 30 panorama canvases
passes a rendered pixel comparison against its source. All 2,391 runtime sprite
frames and 40 background/panorama images match the supplied pixels; all 23
runtime-decoded remastered PCM buffers match the supplied WAV samples exactly.
Storybook playback measured **59.72 FPS at 1920×1080** (eight seconds) and
**59.73 FPS at 3440×1440** (twelve seconds). The initial eight-second ultrawide
run measured 58.71 FPS; both measurements are retained in the validation record.
Current review captures are under `captures/storybook-review/`.
See [complete validation](evidence/complete-remaster-validation.json) and the
[playback inventory](evidence/complete-remaster-scene-review.tsv).

Validation includes complete playback of all 41 scene entries and all 6,631 raw
animation frames. An identity-art regression compares every pixel of every raw
frame between indexed and HD composition. Additional tests cover saved actors,
opaque restore, primitive/sprite ordering, mirrored/clipped sprites, 1080p/ultrawide/
4K anchors, every source pose, stable colors across frame sizes, actor holds,
timed repeats, WHILE continuations, layering and the actual Gulliver departure.

`tools/review_scenes.rs` reproduces eight review captures per scene and the
[playback inventory](evidence/scene-review.tsv). Pass an imported pack directory
and output directory as positional arguments to review a custom pack and compare
its indexed output against the default VM throughout playback. Optional third and
fourth arguments select aspect and output resolution, for example `auto 1290x540`.
The tool also verifies every registered panorama and exports decoded audio buffers.
All 30 original sprite contact
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
