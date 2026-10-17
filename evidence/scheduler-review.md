# Scheduler correctness review

Scope: `src/scheduler.rs`, the supplied `RESOURCE.001` ADS/TTM data, and the primary DGDS interpreter references. This review changes scheduling and replay state; it does not introduce physics or alter the original authored sprite coordinates.

## Confirmed defects and corrections

1. **Held frames inherited their final drawing state.** The compiler replayed a held frame after restoring the actor's state *after* executing that frame. If the source changed colors, clip, brush or bitmap selection after a draw, the next repaint could change or hide that earlier draw. Held replay now starts with a snapshot of the frame-entry drawing context, followed by the original ordered drawing/state commands. The supplied streams contain numerous state changes after drawing, including `WOULDBE.TTM` frame 102 (`SET COLORS`) and `MJRAFT.TTM` frame 35 (`SELECT BMP`). The test renders a held source frame whose late color/clip change would previously recolor or hide its earlier pixel.

2. **Time-limited actors stopped at their first PURGE.** Negative ADS run counts mean repeat the actor until its time cut. The old compiler completed the actor on PURGE even before that deadline. `finish_due` now repeats it while the cut remains in the future. A genuine time cut retains the current sequence position; `ADD 0x2005` resumes it. By contrast, ordinary PURGE completion rewinds to the sequence's start even if its FINISHED event has already been consumed. This follows the distinct PURGE and time-cut paths in `ADSInterpreter::run`.

3. **WHILE fell through immediately.** `ACTIVITY.ADS` segment 7 uses `WHILE RUNNING (env 4, seq 5)` before starting seq 22. The previous evaluator treated WHILE as an ordinary IF and started the successor during the same tick. The evaluator now retains a continuation at WHILE, yields when the body completes, and resumes after the loop when the actor stops. This prevents overlapping mutually exclusive actor states in that branch.

4. **Explicit draw-order instructions were ignored.** Four `0x4000` operations in `BUILDING.ADS` move environment 3, sequence 44 to the end of the draw list. The compiler previously reconstructed its sorted order every tick, discarding this instruction. A persistent order now follows source sequence order and supports both ADS ordering opcodes. The behavior follows the reference's exact list operations: `0x4000` appends, `0x4010` prepends.

5. **ADS segments ran in numeric order.** The archive's first ACTIVITY segment IDs are `1, 12, 11`, not sorted order. The parser now retains source order. A segment activated earlier in that order can run later in the same tick, rather than always waiting until the next tick. Segment self-stop (`F010 -1`) now immediately ends evaluation, preventing later rules in the same segment from starting additional actors.

6. **Pause had been treated as stop.** Paused actors now retain their held drawing, while PAUSED/NOT PAUSED and NOT RUNNING conditions distinguish pause from stop. This completes the supported state semantics; the supplied ADS streams do not currently use the pause instruction.

## Validation

Nine scheduler tests pass in release mode. They cover exact positive repeat counts; GOTO plus a timed cut; repeating PURGE before a timed deadline; resuming from a cut position; restarting after an ordinary completion; stable held color/clip rendering; WHILE continuation; ordering and immediate segment self-stop; archive source ordering; and drawable timelines for all 41 scene entries.

The full scene timeline harness additionally renders every compiled frame of all 41 supplied scene entries without a panic. Draw counts and caps are recorded in [scene-review.tsv](scene-review.tsv); temporary detailed traces were also saved under `/tmp`.

## Sources and limitations

Primary semantic references:

- [ADS interpreter](https://github.com/scummvm/scummvm/blob/master/engines/dgds/ads.cpp): `run`, `updateSeqTimeAndFrame`, `logicOpResult`, `handleLogicOp`, and ordering/start/stop operation handlers.
- [TTM interpreter](https://github.com/scummvm/scummvm/blob/master/engines/dgds/ttm.cpp): frame-local state mutations and the `_executed` guards separating one-shot operations from held drawing.

The original Windows executable's complete global itinerary remains unrecovered. The browser still selects a relevant ADS segment and caps a compiled timeline at 2,400 original scheduler ticks. Four directly launched TTM resources still use a bounded local fallback. The seeded random path and midpoint random delays remain deterministic browser policies. Exact wall-clock speed and the reference interpreter's end-of-tick advancement boundary were not changed by this review. No physical collision system exists in the source scripts; interactions are authored frame placements and should be assessed through playback of the original runtime before claiming collision equivalence.

## Follow-up: transition replay and scene selection

- Wipe opcodes `A010`–`A090` are no longer cached in a held actor's drawing replay. This is an explicit modern presentation policy: the frontend animates each wipe for 350 ms, whereas the reference's repeated wipe merely copies an already-composed surface. A held replay must therefore keep drawing its actor without restarting that timed animation when another actor changes the framebuffer. A regression executes two new source frames with three-tick holds: each emits its wipe once, while the sprite remains visible on all six scheduler ticks.
- Before this review, the default deterministic `GJGULIVR.TTM` browser route selected `BUILDING.ADS` segment 3. That segment does not reference the two wipe-bearing states. Increasing the timeline bound from 2,400 to 12,000 ticks in a temporary diagnostic copy still produced zero wipe commands or presentation events. The absence was caused by branch selection, not just the timeline cap.
- The browser now scores candidate segments by **distinct referenced sequence IDs**, rather than the number of start instructions. Segment 3 contains 27 starts but only 15 distinct states; segment 4 covers 18 distinct states and includes both wipes. This changes only the Gulliver selection among the supplied ADS-backed browser entries. The new route emits left-to-right at scheduler tick 981 and right-to-left at tick 1843, once each. The archive regression verifies that both original wipe directions occur in the selected timeline. This is a documented browser policy, not a claim to have recovered the Windows application's global branch choice.
- `GFFFOOD.TTM` and `FISHWALK.TTM` deliberately select the same `FISHING.ADS` segment 5: it starts environment 5 sequence 19, then starts environment 3 sequence 1 when walking finishes. The saved contact sheet visibly includes John, shark and swimming frames. The initial “background only” concern was withdrawn after reopening the artifact; no unsupported playback change was made.
- `GJVIS6.TTM` explicitly authors its red ending. Raw frames 140–154 fill persistent color-12 rectangles across the screen from x282 through x624, with y43 and height302. Frame 155 repeats the final strip, frees the sprite sets and ends the actor. The red field is supported by the source commands, so this review does not remove it.

- `MJSAND.TTM` explicitly loads its apparent background props: raw frame 128 (sequence 35) loads `SHIPS.BMP` into slot 4 and draws entering ship frames 7/3; raw frame 185 redraws SHIPS frames 9/18 and jumps to label 37; raw frame 187 loads `GJBIPLAN.BMP` and draws the aircraft. Their presence is supported by the scene's own script, so this review does not remove them as supposed state leakage.

The follow-up scheduler suite has nine passing tests. The final release harness renders all 41 scene timelines without a panic, and the Gulliver event trace confirms exactly two presentation events at ticks 981/1843.
