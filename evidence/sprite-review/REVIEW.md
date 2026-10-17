# Sprite reconstruction audit

All 116 original sheets / 2,391 frames were decoded and audited. All 96 generated replacement frames across JOHNWALK, MJJOG2 and MJREAD were compared with their original frame dimensions, native alpha coverage, centroid, silhouette IoU, best same-sheet silhouette match, mean RGB and distinct generated RGB count. Thirty paginated original contact sheets cover every source frame. Eight paired contact sheets show every replacement alongside the original at common scale and registration. Source archive SHA-256 and precise review coverage are recorded in `review-coverage.json`.

Visual inspection is now complete for all 30 original contact pages (all 2,391 source frames across all 116 sheets) and all eight paired pages (all 96 replacements). `visual-page-review.json` records each page, frame range and findings. This is comprehensive visual review at contact-sheet scale; it is not a claim of per-pixel full-resolution inspection or proof of every animation-script composition.

## Verdict

The generated art is a visual reinterpretation. It is **not an exact pose-faithful reconstruction**, even though all 96 packs have the correct original frame dimensions multiplied by four. Fitting a generated silhouette into the original bounding box preserves placement but cannot recover internal joint angles, gait phase, hand-held props, clothing, or the order of book-page gestures.

Mean alpha silhouette IoU is 0.758; median is 0.775. Eight frames score below 0.65 and three below 0.45. Two frames match another same-sheet source silhouette at least 0.10 better than their intended source. These are screening metrics, not a semantic pass criterion: several obvious gesture mismatches score above 0.80 because the seated torso dominates their shape.

## Concrete findings

- **JOHNWALK #013:** Original 8×1 frame contains a tiny nontransparent white mark. Generated replacement is empty. Earlier provenance describing this as intentionally empty was incorrect. This is the sole empty/nonempty mismatch.
- **JOHNWALK #007:** Original stride is extended; replacement bends the rear leg differently. IoU 0.602 and nearest silhouette is source #004. Other walking frames preserve broad facing/action categories but approximate foot placement and knee angles.
- **MJJOG2 all 24 frames:** Generated character wears the everyday floral sleeveless shirt and shorts. The recovered source has a gray long-sleeved jogging outfit and long trousers. Running and facing directions broadly match, but costume and some gait phases do not. #017 is a conspicuous leg/torso difference (IoU 0.601).
- **MJREAD #002:** Source holds the cap above the head; replacement touches a cap worn on the head.
- **MJREAD #006:** Source tilts the head conspicuously; generated head remains upright.
- **MJREAD #012–018 and #022:** Original raises/turns book pages through distinct positions. Generated frames repeat a mostly stable open-book reading pose. Their high silhouette scores do not validate this animation motion.
- **MJREAD #023:** Source is bareheaded, with cap artwork provided separately as #024. Replacement wears a cap, introducing a duplicate-hat risk when the script combines layers. Source #024 and #025 are cap object frames and were retained as objects.
- **MJREAD #029:** Source remains in a low kneeling pose; replacement transitions into a higher rising/crouching pose. Registration alone does not correct the joint change.
- **MJREAD #031–034:** Water effects preserve broad splash-to-ripple progression, but locations and connected components differ substantially. IoUs are 0.598, 0.546, 0.347 and 0.099. Sparse ripple #034 is particularly unsuitable for judging quality by area overlap alone.

The preceding technical component isolation removed unrelated neighboring cell fragments. It does not correct these semantic differences. Original multipart frames, tiny masks, shadow-only frames, detached heads/limbs, hats, and compositing elements are common throughout the 116-sheet source set; a universal “keep largest component” rule would destroy valid content. Replacements must be evaluated against source frame roles and the animation script.

## Outputs and reproduction

- `manifest.json`: complete original frame inventory and all replacement measurements.
- `sheet-coverage.csv`: all 116 sheets, frame/empty/replacement counts and contact page coverage.
- `replacement-audit.csv`: compact replacement screening metrics.
- `original-page-01.jpg` through `original-page-30.jpg`: all original frames.
- `paired-*.jpg`: all 96 original/generated pairs.
- `original/`: technical SAHD exports preserving original palette RGB and index-zero transparency.

From the project directory, compile `tools/audit_sprites.rs` with Rust 2021 and run it with `original evidence/sprite-review/original`. Then run `python tools/audit_sprites.py` (Pillow required). Auditing reads production assets and writes only audit evidence; it does not modify artwork or renderer behavior.

## Complete original-artwork review

No malformed decoded frame, shifted image stride, broken nibble plane, or unexpected sheet-index displacement was detected in the 30 original contact pages. Character poses, object outlines, wing cycles, cloth/hat transformations and particle sequences progress coherently. All images also pass the decoder's strict dimensions and decompressed-byte-count checks. This supports the resource decoder; it is not an independent binary-equivalence comparison against the Windows renderer.

The following are facts in the recovered source artwork, not new corruption introduced by the Rust decoder:

- Many 8×1/8×2 frames contain tiny black, white or colored marks. Some frames are fully transparent, but a visually negligible frame is not necessarily empty. Preserve the actual indices and pixels.
- Partial heads, torsos, legs, hands, caps, gloves, optic lenses and shadows are individual image frames throughout FIRE, MJBOTTLE, MJCOCO, MJFISH, MJ_AMB, MJTELE and the date/gift sheets. Long straight image boundaries on these pieces do not establish clipping corruption.
- Sparse disconnected water highlights/ripples occur in GJCATCH, GJDIVE, MJDIVE, MJFISH, MJREAD, SPLASH, mermaid and ship sheets. All components matter; largest-component cleanup would delete valid source content.
- MJBOTTLE #000–002 contain opaque blue rectangular water patches. MJDIVE #007–008 and #024–026 include tree/foliage rectangles or fragments. Their pixels are genuinely present in the archive. Interpreting their exact restoration/occlusion role requires script composition; they should not be discarded as extraction artifacts.
- BACKGRND includes independent shoreline strips, trunk and crown; TRUNK has distinct trunk/crown images. THNKBUBL and STNDLAY also embed composed mini-scenes. Matching by asset name, color, or broad geometric masks cannot substitute for draw/capture order.
- Dark parachutes, black tuxedos, black motion streaks and black outlines are visibly intended opaque content. Transparency comes from source index zero; treating every RGB-black pixel as transparent would erase valid art.
- MJJOG1/MJJOG2 source frames clearly show the gray full jogging tracksuit. Ordinary floral-shirt frames elsewhere are a different costume, confirming that the generated jogging sheet changed the original design.

The source images alone cannot prove the exact semantic purpose of every small mark, cropped body, background patch or animation slot. Their presence is confirmed; compositing roles above are inferred from neighboring frames and visible content and must be validated through VM/script behavior. No additional decoder defect was established by this full contact-page inspection.
