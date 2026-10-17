# Animation and scheduler reconstruction

## Evidence

The supplied `RESOURCE.001` contains 41 `.TTM` resources, 6,631 frame delimiters, and 10 `.ADS` resources containing 66 rule segments. TTM `VER:` is `4.09`; `PAG:` stores the frame count; `TT3:` carries the compressed instructions. The shipped TTMs use compression method 1, DGDS run-length encoding. Images also use method 2, LZW.

Instruction words are little endian. Their low nibble specifies the number of signed 16-bit operands; `0xF` introduces a null-terminated, word-padded string. `0x0FF0` ends a frame. `0x1111` starts an actor sequence; `0x1101` also defines a GOTO target. Asset filenames, coordinates, sprite frame numbers, palette references, clip rectangles, and frame delays are interpreted from the original archive.

The original first implementation followed TTM storage order. That missed **528 PURGE commands**, which end an individual actor sequence, and **75 GOTO commands**, which loop/jump. It also played unrelated labeled states consecutively. The ADS resources establish the actual dependencies between these states, including concurrent actors, repeated sequences, timed cuts, compound conditions, weighted random choices, and explicit stop/start operations.

For example `WALKSTUF.ADS`, segment 3, starts `MJJOG.TTM` sequence 1; completion starts 2, then 3. It runs sequence 13 twice, sequence 14 three times, and sequence 4 three times. Those dependencies and repeat counts are now executed. `ACTIVITY.ADS` combines independently timed actors; drawing order follows source sequence order rather than numeric label order.

Primary format/semantic references (used with direct inspection of the supplied bytes):

- [ScummVM TTM interpreter](https://github.com/scummvm/scummvm/blob/master/engines/dgds/ttm.cpp)
- [ScummVM ADS interpreter](https://github.com/scummvm/scummvm/blob/master/engines/dgds/ads.cpp)
- [ScummVM chunk parsers](https://github.com/scummvm/scummvm/blob/master/engines/dgds/parser.cpp)
- [ScummVM compression](https://github.com/scummvm/scummvm/blob/master/engines/dgds/decompress.cpp)

## Native implementation

`animation.rs` decodes TTM drawing commands. All visual opcodes present in the supplied scripts are supported: sprites and horizontal mirroring; get/put rectangles; persistent-area storage; colors; pixels, lines, rectangles, ellipses; clipping; and buffer copies. Each frame exposes resolved sprite placements plus the current background identity for the HD renderer. Palette changes share the original common palette. Sample requests are emitted for the frontend's audio queue.

`scheduler.rs` parses ADS environment references and rule segments, then compiles a deterministic timeline for each scene browser entry. It implements NOT_PLAYED/PLAYED/FINISHED/RUNNING/NOT_RUNNING conditions, nested conditions, AND/OR, weighted choices, stop/start, positive repeat counts, negative timed cuts, GOTO, and PURGE. Bitmap and get/put slots are namespaced per environment, and held actor frames redraw without repeating allocation or sound side effects. Timelines use original scheduler ticks; the frontend controls wall-clock playback speed.

The native scene browser chooses the ADS segment that most directly starts the requested TTM; this is a browser policy. The original Windows application chooses these segments through its own screensaver state machine. Random choices use a fixed seed so captures and tests reproduce the same path.

## Validation

- The raw VM test executes all 41 supplied TTMs and every one of the 6,631 stored frames without failure.
- The scheduler compiles all 41 scene entries, each containing actual sprite drawing commands.
- A separate release harness executes every compiled timeline to completion and records sprite draw counts.
- Focused tests verify that PURGE repeats exactly the requested number of times, FINISHED starts the successor, and GOTO loops until an ADS time cut before the next sequence runs.
- Malformed operands/strings and sprite mirroring/clipping/transparency have regression coverage.
- A scheduled MJJOG frame was rendered for visual inspection, alongside the raw clip reference.

## Remaining limits

The original Windows application's global itinerary and direct-launch policies have not been fully reconstructed. TTM resources with no direct ADS start reference use a local browser policy: stored sequences with explicit GOTO loops bounded to three jump-backs. This affects `FIRE.TTM`, `GJGULL1.TTM`, `GJVIS5W.TTM`, and `MJCOCO1.TTM`.

ADS timelines are bounded to 2,400 source ticks to keep perpetual ambient sequences finite in a scene browser. Some stories therefore end at this cap. Random delay ranges use their midpoint. Primitive ellipse edges approximate the original rasterization. The two original wipe instructions now emit presentation events with their source rectangle and direction. They occur in `GJGULIVR.TTM` at raw frames 142 (left-to-right) and 321 (right-to-left), both over `(194,123,175,142)`. A 350 ms presentation duration replaces their hardware-dependent tight loops. Held identical repaints do not restart the transition. The HD frontend transitions toward the final composed frame; the original wipe occurs mid-frame before a few subsequent sprite draws, so this preserves direction and region while approximating that intra-frame staging. Other fade/wipe opcode variants are decoded into events for completeness, although they do not occur in this supplied archive. The HD asset renderer may improve source presentation, but its reconstructed assets are a remake, not recovered high-resolution originals.

## Stored-overlay transparency check

A proposed validity mask for stored areas was not applied: the primary engine reference explicitly uses `compositionBuffer.transBlitFrom(storedAreaBuffer)` ([dgds.cpp](https://github.com/scummvm/scummvm/blob/master/engines/dgds/dgds.cpp)), so palette index zero intentionally reveals the background in this overlay. Get/put rectangles use opaque copying and already restore zero-valued pixels. A regression test now verifies that distinction.


## Sprite, overlap and scheduler review

The current default uses source-faithful 4× RGB artwork for all 2,391 sprite
frames and exact scenery geometry. The earlier generated pack is experimental
(`--painted-art`), following the [complete sprite review](sprite-review/REVIEW.md).
HD composition now follows per-pixel source identity through every save, stored
area and restore; bounding-box masking and tree-color heuristics were removed.
An identity-art regression validates exact composition for all 6,631 raw frames.
The [scheduler review](scheduler-review.md) records corrected held drawing state,
timed repeats, waiting, layer order, and the Gulliver branch now reaching both
wipes. Full Windows story selection and scene handoff remain approximated.
