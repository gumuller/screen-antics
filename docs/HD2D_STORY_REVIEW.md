# HD-2D scenario review

Option 3 now has a separate scene director and art catalog. Every one of the 41
browser entries has explicit choreography: 40 in `src/hd2d_story.rs`, with the
parachute ending in `src/hd2d_ending.rs`. Original and Storybook keep their existing
art and scene timelines.

The review compared the recovered setup, action, reaction and ending with the
new presentation. Repairs include complete fishing casts and catches; fish and
letters changing hands; ignition, roasting and the hot-food reaction; five catches
and an octopus escape; diving judges and scorecards; coconut handling; gull thefts;
ships approaching in perspective; mermaid courtship and the dinner mishap; raft
construction and departure; the office daydream; and the parachute landing.

These are newly staged interpretations of the recovered stories. The catalog has
253 named clips/poses and 614 transparent frames from 54 sprite masters. It is not
a frame-for-frame redraw of all 2,391 original source frames. The browser does not
fall back to old character textures or a generic action loop.

## Spatial and presentation corrections

The new sandbar, curved palm, shore, waterline and desk have their own contact
coordinates. Complete actors replace the original separately drawn heads, arms
and bodies. Rod tips, held food, letters, scorecards, ropes and passengers have
registered contacts. Necessary props persist between actions; hidden actors are
used only for deliberate interactions such as the shark swallowing gag.

Climbers remain visible in front of the crown while wrapping the trunk. Ships
can travel across the panorama margins without the old 4:3 clipping. Day, night
and underwater backgrounds now come from continuous repaired panorama masters;
their core and wider crops have identical registered stage pixels. Office, beach,
title and ending retain their intentional framing.

The ending now approaches the island in perspective and reaches the scale of the
painted final silhouette. A larger ship master supplies the extreme foreground
pass. Large translucent draws and ending dissolves blend disjoint rows in parallel;
cached overlays and framing avoid repeated full-image geometry work. Late frames
no longer incur an additional full frame interval of sleep.

## Reproducible verification

Final result: **84 release tests passed**, all 41 native scene runs exited cleanly,
and all live-control checks passed. The final capture-free 3440×1440 runs with
effects measured **59.48 FPS for THEEND** and **58.84 FPS for GJVIS6**, versus
57.55 and 49.97 before the final performance fixes. The player targets 60 FPS;
these are measured averages on this machine, not a guarantee for every device.

The per-scene acceptance record is
[`evidence/hd2d-story-audit.json`](../evidence/hd2d-story-audit.json). The final
machine-readable report is
[`evidence/hd2d-story-final.json`](../evidence/hd2d-story-final.json).

- The complete release suite checks timeline holds, source composition, new-art
  coverage, contacts, ending continuity, alpha blending, framing and pacing.
- Continuous rendering exercised all **65,883 timeline ticks** across **41 scenes**.
- The browser review produced **860 action checkpoints** and **40 background/aspect
  compositions**. These supplement the native visual review; counts alone do not
  establish that a story is correct.
- Native SDL playback covers every browser entry. Targeted ultrawide rechecks cover
  the ship, ending, palm climbs and remaining supporting scenes. Logs preserve
  measured FPS, audio-device availability, queued cues and process exit status.
- Live controls check pause, all three styles, atmosphere, all five aspect settings,
  both audio sources, scene navigation and restart. Paused round-trip comparisons
  exclude the desktop notification area.
- Original and Storybook reference captures are checked byte-for-byte.
- Asset checks cover all 614 frame caches, 10 backgrounds, 30 registered panoramas
  and 23 PCM16/44.1 kHz mono audio files.

```sh
cargo test --release
CARGO_MANIFEST_DIR="$PWD" rustc --edition=2021 -O -Awarnings tools/monitor_hd2d_motion.rs -o target/monitor-story
target/monitor-story captures/story-telemetry --all --no-video --limit=14400
CARGO_MANIFEST_DIR="$PWD" rustc --edition=2021 -O -Awarnings tools/review_hd2d_scenes.rs -o target/review-hd2d-scenes
target/review-hd2d-scenes
python3 tools/verify_hd2d_native.py --out captures/native-review
python3 tools/verify_hd2d_native.py --scene GJVIS6 --scene THEEND --aspect 43:18 --resolution 3440x1440 --out captures/native-wide
python3 tools/verify_hd2d_controls.py
```

Native verification requires the local Hyprland session, SDL2 and `grim`; it opens
and captures only its own child player window. Its screenshots are review samples,
not a claim of exhaustive visual correctness at every possible display size.
The renderer remains native Rust/SDL2 software compositing, with optional lighting,
water, smoke/embers, bloom and ACES grading; it does not use a 3D GPU scene.

Generated masters and prompts are preserved in
[`assets/hd2d-v2`](../assets/hd2d-v2/README.md). Earlier motion, static and first-pass
native reports are retained as historical evidence, including the failures that
led to these repairs.
