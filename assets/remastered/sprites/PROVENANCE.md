# Sprite artwork provenance

`johnwalk-remade-atlas.png` was created using the built-in ImageGen tool from the recovered `evidence/JOHNWALK.BMP.png` pose sheet on 2026-10-17. This is newly generated painted artwork. Its 36 cell positions were requested to follow the recovered original pose sequence. The complete review found pose mismatches. Frame 13 was incorrectly made empty: the recovered 8×1 frame contains a tiny opaque white mark.

`JOHNWALK.BMP/*.rgba` are technical crops of that generated atlas, fitted into the original nontransparent bounds at four times the original dimensions. `manifest.json` records every frame, original bounds, and atlas crop. Frame positions and timings remain controlled by the original animation scripts. Generated poses approximate the original animation and are not a verified exact reconstruction of every contour.

Additional imagegen redrawn sheets: `MJJOG2.BMP` (24 frames) and `MJREAD.BMP` (36 frames). Their original contact sheets supplied the pose/index reference and the first generated JOHNWALK atlas supplied the character/style reference. Both generated atlases are persisted alongside the first atlas. Individual frame SAHD files and registration manifests are in their named sprite directories. MJREAD frame 35 isolates the generated yellow star from an adjoining ripple in the generated atlas.

The default renderer now uses all 2,391 original decoded poses with deterministic 4× contour-aware enlargement and local true-color material filtering. Local filtering avoids color flicker caused by the earlier frame-size-dependent light ramp. These are enhanced originals, not redrawn artwork. The generated pack is retained only behind `--painted-art`, an experimental option with documented pose and geometry mismatches. No external assets are downloaded.

See [the complete audit](../../../evidence/sprite-review/REVIEW.md): all 116 original sheets and all 96 generated frames were visually reviewed. Matching dimensions and bounding boxes did not establish matching gestures, costumes or separate props.

## Exact ImageGen prompt

Use case: style-transfer. Edit target: attached Screen Antics Johnny Castaway sprite sheet, 520 by 328 original. Remake this EXACT entire sheet as detailed high-resolution hand-painted 2D cartoon animation sprites. Preserve EXACT original grid: ten columns, four rows, last row six sprites, ALL 36 original poses in EXACT order and directions, matching the placement and silhouette and clothing for each pose. The man is a shaggy black-bearded stranded castaway with a white sailor cap, red tropical patterned sleeveless shirt, white shorts and bare feet. Keep funny scruffy appealing 1990s adventure cartoon character identity. Each sprite isolated, transparent background. Output canvas EXACT 2080 by1312, exactly four times original; each original 52 by82 grid cell becomes208 by328. Sprites must remain inside their own cells, anchored at same locations scaled4. Crisp clean ink contours, smooth painted skin shading, fabric texture, expressive facial detail, rich unrestricted 24-bit colors. No grid lines, no text, no labels, no added props, no ground shadows between cells. This is a production replacement sprite atlas: all poses/cells and transparent margins must match source exactly. Do not simplify into character concept art; preserve the entire exact original 36frame sequence.

Tool option: transparent_background=true. Requested dimensions were advisory; generated dimensions were 1580×995. Alpha was preserved when slicing and resizing.

## Technical isolation QA

`tools/isolate_sprite_frames.py` reproduces extraction from preserved generated atlases. Human cells use a slightly expanded source ROI followed by largest connected alpha silhouette isolation before bounding-box registration. This removes adjacent-cell fragments without clipping limbs that extend past the advisory generated grid. Tiny object cells use their nominal cells; water effects retain multipart alpha. Per-frame manifests record extraction boxes and discarded-component counts. Original frame sizes and opaque registration bounds remain authoritative.
