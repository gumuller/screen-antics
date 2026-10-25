# HD-2D standalone rebuild record

The active rebuild replaced the experimental flat mode-3 importer with
`src/hd2d_scene.rs`. See `docs/HD2D.md` for current behavior and verification.

## Root causes established

- The supplied generator replaced only twelve individual frames across nine
  atlases; 107 atlases were identical to Storybook. The validator checked file
  geometry, not complete new animation artwork.
- New ISLAND2 and ISLETEMP backgrounds were identical, whereas the original
  islands/palms differed by 272 logical pixels.
- New sand, horizon, palm curvature and beach shoreline did not match native
  choreography. Original feet around y310 were over water in the new scene.
- Night/ocean exports contained unintended outside-photo padding.
- Several sheets mix unrelated objects: GJBIPLAN includes rowing boats/crew,
  SSUZY3 includes Johnny, bubbles and props, and SMDATE11 includes tails/limbs.
  Whole-sheet substitutions produced wrong objects and duplicate complete actors.

## Result

- A separate catalog of 152 clips/poses, 232 transparent extracted frames and
  fifteen sprite masters plus a clean ending backdrop. No Original/Storybook sprite texture fallback.
- All 116 source families/2,391 source frames have explicit replacement, integrated
  component, painted scenery or procedural effect roles. Current 41-scene playback
  exercises 105 source families; catalog coverage tests include the remainder.
- New contacts for sand, curved palm, fishing, an airborne dive into open water,
  ship waterlines/terrain depth, diagonal beach and seated office work.
- Head/arm/body parts coalesce into whole characters; paired interactions suppress
  solo companions; table/gramophone roles remain separate. Walk phase follows
  distance. Bath and dive key poses follow action state rather than unrelated loops.
- All ten core/30 panoramic arrangements use their own world transform; unintended
  night/ocean padding is removed, framed scenes stay framed, sprites can cross
  the old stage edges. All 23 audio files are retained.
- Effects receive a new-world actor/terrain visibility mask and new fire positions.

## Evidence

`evidence/hd2d-motion-review.json` summarizes release tests, continuous simulation, native
controls, FPS and unchanged Original/Storybook reference outputs. Captures and
logs live under `captures/motion-*`; source art surveys are retained under
`captures/hd2d-source-survey`.

This is a semantic reinterpretation with complete cycles and key poses, not a
one-for-one restoration of all 2,391 original limb/facial variants. The prior
`evidence/hd2d-integration.json` describes the superseded flat import and is not
validation of this rebuild.

## Motion follow-up

The user rejected the initial static verification. Continuous 60-Hz comparisons
exposed bbox-driven facing flips, sparse-key root jumps, inappropriate looping of
key poses, mixed-sheet role errors and the baked-in ending card. The motion layer,
new gait/gag poses and ending director address these separately. The old static
report is explicitly superseded; motion evidence and native playback are required
for the current result.
