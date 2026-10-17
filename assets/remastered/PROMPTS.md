# Remastered background generation

All artwork here was made with the built-in ImageGen tool on 2026-10-17.
Sources are the recovered backgrounds under `evidence/`; final generated PNGs
are retained under `source/`, and runtime RGBA containers under `backgrounds/`.
RGBA conversion is lossless format conversion. Requested dimensions were
advisory: the actual paintings are about 1.6 megapixels; rendering at 4K does
not add newly painted 4K detail.

## Final prompt set

### ISLETEMP.SCR

Use case: style-transfer. Edit target: attached original Screen Antics island background. Create the actual high-resolution 24-bit remastered game background, 2560x1400 landscape. Faithfully REDRAW every element with clean hand-inked cartoon outlines, smooth cel shading, rich tropical colors, detailed but restrained sand, palm fronds and water. Retain the nostalgic early-1990s humorous adventure-game style, but paint clean high-resolution curves and shading instead of chunky original pixels or dithering. CRITICAL register geometry: keep exactly the original composition and camera, horizon at y=0.437 of height, lone palm tree on right with trunk base at x=0.704,y=0.902, island spans x=.43 to.91 near bottom and cloud silhouettes at original positions. Do not move or resize the island/tree; original animated character feet must still land in the same positions. Empty background ONLY, no people, no extra islands, buildings, props or text. Bright turquoise sky, cobalt-to-teal tropical sea with small white wave accents, warm golden sand, lively emerald fronds, reddish brown trunk. Preserve original silhouette locations to allow old sprite coordinates to overlay. No borders, logos, text, black margins. Full bleed rectangular game scene.

### ISLAND2.SCR

Use case: style-transfer. Edit target: supplied original game background. Remake the attached original Screen Antics island-on-LEFT game background in the SAME hand-inked cartoon style as a high-resolution 24-bit adventure-game remaster. Smooth painted cel shading, tropical emerald foliage, warm golden sand, turquoise sky, deep blue and teal waves. Preserve EXACT camera/layout/geometry of source: left island and palm stay in same normalized positions, horizon same height, island at bottom left. No people, extra objects, text or black borders. It is a registered replacement behind animated sprites so preserve palm trunk base and sand outline positions. 2560x1400 full-bleed landscape.

### NIGHT.SCR

Use case: style-transfer. Edit target: supplied original game background. Remake attached Screen Antics night ocean background as high resolution 24-bit clean hand-painted 1990s cartoon adventure-game art, smooth rich cel-shading. Preserve exact layout: small pale moon at original top-left position, rich indigo sky with tiny stars, horizon at original normalized height, rippling moonlight path down left quarter of dark blue teal ocean. No islands, people, boat, text, added props or black margins. Keep original camera and geometry. Full-bleed 1920x1440 landscape.

### OCEAN00.SCR

Use case: style-transfer. Edit target: supplied original game background. Remake attached original ocean-only game scene as high-resolution 24-bit hand-painted tropical cartoon background matching the island remaster style. Empty turquoise sky above horizon same normalized height as reference; rich cobalt and teal water below, gently layered waves with small white foam accents. Clean drawn organic curves and restrained painted cel shading, no pixels or dithering. NO island, trees, characters, boats, text, border or extra objects. Full bleed 1920x1440 landscape, source composition preserved.

### JOFFICE.SCR

Use case: style-transfer. Edit target: attached original game background. Remaster attached office scene in high-resolution 24-bit hand-inked cartoon adventure-game art. Keep source room's exact position and dimensions inside black surround, exact perspective, yellow wooden desk, chair, tall stack of papers on small left table, round wallclock and skyscraper window. Rich but tasteful color, painted cel shading, clean linework rather than dithering/pixels. No people, extra objects, text or borders beyond existing intentional black surround. Original coordinates must remain valid for character sprites. 1920x1440.

### SUZBEACH.SCR

Use case: style-transfer. Edit target: attached original game background. Remaster attached game background in high-resolution 24-bit hand-inked cartoon adventure-game art. Keep EXACT rectangular beach inset position and size inside black surround. Turquoise sky and sea, distant city skyline on right island at horizon, small foreground sandy shore and green foliage at bottom left. Preserve camera, horizon, shoreline silhouettes and normalized positions for sprite registration. Rich painted cel shading, crisp curves, no pixels, no characters or added objects/text. 1920x1440.

### THEEND.SCR

Use case: style-transfer. Edit target: attached original game background. Remaster attached original sunset end-card background in high-resolution 24-bit painted 1990s cartoon adventure-game style. Keep EXACT original composition and camera: small rectangular scroll-shaped inset surrounded by black, vivid sunset orange/red sky, yellow setting sun left of central palm/island silhouette, golden reflection in blue/purple sea. Keep silhouettes and scroll frame in original normalized positions. Clean inked outlines and smooth painted shading. No text, no characters, no added props. 2560x1400.

### INTRO.SCR

Use case: style-transfer. Edit target: attached original game background. Remaster attached original Johnny Castaway title screen in high-resolution 24-bit hand-painted cartoon adventure-game style, preserve layout and original lettering. Keep the original central scroll panel inside black surround; turquoise sky, tropical sea and palm on right, fisherman standing on small sandy island and shark fin on left. Exact readable text: 'SCREENANTICS!' at top, 'JOHNNY CASTAWAY' large yellow title on left and 'SIERRA ©1992' lower right. Preserve same original composition and fun character style. Smooth clean drawn contours and rich cel shading instead of dithering, no added elements or text. 1920x1440.

### OCEAN01.SCR / OCEAN02.SCR

Each call used the remastered OCEAN00 image and its respective original phase as references. `PHASE` was respectively `second` and `third`:

Use case: lighting-weather. First image is the actual remastered ocean game background. Second image is the original PHASE water animation phase. Make the PHASE matching high resolution ocean animation frame. Keep EXACT same camera, horizon height, turquoise sky color, blue/teal palette, wave sizes and clean hand-painted cartoon style as first image. Only shift and reshape the ocean wave crests subtly to create gentle rolling water, borrowing phase from second reference. No added clouds, boats, islands, characters, text or margins. Full bleed 1920x1440 landscape. Preserve consistency with first image so frames can crossfade seamlessly.

### ISLETEMP.SCR.wide / ISLAND2.SCR.wide

Each call used its corresponding painted island background as reference:

Use case: compositing. Edit target: attached finished game background. OUTPAINT into an ultrawide 21:9 3440x1440 panoramic game background. Keep the entire existing image centered, at full existing height, preserve EVERY existing tree, island, horizon, cloud in the CENTER portion unchanged in scale and height. Add ONLY matching sky and ocean to left and right to expand canvas horizontally. No duplicate island or tree. Keep horizon perfectly straight and exactly at reference height. Continue water waves and turquoise sky naturally without any seams, stripes, edge repetitions, tiling, mirrored water, new objects or text. Clean painted cartoon style identical to reference. The reference occupies central76.5% of output width, with11.75% newly painted extension on each side. Tree for right-island scene should end up near x=.656, for left-island scene x=.335. Output ultrawide aspect21:9, not16:9.

See [sprite prompt set](sprites/PROVENANCE.md) for the three actual sprite redraws,
frame registration manifests, extraction cleanup and fallback distinctions.
