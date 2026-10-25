# Standalone HD-2D scene assets

Runtime option 3 loads this catalog through `src/hd2d_scene.rs` and the authored
sequences in `src/hd2d_story.rs`. The ending has its own director.

- 253 clips/poses, 614 transparent frames, 54 sprite masters.
- 10 core backgrounds and 30 registered panoramic variants.
- Three continuous panorama masters repair the supplied day/night/ocean seams.
- 23 remastered PCM16 / 44.1 kHz sound files.
- No Original or Storybook sprite texture is drawn in this mode.

`masters/` preserves generated artwork and per-master prompts; `prompts/` holds
the initial shared production prompts. All new raster art was made with the
built-in image-generation tool. Rejected panorama candidates are retained as
references and are not consumed by the build.

`tools/prepare_hd2d_scene_assets.py` extracts complete alpha components and
registers anchors without resizing individual poses. It derives consistent
background crops and emits JSON/TSV plus SAHD caches. `Play.sh` can regenerate
missing caches from the durable clip/background PNGs.

See `docs/HD2D.md` for staging and controls, and `docs/HD2D_STORY_REVIEW.md` for
scenario review and native validation evidence.
