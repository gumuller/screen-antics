# Scene contact-sheet review

Visually reviewed every one of the 41 scene contact sheets in this directory: eight sampled full-scene captures and eight corresponding crop panels per scene, totaling 328 scene stills and 328 crop views. This covers the supplied source-faithful HD/pixel-provenance captures. It does not establish frame-by-frame equivalence with the original Windows program, validate unsampled transitions, or prove temporal animation behavior.

## Findings and script validation

1. **GJVIS6.TTM #0793:** The large red field is explicitly authored by raw frames 140–155: persistent color-12 rectangles span y43/height302 while moving across the screen. It is not a newly introduced HD occlusion error. The original Windows application's final scene handoff remains unverified. See [scheduler evidence](../scheduler-review.md).
2. **FIRE.TTM:** Most samples are background-only, with tiny ember/flame/smoke elements appearing in later samples. This looks like a supplemental fire-effect clip; it should not be described as a complete castaway scene without checking how the original scheduler uses it.
3. **MJAMBWLK.TTM:** The actor appears in the same standing position in all eight widely separated samples. This may be regular sampling aliasing a short loop or a waiting state; it is not proof of a freeze.
4. **MJSAND.TTM #1000–2334:** Ship, flags, raft and aircraft are loaded/drawn by this scene itself (raw frames128/185/187). These are authored props, not evidence of generated atlas fragments. Exact original global handoff/clearing behavior remains unverified; no unsupported removal was made. See [scheduler evidence](../scheduler-review.md).

## Checks that passed in the sampled views

No obvious duplicate castaway, repeated detached body fragment, generated-atlas toe fragment, floating reading-head strip, or large unintended rectangular water/tree restoration patch remains visible in the sampled ordinary island scenes. Fishing rods/lines, hats, coconut props, splash phases, leaves, rafts, thought bubbles, tables/chairs and secondary characters appear with coherent source-style silhouettes. The gray tracksuit is preserved in MJJOG. MJDIVE and related climbing scenes show the actor interacting with the trunk/crown without the earlier generated-sprite limb mismatch.

Left/right island layouts vary by scene. The fixed crop region misses or truncates actors/props for GJVIS5, GJVIS5W, MJFISHC and SJLEAVES; the corresponding full-scene panels show the actors at the expected left-side island. This is a review-crop limitation, not evidence that the runtime clipped away those actors. Thought bubbles and entering ships also extend beyond crop boundaries while remaining visible in the full-scene panels.

Office, city-beach and ending scenes use a small composed source scene centered on a black field. Their black margins are visible in the source artwork and are not blank-scene failures. Dark tuxedos, silhouettes and parachute content remain opaque in the reviewed samples. THEEND's red sunset background is intentional source content and is distinct from the GJVIS6 solid-red rectangle candidate.

## Per-scene coverage

| Contact sheet | Sampled visual result |
|---|---|
| FIRE.TTM | Supplemental tiny fire/smoke elements; mostly background-only samples. |
| FISHWALK.TTM | Actor, shark, leap and seated aftermath coherent. |
| GFFFOOD.TTM | Reopened and verified: John, shark, swim/leap and seated aftermath visible; no blank-clip finding. |
| GJCATCH2.TTM | Fishing, fish/octopus and tree interaction coherent. |
| GJDIVE.TTM | Dive, scoring props and tree interaction coherent. |
| GJGULIVR.TTM | Final revised branch: sleeping actor, miniature crew/ships, disappearance/reappearance stages and subsequent action; both wipes traced at ticks981/1843. |
| GJGULL1.TTM | Gull approach/perching and actor interaction coherent. |
| GJHOT.TTM | Fan/heat/melting poses coherent. |
| GJLILIPU.TTM | Ship movement and small figures coherent in sampled views. |
| GJNAT1.TTM | Mask/leaf costume sequence coherent; no duplicate actor seen. |
| GJNAT3.TTM | Boat and costume-removal sequence coherent. |
| GJVIS3.TTM | Telescope and motorboat action coherent. |
| GJVIS5.TTM | Left-island actor/aircraft coherent; crop misses much action. |
| GJVIS5W.TTM | Left-island running/aircraft coherent; crop misses much action. |
| GJVIS6.TTM | Ordinary samples coherent; red ending confirmed in source commands. |
| MEANWHIL.TTM | Clock, office and thought bubbles coherent; black field intentional. |
| MJAMBWLK.TTM | Same standing pose in every sample; sampling/wait-state caveat. |
| MJBATH.TTM | Water torso, hat/rod and gull action coherent. |
| MJCOCO.TTM | Coconut handling and crouching phases coherent. |
| MJCOCO1.TTM | Coconut toss, shadow and actor phases coherent. |
| MJDIVE.TTM | Tree climb/dive/submerged phases coherent. |
| MJFIRE.TTM | Wood/fire/actor action coherent in sampled views. |
| MJFISH.TTM | Rod bending, fish and actor action coherent. |
| MJFISHC.TTM | Mirrored/left-island fishing coherent; crop miscenters actor. |
| MJJOG.TTM | Gray tracksuit, stretching and running preserved. |
| MJRAFT.TTM | Paddle/raft-building body interactions coherent. |
| MJREAD.TTM | Seated/book and gull/cap phases coherent; no stray strip seen. |
| MJSAND.TTM | Coherent recognizable props; ship/flag/aircraft drawings confirmed in the source script. |
| MJTELE.TTM | Telescope and actor silhouettes coherent. |
| SASKDATE.TTM | Mermaid, thought props and actor interaction coherent. |
| SBREAKUP.TTM | Raft/mermaid/thought/castaway sequence coherent. |
| SHARK1.TTM | Shark, pipe, hat and seated actor action coherent. |
| SJGLIMPS.TTM | Fishing/mermaid glimpse and thought markers coherent. |
| SJLEAVES.TTM | Raft/leaves/shark/mermaid action coherent; crop miscenters left island. |
| SJMSSGE.TTM | Bottle/letter and thought bubble action coherent. |
| SJMSUZY.TTM | Beach, raft and two-character interaction coherent; intentional black margin. |
| SJWORK.TTM | Office and daydream composition coherent; intentional black margin. |
| SMDATE.TTM | Tuxedo/mermaid/table/chair/gramophone action coherent. |
| SUZYCITY.TTM | Beach/swim/thought composition coherent; intentional black margin. |
| THEEND.TTM | Clock, sunset/parachute and title coherent; intentional black margin. |
| WOULDBE.TTM | Yacht, castaway and secondary-character action coherent. |

Only supplied captures were reviewed. There is no claim that all eight stills imply an entire animation or scheduler is correct.

## Corrected observation

The current GFFFOOD contact sheet was reopened after a conflicting initial observation. It clearly contains John at #0001/#0105, a shark at #0209, swimming/leaping at #0418 and seated aftermath at #0523/#0627. The initial background-only observation is withdrawn and must not be treated as a runtime defect. The associated FISHING.ADS sequence deliberately joins FISHWALK and GFFFOOD according to the animation-script review.

Final Gulliver contact sheet was refreshed after branch selection was corrected and visually rechecked. The other forty contact sheets retain their reviewed source timeline.
