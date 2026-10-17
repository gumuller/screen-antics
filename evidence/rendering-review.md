# Rendering and animation verification

The HD renderer now tracks the source of each indexed pixel through the actual
VM operation order. Sprite coverage, clipping, primitive overwrites, opaque
palette-zero copies, background saves, stored regions and GETPUT restoration
carry this identity together. Presentation samples the corresponding HD texture
only where that source actually owns the final pixel. Rectangular sprite masks,
post-hoc sprite replay and the tree-color heuristic were removed.

All 2,391 source poses use a consistent 4x enlargement and local RGB filtering by
default; the 96 generated interpretations and painted scenery remain behind
`--painted-art`. Local material color depends on neighboring pixels rather than
normalized frame bounds, preventing shade changes as pose dimensions change.
The original stage geometry remains fixed at every output aspect ratio.

Validation:

- 34 release tests pass; cargo fmt --check passes; release build succeeds offline.
- All 6,631 original raw frames compare pixel-for-pixel between indexed composition
 and HD composition with identity artwork. This checks composition, not equivalence
 to the unexecuted Windows original.
- All 41 scheduled scene timelines render to their browser completion/cap. Eight
 snapshots per scene are recorded in scene-review/*.jpg; scene-review.tsv has
 exact frame and draw totals.
- All 116 sprite sheets / 2,391 frames and all 96 generated replacements were visually
 reviewed; see sprite-review/REVIEW.md.
- Native 1920x1080: 479 presents / 8.002 seconds = 59.86 FPS; 481 script ticks.
- Native 3440x1440: 478 presents / 8.012 seconds = 59.66 FPS; 480 script ticks.
- Native Gulliver 1920x1080: 2,039 presents / 34.006 seconds = 59.96 FPS; 1,999 script ticks.
 The run crossed both recovered wipe events; their two 350 ms holds account for
 the difference between presentation cadence and source tick advancement.

Run measurements used --mute and --seconds. Artwork decoding/loading precedes
the playback timer. They measure presented frames, not a GPU profiler's latency
or original-script speed. The two short runs had no simultaneous test suite.
The long Gulliver run overlapped the offline contact capture refresh at startup.

The source Windows global itinerary, time-of-day state and some scene handoffs
remain approximated. Four browser timelines hit a 2,400 tick cap. Random selection
is seeded, random delays use their midpoint, and wipes target the final composed
frame rather than the original mid-frame buffer. See scheduler-review.md.

The current preview is launched through Play.sh; H toggles original pixel art,
Space advances scenes and Escape exits.
