# Standalone HD-2D presentation

Start with `./PlayHD2D.sh`, or select **3** in the player. Original (**1**) and
Storybook Remaster (**2**) retain their art, timelines and renderer. The default
launcher still starts Storybook.

## Artwork and staging

Option 3 uses `assets/hd2d-v2`: 253 named clips/poses and 614 transparent frames
extracted from 54 sprite masters, a clean ending backdrop, 10 core backgrounds,
30 panoramic variants and 23 remastered sound effects. Complete new actors replace
old separately drawn heads, arms and bodies. It is a reinterpretation of the
recovered stories, not 2,391 individually redrawn source frames.

The earlier `assets/hd2d` handoff remains as source/reference. Its hybrid sprite
set is not used by option 3: comparison found 107 sheets identical to Storybook.
File-format validation alone did not establish a complete artistic remake.

All 41 browser entries have explicit choreography. `src/hd2d_story.rs` directs
40 entries, including aliases and supporting clips; `src/hd2d_ending.rs` directs
the ending. Fishing lines attach to measured rod tips, food and letters transfer
between complete poses, ships carry their passengers, and props persist through
their relevant actions. Sounds are retimed to those actions. Standalone helpers
receive enough context to make sense in the browser.

`src/hd2d_layout.rs` describes the painted world: sand, curved palm, horizon,
shoreline and water. Both original island positions map to the same new island.
Sprites use these coordinates throughout movement, contacts, occlusion and
lighting. Boats pass behind the island and trunk; underwater parts receive water
coverage. Sprites can cross the old 640-pixel stage boundary into panorama margins.

Day, night and underwater panoramas come from continuous repaired masters.
Each core and its three wider variants are crops of the same registered painting,
so stage pixels match byte-for-byte. Office, beach and title scenes retain their
intentional framing. The original supplied paintings remain in `assets/hd2d`.

## Controls

| Control | Behavior |
| --- | --- |
| 1 / 2 / 3 | Original / Storybook / HD-2D |
| H | Cycle art styles |
| L / `--no-effects` | Toggle HD-2D atmosphere |
| A / `--aspect MODE` | Auto, original/core, 16:9, 21:9, 43:18 |
| S / `--audio MODE` | Original/remastered sound |
| P | Pause animation and visual time |
| M | Mute |
| Right / Left | Browse scenes |
| R | Restart scene |
| F / Esc | Fullscreen / quit |

Style changes preserve elapsed scene time and aspect/audio settings. Pause also
holds animation phases and visual effects. Original/Storybook keep their existing
browser compilation policy; HD-2D expands source holds onto a uniform 60 Hz clock
and permits longer stories to reach their endings.

```sh
./PlayHD2D.sh --scene MJFIRE --fullscreen
./PlayHD2D.sh --aspect 43:18 --resolution 3440x1440
./PlayHD2D.sh --no-effects --audio original
```

## Rendering and asset production

This remains a native Rust/SDL2 software compositor. Its optional postprocessing
adds campfire lighting, two scrolling water-normal layers, reflections,
embers/smoke, emissive blur and ACES grading. The effects mask follows new-world
sprite coverage. There is no 3D geometry or WGPU/OpenGL shader backend.

Generated PNGs and production prompts are preserved in `masters/` and `prompts/`.
The built-in image-generation tool produced the new raster art. The preparation
script only extracts alpha components, registers anchors, crops panoramas, and
writes runtime caches; it does not draw substitute characters.

```sh
python3 tools/prepare_hd2d_scene_assets.py
```

`scene-art.json`, `scene-art.tsv` and `scene-contacts.tsv` record frame bounds,
scale, feet/prop anchors and contacts. `scene-backgrounds.tsv` records each camera
and stage rectangle. `Play.sh` rebuilds missing or stale `.rgba` caches from PNGs.

## Verification

See [the scenario review](HD2D_STORY_REVIEW.md) and
[`evidence/hd2d-story-audit.json`](../evidence/hd2d-story-audit.json) for current
acceptance evidence. Earlier static and motion reports are historical and do not
establish acceptance of this rewrite.

```sh
cargo test --release
CARGO_MANIFEST_DIR="$PWD" rustc --edition=2021 -O -Awarnings tools/monitor_hd2d_motion.rs -o target/monitor-story
target/monitor-story captures/story-telemetry --all --no-video --limit=14400
# Source animation left; actual HD-2D rendering right:
target/monitor-story captures/story-comparison --effects --limit=14400 SMDATE MJFIRE
CARGO_MANIFEST_DIR="$PWD" rustc --edition=2021 -O -Awarnings tools/review_hd2d_scenes.rs -o target/review-hd2d-scenes
target/review-hd2d-scenes
```

The static browser review uses the complete HD-2D timelines and writes action
checkpoints plus 40 background/aspect compositions to `captures/hd2d-story-final`.
Continuous telemetry exercises every tick. Native runs verify the actual SDL
window, audio device, controls and measured presentation rate. Original and
Storybook reference captures are checked byte-for-byte for preservation.

These sequences use expressive key poses and complete gait cycles rather than
reconstructing every original facial and limb variation.
