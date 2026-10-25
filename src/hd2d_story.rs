//! Authored actions for stories whose recovered body-part instructions cannot
//! express the new complete-pose drawings. Time is the compiled ADS tick, so
//! seeking, pausing, replaying and display refresh do not change the story.
use crate::hd2d_layout::{Layout, Point};

#[derive(Clone, Debug)]
pub struct Layer {
    pub clip: &'static str,
    pub phase: usize,
    pub point: Point,
    pub scale: f32,
    pub flip: bool,
    pub shadow: bool,
    pub waterline: Option<f32>,
    pub depth: Option<f32>,
}
#[derive(Clone, Debug)]
pub struct FishingLine {
    pub actor: usize,
    pub end: Point,
    pub taut: bool,
    pub in_water: bool,
}
#[derive(Default, Debug)]
pub struct Frame {
    pub beat: &'static str,
    pub black_background: bool,
    pub layers: Vec<Layer>,
    pub line: Option<FishingLine>,
    pub splash: Option<(Point, f32)>,
    pub smoke: Option<Point>,
    pub scores: Vec<(usize, &'static str)>,
    pub lightning: Option<(Point, Point)>,
    pub projectiles: Vec<Point>,
    pub ropes: Vec<(Point, Point)>,
}
fn mix(a: f32, b: f32, u: f32) -> f32 {
    a + (b - a) * u.clamp(0., 1.)
}
impl Frame {
    fn add(&mut self, clip: &'static str, phase: usize, x: f32, y: f32, flip: bool) {
        self.layers.push(Layer {
            clip,
            phase,
            point: Point::new(x, y),
            scale: 1.,
            flip,
            shadow: clip.starts_with("johnny-") && clip != "johnny-firecraft",
            waterline: None,
            depth: None,
        });
    }
    fn walk(&mut self, t: f32, start: f32, end: f32, duration: f32, pole: bool) {
        let x = mix(start, end, t / duration);
        let distance = (x - start).abs();
        self.add(
            if pole {
                "johnny-pole-walk"
            } else {
                "johnny-walk"
            },
            (distance / 5.0) as usize % 8,
            x,
            258.,
            end < start,
        );
    }
}

pub fn frame(scene: &str, tick: usize) -> Option<Frame> {
    let t = tick.saturating_sub(1) as f32 / 60.;
    match scene {
        "MJFISH.TTM" => Some(fishing(t, false)),
        "MJFISHC.TTM" => Some(fishing(t * (32.5 / (1690. / 60.)), true)),
        "MJFIRE.TTM" => Some(fire(t)),
        "MJRAFT.TTM" => Some(raft(t)),
        "SHARK1.TTM" => Some(shark(t)),
        "GJDIVE.TTM" => Some(diving(t)),
        "GJGULIVR.TTM" => Some(gulliver(t)),
        "GJNAT1.TTM" => Some(rain_dance(t)),
        "GJHOT.TTM" => Some(heat(t)),
        "GJNAT3.TTM" => Some(mask_boat(t)),
        "MJDIVE.TTM" => Some(solo_diving(t)),
        "GJCATCH2.TTM" => Some(octopus_catch(t)),
        "MJREAD.TTM" => Some(reading(t)),
        "MJJOG.TTM" => Some(jogging(t)),
        "MJCOCO.TTM" => Some(coconuts(t)),
        "MJTELE.TTM" => Some(spyglass(t, false)),
        "GJVIS3.TTM" => Some(spyglass(t, true)),
        "GJGULL1.TTM" => Some(gull_visit(t)),
        "GJLILIPU.TTM" => Some(lilliput_ship(t)),
        "FIRE.TTM" => Some(fire_helper(t)),
        "MJBATH.TTM" => Some(bathing(t)),
        "MJCOCO1.TTM" => Some(coconut_throw(t)),
        "FISHWALK.TTM" | "GFFFOOD.TTM" => Some(shark_gulp(t)),
        "GJVIS6.TTM" => Some(close_ship(t)),
        "MJAMBWLK.TTM" => Some(ambient(t)),
        "GJVIS5.TTM" => Some(plane_rescue(t)),
        "GJVIS5W.TTM" => Some(plane_rescue(t * 789. / 982.)),
        "MJSAND.TTM" => Some(sandcastle(t)),
        "MEANWHIL.TTM" | "SJWORK.TTM" => Some(office(t)),
        "SJMSSGE.TTM" => Some(message_bottle(t)),
        "SASKDATE.TTM" => Some(mermaid_invitation(t)),
        "SBREAKUP.TTM" => Some(mermaid_breakup(t)),
        "SJGLIMPS.TTM" => Some(mermaid_glimpse(t)),
        "SUZYCITY.TTM" => Some(suzy_city(t)),
        "SJLEAVES.TTM" => Some(raft_departure(t)),
        "SJMSUZY.TTM" => Some(suzy_reunion(t)),
        "SMDATE.TTM" => Some(mermaid_date(t)),
        "WOULDBE.TTM" => Some(party_rescue(t)),
        _ => None,
    }
}

/// Recovered SFX numbers, retimed to the authored action rather than the old
/// body-part command timestamps. None preserves the source scene's audio.
pub fn sound_events(scene: &str, tick: usize) -> Option<Vec<u16>> {
    let mut result = Vec::new();
    let mut cue = |seconds: f32, sfx: u16| {
        if tick == (seconds * 60.).round() as usize + 1 {
            result.push(sfx);
        }
    };
    match scene {
        "MJFISH.TTM" | "MJFISHC.TTM" => {
            let rate = if scene == "MJFISHC.TTM" {
                32.5 / (1690. / 60.)
            } else {
                1.
            };
            for (start, speed) in [(3., 1.), (16.5, 13.5 / 12.5)] {
                for (local, sfx) in [
                    (0.45, 6),
                    (2.4, 4),
                    (3., 16),
                    (7., 5),
                    (8.2, 8),
                    (9.3, 3),
                    (11.5, 4),
                    (12.5, 16),
                ] {
                    cue((start + local / speed) / rate, sfx);
                }
            }
            for i in 0..4 {
                cue((29.1 + i as f32 * 0.8) / rate, 7);
            }
        }
        "MJFIRE.TTM" => {
            for (time, sfx) in [
                (3., 23),
                (5., 23),
                (7., 23),
                (15., 15),
                (16., 15),
                (17., 15),
                (18., 15),
                (19.8, 9),
                (31.6, 9),
                (37.3, 6),
                (37.5, 19),
                (38., 19),
                (39.1, 5),
                (39.4, 3),
                (40.1, 3),
                (46.5, 9),
                (48.5, 9),
            ] {
                cue(time, sfx);
            }
        }
        "MJRAFT.TTM" => {
            for (time, sfx) in [
                (0.3, 6),
                (0.9, 6),
                (2.5, 15),
                (2.8, 15),
                (3.1, 15),
                (3.5, 9),
                (5.2, 23),
            ] {
                cue(time, sfx);
            }
        }
        "SHARK1.TTM" => {
            for (time, sfx) in [(7., 5), (9.3, 1), (10.6, 9), (13., 16), (17., 9)] {
                cue(time, sfx);
            }
        }
        "GJDIVE.TTM" => {
            for i in 0..3 {
                let start = i as f32 * (3172. / 60. / 3.);
                for (time, sfx) in [
                    (6., 14),
                    (7., 8),
                    (8.7, 1),
                    (10.2, 19),
                    (10.8, 16),
                    (11.2, 16),
                    (11.6, 16),
                ] {
                    cue(start + time, sfx);
                }
            }
        }
        "GJHOT.TTM" => {
            for (time, sfx) in [(2., 24), (10., 9), (12., 16)] {
                cue(time, sfx);
            }
        }
        "GJNAT1.TTM" => {
            for (time, sfx) in [
                (0.1, 24),
                (6., 9),
                (8.3, 22),
                (8.8, 22),
                (9.3, 22),
                (25., 10),
                (27.5, 19),
                (28.2, 19),
            ] {
                cue(time, sfx);
            }
            for i in 0..32 {
                cue(11. + i as f32 * 0.4, 6);
            }
        }
        "GJNAT3.TTM" => {
            for i in 0..12 {
                cue(i as f32 * 0.4, 6);
            }
            for (time, sfx) in [(5., 5), (7., 8), (9., 19), (28., 24)] {
                cue(time, sfx);
            }
        }
        "GJGULIVR.TTM" => {
            cue(2., 9);
            cue(3.5, 24);
            for i in 0..8 {
                cue(13.2 + i as f32 * 0.45, 16);
                cue(28.2 + i as f32 * 0.45, 16);
            }
            for i in 0..5 {
                cue(34.6 + i as f32 * 0.3, 21);
            }
            cue(37., 4);
        }
        "MJDIVE.TTM" => {
            let rate = 3172. / 3. / 901.;
            for (time, sfx) in [(6., 14), (7., 4), (8.7, 1), (10.8, 16), (11.2, 16)] {
                cue(time / rate, sfx);
            }
        }
        "MJREAD.TTM" => {
            for time in [4.1, 4.8, 6., 7.2, 8.2, 12., 14., 16.5] {
                cue(time, 21);
            }
            cue(17.2, 8);
            cue(19.3, 3);
        }
        "MJJOG.TTM" => {
            for time in [3., 5., 7., 31.2, 31.8, 32.4] {
                cue(time, 9);
            }
            cue(9., 4);
            cue(11., 4);
        }
        "MJCOCO.TTM" => {
            for (time, sfx) in [
                (3.7, 23),
                (4.88, 5),
                (7.24, 5),
                (10.5, 9),
                (12., 4),
                (13.8, 4),
                (14., 23),
                (14.5, 23),
            ] {
                cue(time, sfx);
            }
        }
        "MJTELE.TTM" | "GJVIS3.TTM" => {
            cue(0.3, 9);
            if scene == "GJVIS3.TTM" {
                cue(7.7, 5);
            }
        }
        "GJGULL1.TTM" => {
            for time in [0.6, 1.3, 2., 3., 4.1, 12., 13., 14., 16., 17.5, 20., 22.] {
                cue(time, 21);
            }
            cue(38., 9);
        }
        "GJLILIPU.TTM" => {}
        "FIRE.TTM" => {
            cue(5.5, 23);
        }
        "MJBATH.TTM" => {
            for time in [0.5, 2., 4., 6., 8., 10., 12.] {
                cue(time, 16);
            }
            for time in [1., 3., 9.2, 11., 13.5, 16., 18.] {
                cue(time, 21);
            }
            cue(13.2, 5);
            cue(16., 3);
            cue(23., 24);
        }
        "MJCOCO1.TTM" => {
            for (time, sfx) in [
                (3.7, 23),
                (4.88, 5),
                (7.24, 5),
                (10.5, 9),
                (13., 4),
                (16., 4),
                (20.2, 23),
                (22.5, 9),
            ] {
                cue(time, sfx);
            }
        }
        "FISHWALK.TTM" | "GFFFOOD.TTM" => {
            for (time, sfx) in [
                (2.9, 1),
                (3.2, 5),
                (3.6, 11),
                (4.4, 9),
                (5.1, 19),
                (5.7, 19),
                (6.5, 16),
                (7.3, 9),
                (7.7, 1),
                (8.3, 19),
                (8.7, 19),
            ] {
                cue(time, sfx);
            }
        }
        "GJVIS6.TTM" => {
            cue(4., 19);
            cue(7.5, 5);
            cue(9., 8);
        }
        "MJAMBWLK.TTM" => {}
        "SJGLIMPS.TTM" => {
            cue(3.6, 4);
            cue(4.2, 16);
            cue(14., 16);
            cue(18., 5);
            cue(29.6, 4);
            cue(30.2, 16);
        }
        "SASKDATE.TTM" => {
            cue(6., 16);
            cue(8.9, 16);
            cue(21., 9);
            cue(23.7, 4);
            cue(25., 16);
        }
        "SBREAKUP.TTM" => {
            cue(3., 15);
            cue(9., 16);
            cue(17., 5);
            cue(23., 24);
        }
        "SUZYCITY.TTM" => {
            cue(14.5, 16);
            cue(17., 9);
        }
        "SJLEAVES.TTM" => {
            cue(5.5, 23);
            cue(8., 16);
            cue(17., 15);
            cue(18.3, 4);
            cue(19., 16);
        }
        "SJMSUZY.TTM" => {
            cue(8.5, 16);
            cue(10., 12);
            cue(12., 5);
            cue(15., 8);
            cue(16.5, 9);
        }
        "SMDATE.TTM" => {
            for (t, s) in [
                (0., 6),
                (3., 5),
                (4.4, 8),
                (5., 1),
                (6., 22),
                (9., 23),
                (12., 15),
                (14., 12),
                (16., 20),
                (18.5, 19),
                (21., 15),
                (26.5, 5),
                (28.5, 1),
                (30., 24),
            ] {
                cue(t, s);
            }
        }
        "WOULDBE.TTM" => {
            for (t, s) in [
                (3., 5),
                (6., 19),
                (9., 4),
                (10.2, 1),
                (12., 9),
                (13., 19),
                (22.5, 16),
                (23.3, 19),
                (24., 19),
                (25.2, 9),
            ] {
                cue(t, s);
            }
        }
        "SJMSSGE.TTM" => {
            cue(3.6, 23);
            cue(6., 9);
            cue(18., 5);
            cue(23., 4);
            cue(23.8, 1);
            cue(24., 24);
        }
        "MEANWHIL.TTM" | "SJWORK.TTM" => {
            cue(8.2, 9);
            cue(17.95, 24);
        }
        "MJSAND.TTM" => {
            for i in 0..16 {
                cue(1. + i as f32 * 0.25, 15);
            }
            for t in [10., 11., 15., 17., 19., 21., 23.] {
                cue(t, 10);
            }
            cue(5.5, 9);
            cue(11.8, 5);
            cue(14., 14);
            cue(25., 4);
            cue(36.8, 8);
            cue(38.1, 1);
        }
        "GJVIS5.TTM" | "GJVIS5W.TTM" => {
            let rate = if scene == "GJVIS5W.TTM" {
                789. / 982.
            } else {
                1.
            };
            for (time, sfx) in [
                (4.5, 19),
                (7.9, 23),
                (8.4, 9),
                (9.7, 4),
                (10.35, 23),
                (10.65, 5),
                (11.6, 1),
            ] {
                cue(time / rate, sfx);
            }
        }
        "GJCATCH2.TTM" => {
            for start in [2.8, 8.8, 11.2, 13.6, 16., 18.4] {
                let cast = if start == 2.8 { 1.2 } else { 0.6 };
                cue(start + cast * 0.5, 4);
                cue(start + cast, 16);
            }
            for (time, sfx) in [
                (20., 11),
                (20.4, 5),
                (21.3, 8),
                (23., 4),
                (24., 15),
                (25.3, 1),
                (28.6, 3),
            ] {
                cue(time, sfx);
            }
        }
        _ => return None,
    }
    Some(result)
}

fn fishing(t: f32, right: bool) -> Frame {
    let mut out = Frame::default();
    let shore = if right { 525. } else { 145. };
    let facing_left = !right;
    let direction = if right { 1. } else { -1. };
    let float = Point::new(shore + direction * 122., 298.);
    if t < 3.0 {
        out.beat = "carry pole to shore";
        out.walk(t, 410., shore, 3., true);
        return out;
    }
    if t >= 29.0 {
        out.beat = "carry pole back to shade";
        out.walk(t - 29., shore, 410., 3.5, true);
        return out;
    }
    // Two complete casts/catches. The second has a shorter wait; the action
    // always ends with the fish entering the water before walking away.
    let local = if t < 16.5 {
        t - 3.
    } else {
        (t - 16.5) * 13.5 / 12.5
    };
    let (clip, phase) = if local < 1.8 {
        out.beat = "bait hook";
        ("johnny-fishing-gag", ((local / 0.45) as usize).min(3))
    } else if local < 3.0 {
        out.beat = "cast";
        (
            "johnny-fishing-cast",
            (((local - 1.8) / 0.20) as usize).min(5),
        )
    } else if local < 7.0 {
        out.beat = "wait for bite";
        ("johnny-fishing-cast", 5)
    } else if local < 8.2 {
        out.beat = "tug and reel in";
        ("johnny-fishing-cast", 6 + ((local - 7.) / 0.3) as usize % 2)
    } else if local < 9.3 {
        out.beat = "fish on hook";
        ("johnny-fishing-gag", 4)
    } else if local < 11.0 {
        out.beat = "inspect small catch";
        ("johnny-fishing-gag", 5)
    } else if local < 11.5 {
        out.beat = "wind up to release fish";
        ("johnny-fishing-gag", 6)
    } else {
        out.beat = "throw fish back";
        ("johnny-fishing-gag", 7)
    };
    out.add(clip, phase, shore, 258., facing_left);
    if clip == "johnny-fishing-cast" {
        // Before the forward cast, the hook hangs near the rod. On release it
        // flies outward, then remains at one water contact until the pull.
        let u = ((local - 2.4) / 0.6).clamp(0., 1.);
        let mut end = Point::new(
            mix(shore + direction * 50., float.x, u),
            mix(206., float.y, u) - 42. * (std::f32::consts::PI * u).sin(),
        );
        if local >= 7. {
            let u = ((local - 7.) / 1.2).clamp(0., 1.);
            end = Point::new(
                mix(float.x, shore + direction * 28., u),
                mix(float.y, 212., u),
            );
        }
        out.line = Some(FishingLine {
            actor: 0,
            end,
            taut: local >= 7.,
            in_water: (3.0..7.).contains(&local),
        });
    }
    if (11.5..12.5).contains(&local) {
        let u = local - 11.5;
        let x = shore + direction * (30. + 92. * u);
        let y = mix(217., 298., u) - 50. * (std::f32::consts::PI * u).sin();
        out.add("fish-green", (u * 8.) as usize % 3, x, y, facing_left);
        out.layers.last_mut().unwrap().scale = 0.9;
    } else if local >= 12.5 {
        out.splash = Some((float, local - 12.5));
    }
    out
}

fn fire(t: f32) -> Frame {
    let mut out = Frame::default();
    let fire = Point::new(350., 261.);
    if t < 2. {
        out.beat = "approach tinder";
        out.add("driftwood", 0, 350., 261., false);
        out.walk(t, 410., 380., 2., false);
    } else if t < 21. {
        let (beat, phase) = if t < 4. {
            ("arrange tinder", 0)
        } else if t < 10. {
            ("rub sticks", 1 + ((t - 4.) * 5.) as usize % 3)
        } else if t < 11.5 {
            ("wipe forehead", 6)
        } else if t < 15. {
            ("rub sticks harder", 1 + ((t - 11.5) * 7.) as usize % 3)
        } else if t < 19. {
            ("blow on ember", 4 + ((t - 15.) * 2.) as usize % 2)
        } else {
            ("fire catches", 7)
        };
        out.beat = beat;
        out.add("johnny-firecraft", phase, fire.x, fire.y, true);
        if t >= 15. {
            out.smoke = Some(fire);
        }
    } else if t < 23.5 {
        out.beat = "fetch fish";
        out.walk(t - 21., 380., 455., 2.5, false);
    } else if t < 26. {
        out.beat = "pick up fish";
        out.add(
            "johnny-food-pickup",
            (((t - 23.5) / 0.8) as usize).min(2),
            455.,
            258.,
            false,
        );
    } else if t < 29. {
        out.beat = "return with fish";
        out.walk(t - 26., 455., 405., 3., false);
        out.layers.last_mut().unwrap().clip = "johnny-food-walk";
    } else if t < 35. {
        out.beat = "roast fish over fire";
        out.add(
            "johnny-roast",
            ((t - 29.) * 0.7) as usize % 2,
            390.,
            267.,
            true,
        );
    } else if t < 51. {
        let (beat, phase) = if t < 36.5 {
            ("inspect cooked fish", 0)
        } else if t < 37.3 {
            ("raise fish to mouth", 1)
        } else if t < 38.1 {
            ("bite hot fish", 2)
        } else if t < 39.0 {
            ("chew", 3)
        } else if t < 39.4 {
            ("too hot", 3)
        } else if t < 40.1 {
            ("flame escapes mouth", 5)
        } else if t < 40.9 {
            ("breathe flame", 6)
        } else if t < 42.2 {
            ("cover burning mouth", 7)
        } else if t < 45. {
            ("fan mouth", 8)
        } else if t < 47. {
            ("catch breath", 9)
        } else if t < 48. {
            ("spit fishbone", 10)
        } else {
            ("recover", 11)
        };
        out.beat = beat;
        out.add("johnny-hot-meal", phase, 416., 267., true);
    } else {
        out.beat = "return to shade";
        out.walk(t - 51., 405., 435., 3.95, false);
    }
    if t < 23.5 {
        out.add("fish-green", 0, 485., 258., false);
    }
    if t >= 19. {
        out.add("campfire", (t * 10.) as usize, fire.x, fire.y, false);
        out.layers.last_mut().unwrap().scale = if t < 21. {
            mix(0.15, 1., (t - 19.) / 2.)
        } else {
            1.
        };
    }
    out
}

fn raft(t: f32) -> Frame {
    let mut out = Frame::default();
    let ends = [0.6, 1.2, 1.8, 2.4, 2.8, 3.2, 3.5, 4.3, 4.9, 5.8, 6.5, 7.2];
    let phase = ends.iter().position(|&end| t < end).unwrap_or(11);
    out.beat = [
        "lay out logs",
        "align logs",
        "loop rope",
        "wrap logs",
        "tighten rope",
        "pull too hard",
        "tangle ankles",
        "fall backwards",
        "untangle feet",
        "tie final knot",
        "stand up",
        "inspect finished raft",
    ][phase];
    out.add("johnny-raft-build", phase, 350., 267., false);
    out.layers[0].shadow = false;
    out
}

fn shark(t: f32) -> Frame {
    let mut out = Frame::default();
    let (beat, phase) = if t < 7. {
        ("read at shore", 0)
    } else if t < 9.3 {
        ("notice shark and pull feet back", 1)
    } else if t < 10.6 {
        ("fall back from snapping shark", 2)
    } else if t < 11.4 {
        ("sit up after fright", 3)
    } else if t < 14. {
        ("scold shark", 4)
    } else if t < 17. {
        ("check toes", 7)
    } else if t < 18.1 {
        ("stand carefully", 5)
    } else {
        ("safe with book", 6)
    };
    out.beat = beat;
    out.add("johnny-shark-reaction", phase, 195., 270., false);
    if phase == 3 || phase == 7 {
        out.add("book", 0, 209., 269., false);
        out.layers.last_mut().unwrap().scale = 0.6;
    }
    if (4.0..14.5).contains(&t) {
        let x = if t < 7. {
            mix(-25., 137., (t - 4.) / 3.)
        } else if t < 9.3 {
            137.
        } else if t < 10.6 {
            148.
        } else {
            137.
        };
        let phase = if t < 9.3 {
            0
        } else if t < 10.6 {
            1
        } else if t < 13. {
            2
        } else {
            3
        };
        let y = if t < 13. {
            309.
        } else {
            309. + (t - 13.) * 32.
        };
        out.add("shark-head", phase, x, y, false);
        out.layers.last_mut().unwrap().waterline = Some(309.);
        if t < 13. {
            out.splash = Some((Point::new(x, 309.), 0.6));
        }
    }
    out
}

fn diving(t: f32) -> Frame {
    let mut out = Frame::default();
    let cycle = 3172. / 60. / 3.;
    let attempt = ((t / cycle) as usize).min(2);
    let u = t - attempt as f32 * cycle;
    let vote = (10.2..14.).contains(&u);
    let judge_phase = if vote {
        2
    } else if (9.8..10.2).contains(&u) {
        1
    } else {
        0
    };
    let scores = [
        ["5.2", "-5", "5.5", "5.6"],
        ["5.6", "5.2", "5.8", "5.7"],
        ["10", "-5", "10", "5.5"],
    ][attempt];
    for (i, (name, x, y)) in [
        ("judge-starfish", 225., 271.),
        ("judge-crab", 285., 275.),
        ("judge-octopus", 345., 275.),
        ("judge-gull", 515., 266.),
    ]
    .into_iter()
    .enumerate()
    {
        out.add(name, judge_phase, x, y, false);
        if vote {
            out.scores.push((out.layers.len() - 1, scores[i]));
        }
    }
    if u < 6. {
        out.beat = "climb palm for dive";
        let layout = Layout::for_background("ISLETEMP.SCR");
        let p = if u < 0.5 {
            Point::new(415., mix(258., 225., u / 0.5))
        } else {
            layout.palm_at((u - 0.5) / 5.5 * 0.9)
        };
        out.add("johnny-climb", (u * 6.) as usize, p.x, p.y, false);
        out.layers.last_mut().unwrap().shadow = false;
    } else if u < 7. {
        out.beat = "prepare to dive";
        out.add("johnny-dive-sequence", 0, 465., 82., false);
        out.layers.last_mut().unwrap().shadow = false;
    } else if u < 8.7 {
        out.beat = match attempt {
            0 => "headfirst dive",
            1 => "somersault dive",
            _ => "cannonball dive",
        };
        let a = (u - 7.) / 1.7;
        let phases: &[usize] = match attempt {
            0 => &[1, 2, 2, 3, 4],
            1 => &[1, 5, 6, 7, 3, 4],
            _ => &[1, 8, 8, 8, 9],
        };
        let phase = phases[((a * phases.len() as f32) as usize).min(phases.len() - 1)];
        out.add(
            "johnny-dive-sequence",
            phase,
            465. - 365. * a,
            65. + 209. * a * a,
            false,
        );
        let layer = out.layers.last_mut().unwrap();
        layer.shadow = false;
        layer.waterline = Some(302.);
    } else if u < 9.5 {
        out.beat = "water entry";
        out.splash = Some((Point::new(100., 302.), u - 8.7));
    } else if u < 10.5 {
        out.beat = "surface after dive";
        out.add(
            "johnny-water-tread",
            ((u - 9.5) * 3.) as usize % 2,
            mix(100., 145., u - 9.5),
            302.,
            false,
        );
        let layer = out.layers.last_mut().unwrap();
        layer.shadow = false;
        layer.waterline = Some(302.);
    } else if u < 12.5 {
        out.beat = "climb out at shoreline";
        let a = (u - 10.5) / 2.;
        out.add(
            "johnny-walk",
            (a * 12.) as usize,
            mix(145., 180., a),
            mix(340., 258., a),
            false,
        );
        let layer = out.layers.last_mut().unwrap();
        layer.shadow = false;
        layer.waterline = Some(302.);
    } else {
        out.beat = if vote {
            "judges score the dive"
        } else {
            "return to palm"
        };
        out.walk(u - 12.5, 180., 415., cycle - 12.5, false);
    }
    out
}

fn gulliver(t: f32) -> Frame {
    let mut out = Frame::default();
    if t < 2. {
        out.beat = "find a spot to nap";
        out.walk(t, 360., 390., 2., false);
    } else if t < 3. {
        out.beat = "settle on the sand";
        out.add("johnny-rise", 0, 390., 258., false);
    } else {
        let phase = if t < 21. {
            0
        } else if t < 23. {
            1
        } else if t < 37. {
            2
        } else if t < 38. {
            3
        } else {
            4
        };
        out.beat = match phase {
            0 => "sleep while miniature crew approaches",
            1 => "crew loops ropes around sleeping Johnny",
            2 => "sleep bound while crew withdraws",
            3 => "gull wakes bound Johnny",
            _ => "struggle against ropes",
        };
        out.add("johnny-gulliver", phase, 390., 258., false);
        out.layers.last_mut().unwrap().shadow = false;
    }
    // The ship stays in open water. Its launch goes around the island's left
    // edge; passengers disembark exactly where their walking path begins.
    if (8.0..36.).contains(&t) {
        let x = if t < 13. {
            mix(-100., 105., (t - 8.) / 5.)
        } else if t < 32. {
            105.
        } else {
            mix(105., 760., (t - 32.) / 4.)
        };
        out.add("galleon", 0, x, 183., false);
        let layer = out.layers.last_mut().unwrap();
        layer.scale = 0.45;
        layer.waterline = Some(182.);
    }
    if (13.0..32.).contains(&t) {
        let (x, y, loaded, flip) = if t < 17. {
            let a = (t - 13.) / 4.;
            (
                mix(105., 161., (a - 0.5) * 2.),
                mix(189., 280., a * 2.),
                true,
                false,
            )
        } else if t < 28. {
            (161., 280., false, false)
        } else {
            let a = (t - 28.) / 4.;
            (
                mix(161., 105., a * 2.),
                mix(280., 189., (a - 0.5) * 2.),
                true,
                true,
            )
        };
        out.add(
            if loaded {
                "tiny-rowboat"
            } else {
                "tiny-rowboat-empty"
            },
            if loaded { (t * 3.) as usize % 2 } else { 0 },
            x,
            y,
            flip,
        );
        let layer = out.layers.last_mut().unwrap();
        // Same hull width despite different master resolutions.
        layer.scale = if loaded { 0.55 } else { 0.46 };
        layer.waterline = Some(y - 1.);
    }
    if (17.0..28.).contains(&t) {
        for i in 0..2 {
            let work_x = 370. + i as f32 * 40.;
            let (clip, phase, x, y, flip) = if t < 21. {
                let a = ((t - 17.) / 4.).clamp(0., 1.);
                (
                    "tiny-sailor-walk",
                    ((t - 17.) * 7.) as usize % 2,
                    mix(161., work_x, a),
                    mix(278., 263., a),
                    false,
                )
            } else if t < 24. {
                (
                    "tiny-sailor-work",
                    ((t - 21.) * 2. + i as f32) as usize % 2,
                    work_x,
                    263.,
                    i == 1,
                )
            } else {
                let a = ((t - 24.) / 4.).clamp(0., 1.);
                (
                    "tiny-sailor-walk",
                    ((t - 24.) * 7.) as usize % 2,
                    mix(work_x, 161., a),
                    mix(263., 278., a),
                    true,
                )
            };
            out.add(clip, phase, x, y, flip);
            out.layers.last_mut().unwrap().scale = 0.25;
        }
    }
    if t >= 34.5 {
        if t < 36.5 {
            let a = (t - 34.5) / 2.;
            out.add(
                "gull",
                (t * 8.) as usize,
                mix(700., 392., a),
                mix(90., 231., a),
                true,
            );
            out.layers.last_mut().unwrap().scale = 0.7;
        } else {
            out.add(
                "gull-perch",
                if t < 37. {
                    3
                } else {
                    ((t - 37.) * 3.) as usize % 3
                },
                392.,
                237.,
                false,
            );
            out.layers.last_mut().unwrap().depth = Some(280.);
        }
    }
    out
}

fn heat(t: f32) -> Frame {
    let mut out = Frame::default();
    if t < 2. {
        out.beat = "walk into the heat";
        out.walk(t, 415., 350., 2., true);
        out.layers[0].clip = "johnny-walk";
    } else {
        let (beat, phase) = if t < 3. {
            ("wipe sweaty brow", 0)
        } else if t < 10. {
            ("fan face", 1 + ((t - 3.) * 4.) as usize % 3)
        } else if t < 11. {
            ("slump in the heat", 4)
        } else if t < 12. {
            ("sit exhausted", 5)
        } else if t < 14.5 {
            ("lie back with fan", 6)
        } else {
            ("sit up slowly", 7)
        };
        out.beat = beat;
        out.add("johnny-heat", phase, 350., 258., false);
    }
    out
}

fn rain_dance(t: f32) -> Frame {
    let mut out = Frame::default();
    if t < 6. {
        out.beat = "too hot; fan face";
        out.add(
            "johnny-heat",
            if t < 1. { 0 } else { 1 + (t * 4.) as usize % 3 },
            350.,
            258.,
            false,
        );
    } else if t < 8. {
        out.beat = "idea: dance for rain";
        out.add("johnny-thinking", 0, 350., 258., false);
        out.add("idea-bulb", 0, 365., 175., false);
        out.add("wood-mask", 0, 390., 263., false);
        out.add("leaf-skirt", 0, 375., 267., false);
    } else if t < 11. {
        out.beat = "put on leaf skirt and mask";
        out.add(
            "johnny-mask-dress",
            (((t - 8.) / 0.75) as usize).min(3),
            350.,
            258.,
            false,
        );
        if t < 9.5 {
            out.add("wood-mask", 0, 390., 263., false);
        }
    } else if t < 24. {
        out.beat = "dance for rain";
        let phase = ((t - 11.) * 5.) as usize % 8;
        out.add(
            "johnny-mask-dance",
            phase,
            350. + 25. * ((t - 11.) * std::f32::consts::PI / 13.).sin(),
            258.,
            phase >= 4,
        );
    } else {
        let (beat, phase) = if t < 25. {
            ("look up at storm cloud", 0)
        } else if t < 25.25 {
            ("lightning interrupts the dance", 4)
        } else if t < 27. {
            ("singed by lightning", 5)
        } else if t < 29. {
            ("cough smoke", 6)
        } else {
            ("sit disappointed beside costume", 7)
        };
        out.beat = beat;
        out.add("johnny-mask-reaction", phase, 350., 258., false);
        if phase >= 4 && phase < 7 {
            out.add("wood-mask", 0, 378., 266., false);
        }
        if (25.0..25.2).contains(&t) {
            out.lightning = Some((Point::new(350., 100.), Point::new(350., 199.)));
        }
        if (25.2..29.).contains(&t) {
            out.smoke = Some(Point::new(350., 214.));
        }
    }
    if (17.0..29.).contains(&t) {
        let phase = if t < 19. {
            0
        } else if t < 25. {
            1
        } else if t < 25.2 {
            2
        } else {
            3
        };
        out.add("storm-cloud", phase, 350., 80., false);
        out.layers.last_mut().unwrap().scale = if t < 19. {
            mix(0.15, 1., (t - 17.) / 2.)
        } else if t > 27. {
            mix(1., 0.1, (t - 27.) / 2.)
        } else {
            1.
        };
    }
    if t >= 6. {
        out.add("palm-fan", 0, 326., 266., false);
    }
    out
}

fn mask_boat(t: f32) -> Frame {
    let mut out = Frame::default();
    let (beat, clip, phase) = if t < 5. {
        (
            "dance in homemade costume",
            "johnny-mask-dance",
            (t * 5.) as usize % 8,
        )
    } else if t < 7. {
        ("notice passing motorboat", "johnny-mask-reaction", 0)
    } else if t < 9. {
        ("lift mask to call for help", "johnny-mask-reaction", 1)
    } else if t < 15. {
        (
            "wave mask at motorboat",
            "johnny-mask-wave",
            ((t - 9.) * 5.) as usize % 4,
        )
    } else if t < 21. {
        (
            "wave palm leaves for rescue",
            "johnny-leaf-wave",
            ((t - 15.) * 5.) as usize % 4,
        )
    } else {
        ("boat leaves; sit disappointed", "johnny-mask-reaction", 7)
    };
    out.beat = beat;
    out.add(clip, phase, 360., 258., false);
    if (3.0..26.).contains(&t) {
        let x = if t < 7. {
            mix(-100., 315., (t - 3.) / 4.)
        } else if t < 19. {
            mix(315., 390., (t - 7.) / 12.)
        } else {
            mix(390., 760., (t - 19.) / 7.)
        };
        out.add("motorboat", 0, x, 178., false);
        out.layers.last_mut().unwrap().scale = 1.3;
        out.layers.last_mut().unwrap().waterline = Some(177.);
        out.add("sailor", 0, x + 15., 169., false);
        out.layers.last_mut().unwrap().scale = 0.35;
        out.add("suzy-wave", 0, x - 22., 168., false);
        out.layers.last_mut().unwrap().scale = 0.30;
    }
    out
}

fn solo_diving(t: f32) -> Frame {
    let mut out = diving(t * (3172. / 3. / 901.));
    out.layers.retain(|l| !l.clip.starts_with("judge-"));
    out.scores.clear();
    if out.beat == "judges score the dive" {
        out.beat = "return to palm after dive";
    }
    out
}

fn reading(t: f32) -> Frame {
    let mut out = Frame::default();
    if t < 16. {
        out.beat = "read while gull settles in palm";
        out.add("johnny-read", (t / 2.5) as usize % 4, 400., 264., false);
    } else {
        let (beat, phase) = if t < 17.2 {
            ("notice diving gull", 1)
        } else if t < 18.2 {
            ("reach for stolen book", 2)
        } else if t < 19. {
            ("stare after book thief", 3)
        } else if t < 21. {
            ("stand and reach for book", 4)
        } else if t < 24. {
            ("scold gull in palm", 5)
        } else {
            ("gull keeps book; look up in defeat", 6)
        };
        out.beat = beat;
        if phase >= 4 {
            out.add("reading-log", 0, 400., 260., false);
        }
        out.add(
            "johnny-book-theft",
            phase,
            if phase < 4 { 400. } else { 410. },
            264.,
            false,
        );
    }
    if (4.0..8.).contains(&t) {
        let a = (t - 4.) / 4.;
        out.add(
            "gull",
            (t * 8.) as usize,
            mix(-60., 469., a),
            mix(60., 52., a) - 40. * (std::f32::consts::PI * a).sin(),
            false,
        );
    } else if (8.0..15.5).contains(&t) {
        out.add(
            "gull-perch",
            if (t * 2.) as usize % 5 == 0 { 2 } else { 0 },
            469.,
            52.,
            true,
        );
    } else if (15.5..17.2).contains(&t) {
        let a = (t - 15.5) / 1.7;
        out.add(
            "gull",
            (t * 8.) as usize,
            mix(469., 416., a) - 60. * (std::f32::consts::PI * a).sin(),
            mix(52., 212., a),
            a < 0.65,
        );
    } else if (17.2..21.).contains(&t) {
        let a = (t - 17.2) / 3.8;
        out.add(
            "gull-book",
            ((t - 17.2) * 7.) as usize % 3,
            mix(416., 469., a) + 100. * (std::f32::consts::PI * a).sin(),
            mix(228., 52., a),
            a > 0.6,
        );
        out.layers.last_mut().unwrap().depth = Some(300.);
    } else if t >= 21. {
        out.add("gull-book", 3, 469., 52., true);
    }
    out
}

fn jogging(t: f32) -> Frame {
    let mut out = Frame::default();
    if t < 2. {
        out.beat = "walk to warm up";
        out.walk(t, 445., 410., 2., false);
    } else if t < 14. {
        let (beat, clip) = if t < 3. {
            ("prepare to stretch", "johnny-rest")
        } else if t < 5. {
            ("stretch arms overhead", "johnny-stretch-up")
        } else if t < 7. {
            ("stretch left side", "johnny-stretch-left")
        } else if t < 9. {
            ("stretch right side", "johnny-stretch-right")
        } else if t < 12. {
            ("touch toes", "johnny-bend")
        } else {
            ("straighten before jogging", "johnny-rise")
        };
        out.beat = beat;
        out.add(clip, 0, 410., 264., false);
    } else if t < 31. {
        out.beat = "jog three laps on the sand";
        let angle = 0.43 + (t - 14.) / 17. * std::f32::consts::TAU * 3.;
        let x = 335. + 180. * angle.sin();
        let y = 247. + 18. * angle.cos();
        out.add(
            "johnny-run",
            ((t - 14.) * 10.3) as usize % 8,
            x,
            y,
            angle.cos() < 0.,
        );
    } else if t < 34. {
        out.beat = "catch breath after jogging";
        out.add(
            if t < 33. {
                "johnny-tired"
            } else {
                "johnny-rise"
            },
            0,
            410.,
            264.,
            false,
        );
    } else {
        out.beat = "walk back to shade";
        out.walk(t - 34., 410., 435., 2170. / 60. - 34., false);
    }
    out
}

fn mermaid_glimpse(t: f32) -> Frame {
    let mut out = Frame::default();
    if t < 3. {
        out.beat = "carry pole to right shore";
        out.walk(t, 405., 525., 3., true);
    } else if t < 18. {
        out.beat = if t < 4.2 {
            "cast to right"
        } else {
            "fish while mermaid passes behind"
        };
        let phase = if t < 4.2 {
            (((t - 3.) / 0.2) as usize).min(5)
        } else if t < 16.5 {
            5
        } else {
            6 + ((t - 16.5) * 3.) as usize % 2
        };
        out.add("johnny-fishing-cast", phase, 525., 258., false);
        let u = ((t - 3.6) / 0.6).clamp(0., 1.);
        let v = ((t - 16.5) / 1.5).clamp(0., 1.);
        out.line = Some(FishingLine {
            actor: 0,
            end: Point::new(
                mix(mix(565., 647., u), 553., v),
                mix(mix(206., 298., u), 212., v),
            ),
            taut: t >= 16.5,
            in_water: (4.2..16.5).contains(&t),
        });
    } else if t < 23. {
        out.beat = "look back and wonder about mermaid";
        out.add("johnny-pole-glance", 1, 525., 258., true);
        if t > 19. {
            out.add("story-thought", 7, 480., 180., false);
            out.layers.last_mut().unwrap().depth = Some(1000.);
        }
    } else if t < 29. {
        out.beat = "try fishing where mermaid was seen";
        out.walk(t - 23., 525., 145., 6., true);
    } else {
        out.beat = "cast toward mermaid departure";
        out.add(
            "johnny-fishing-cast",
            if t < 30.2 {
                (((t - 29.) / 0.2) as usize).min(5)
            } else {
                5
            },
            145.,
            258.,
            true,
        );
        let u = ((t - 29.6) / 0.6).clamp(0., 1.);
        out.line = Some(FishingLine {
            actor: 0,
            end: Point::new(mix(105., 23., u), mix(206., 298., u)),
            taut: false,
            in_water: t >= 30.2,
        });
    }
    if (12.0..18.).contains(&t) {
        let u = (t - 12.) / 6.;
        out.add(
            "mermaid-swim",
            (t * 3.) as usize % 2,
            mix(100., -100., u),
            300. + (t * 3.).sin() * 3.,
            true,
        );
        out.layers.last_mut().unwrap().scale = 0.8;
    }
    out
}

fn suzy_city(t: f32) -> Frame {
    if t < 49. / 60. {
        return office(t);
    }
    let mut out = Frame::default();
    if t < 10.5 {
        out.beat = "read on city beach";
        out.add(
            "suzy-beach-read",
            if t < 2. { 2 } else { (t * 0.5) as usize % 2 },
            100.,
            390.,
            false,
        );
    } else {
        out.add("beach-towel", 0, 100., 390., false);
        if t < 11.5 {
            out.beat = "notice message bottle";
            out.add("suzy-beach-stand", 0, 110., 390., false);
        } else if t < 14.5 {
            out.beat = "walk to washed-up bottle";
            let u = (t - 11.5) / 3.;
            out.add(
                "suzy-walk",
                (u * 12.) as usize,
                mix(110., 205., u),
                mix(390., 420., u),
                false,
            );
            out.layers.last_mut().unwrap().scale = 0.9;
        } else if t < 15.5 {
            out.beat = "pick up message";
            out.add("suzy-bottle-pick", 0, 205., 420., false);
        } else {
            out.beat = "read Johnny's SOS and imagine meeting him";
            out.add("suzy-letter", 0, 205., 420., false);
            if t >= 17. {
                out.add(
                    "suzy-thought",
                    if t < 22.5 { 0 } else { 1 },
                    255.,
                    330.,
                    true,
                );
                out.layers.last_mut().unwrap().depth = Some(1000.);
                out.layers.last_mut().unwrap().scale = 1.25;
            }
        }
    }
    if (6.0..15.).contains(&t) {
        let u = ((t - 6.) / 8.5).clamp(0., 1.);
        out.add("bottle", 0, mix(600., 232., u), mix(350., 421., u), false);
    }
    out
}

fn raft_departure(t: f32) -> Frame {
    let mut out = Frame::default();
    if t < 5.5 {
        out.beat = "finish packing for departure";
        out.add("journey-raft-empty", 0, 550., 278., false);
        out.add("johnny-bundle", 0, 510., 264., false);
    } else if t < 7.5 {
        out.beat = "load bundle on raft";
        out.add("johnny-raft-load", 0, 550., 278., false);
    } else if t < 17. {
        out.beat = "say farewell to mermaid and shark";
        out.add("journey-raft", 0, 550., 278., false);
        out.add(
            "johnny-dialogue",
            if t < 10. {
                2
            } else if t < 15. {
                0
            } else {
                4
            },
            515.,
            258.,
            true,
        );
    } else if t < 18.3 {
        out.beat = "push raft off sand";
        out.add(
            "johnny-raft-journey",
            0,
            mix(550., 575., (t - 17.) / 1.3),
            mix(278., 292., (t - 17.) / 1.3),
            false,
        );
    } else if t < 19. {
        out.beat = "step onto departing raft";
        out.add("johnny-raft-journey", 1, 575., 292., false);
    } else {
        out.beat = "paddle away from island";
        let u = (t - 19.) / 3.3;
        out.add(
            "johnny-raft-journey",
            if t < 20. {
                4
            } else {
                2 + (t * 3.) as usize % 2
            },
            mix(575., 770., u),
            mix(292., 245., u),
            false,
        );
    }
    if (8.0..22.3).contains(&t) {
        let u = ((t - 8.) / 2.).clamp(0., 1.);
        out.add(
            "mermaid-gift",
            if t < 16. { 1 } else { 3 },
            470.,
            mix(335., 292., u),
            false,
        );
        out.layers.last_mut().unwrap().waterline = Some(290.);
        out.add("shark-head", 0, 410., mix(355., 302., u), false);
        let l = out.layers.last_mut().unwrap();
        l.scale = 0.7;
        l.waterline = Some(300.);
    }
    out
}

fn suzy_reunion(t: f32) -> Frame {
    if t < 49. / 60. {
        return office(t);
    }
    let mut out = Frame::default();
    if t < 8.5 {
        out.beat = "Johnny paddles into Suzy's beach";
        out.add("suzy-beach-read", (t * 0.5) as usize % 2, 100., 390., false);
        let u = ((t - 1.) / 7.5).clamp(0., 1.);
        out.add(
            "johnny-raft-journey",
            2 + (t * 3.) as usize % 2,
            mix(700., 270., u),
            mix(300., 427., u),
            true,
        );
    } else {
        out.add("beach-towel", 0, 100., 390., false);
        out.add("journey-raft", 0, 270., 427., true);
        if t < 10. {
            out.beat = "step ashore as Suzy runs to greet him";
            let u = (t - 8.5) / 1.5;
            out.add(
                "johnny-walk",
                (u * 10.) as usize,
                mix(265., 215., u),
                mix(420., 410., u),
                true,
            );
            out.add(
                "suzy-run",
                (u * 8.) as usize % 2,
                mix(110., 190., u),
                mix(390., 410., u),
                false,
            );
        } else {
            out.beat = if t < 12. {
                "happy reunion hug"
            } else if t < 15. {
                "enthusiastic hug lifts Johnny"
            } else if t < 16.5 {
                "couple loses balance"
            } else {
                "Suzy helps dazed Johnny up"
            };
            out.add(
                "suzy-reunion",
                if t < 12. {
                    0
                } else if t < 15. {
                    1
                } else if t < 16.5 {
                    2
                } else {
                    3
                },
                205.,
                415.,
                false,
            );
        }
    }
    out
}

fn mermaid_date(t: f32) -> Frame {
    let mut out = Frame::default();
    if t < 3. {
        out.beat = "walk to shore before date";
        out.walk(t, 405., 145., 3., false);
    } else if t < 5. {
        out.beat = "dive into sea to freshen up";
        let u = (t - 3.) / 2.;
        out.add(
            "johnny-dive-sequence",
            [1, 2, 3, 4][((u * 4.) as usize).min(3)],
            mix(145., 65., u),
            mix(220., 305., u * u),
            true,
        );
        out.layers.last_mut().unwrap().shadow = false;
    } else if t < 6. {
        out.beat = "underwater costume change";
        out.splash = Some((Point::new(65., 302.), t - 5.));
    } else if t < 8. {
        out.beat = "return in tuxedo";
        let u = (t - 6.) / 2.;
        out.add(
            "johnny-tux-walk",
            (u * 10.) as usize,
            mix(120., 390., u),
            258.,
            false,
        );
    } else if t < 9. {
        out.beat = "bring out folded dinner table";
        out.add("johnny-date-setup", 0, 390., 258., false);
    } else if t < 10.5 {
        out.beat = "set up dinner table";
        out.add("johnny-date-setup", 1, 450., 266., false);
    } else if t < 12. {
        out.beat = "set chairs and candle";
        out.add("johnny-date-setup", 2, 450., 266., false);
    } else if t < 12.7 {
        out.beat = "wind gramophone";
        out.add("date-table", 0, 450., 266., false);
        out.add("johnny-date-setup", 3, 335., 266., false);
    } else if t < 14. {
        out.beat = "go to welcome mermaid as she leaps ashore";
        out.add("date-table", 0, 450., 266., false);
        out.add("date-gramophone", 0, 335., 266., false);
        let u = (t - 12.7) / 1.3;
        out.add(
            "johnny-tux-walk",
            (u * 10.) as usize,
            mix(335., 420., u),
            258.,
            false,
        );
    } else if t < 16. {
        out.beat = "welcome mermaid for dinner";
        out.add("date-table", 0, 450., 266., false);
        out.add("date-gramophone", 0, 335., 266., false);
        out.add("date-couple", 0, 450., 264., false);
    } else if t < 21. {
        out.beat = "dinner conversation and toast";
        out.add("date-gramophone", 0, 335., 266., false);
        out.add(
            "date-seated",
            if t < 18.5 { 0 } else { 1 },
            450.,
            266.,
            false,
        );
    } else {
        out.add("date-table", 0, 450., 266., false);
        out.add("date-gramophone", 0, 335., 266., false);
        if t < 22.5 {
            out.beat = "lift mermaid from dinner chair";
            let u = (t - 21.) / 1.5;
            out.add(
                "date-couple",
                1,
                mix(450., 385., u),
                264. - (u * std::f32::consts::TAU * 3.).sin().abs() * 1.5,
                false,
            );
        } else if t < 26.5 {
            out.beat = "dance to gramophone music";
            out.add(
                "date-couple",
                2 + (t * 2.) as usize % 2,
                385. + ((t - 22.5) * 1.3).sin() * 4.,
                264.,
                false,
            );
        } else if t < 27.3 {
            out.beat = "dance lift goes wrong";
            out.add("date-lift-mishap", 0, 385., 264., false);
        } else if t < 28.5 {
            out.beat = "mermaid flies from dance into sea";
            out.add("johnny-tux-startled", 0, 385., 258., false);
            let u = (t - 27.3) / 1.2;
            out.add(
                "mermaid-dive",
                0,
                mix(385., 90., u),
                mix(185., 305., u * u),
                false,
            );
            out.layers.last_mut().unwrap().waterline = Some(302.);
        } else {
            out.beat = "watch mermaid leave after dance mishap";
            if t < 30. {
                out.add(
                    "johnny-tux-walk",
                    ((t - 28.5) * 8.) as usize,
                    mix(385., 170., (t - 28.5) / 1.5),
                    258.,
                    true,
                );
            } else {
                out.add("johnny-tux-startled", 0, 170., 258., false);
            }
            if t < 29.1 {
                out.splash = Some((Point::new(90., 302.), t - 28.5));
            }
            let u = ((t - 29.1) / 4.73).clamp(0., 1.);
            out.add(
                "mermaid-swim",
                (t * 3.) as usize % 2,
                mix(90., -100., u),
                mix(295., 220., u),
                true,
            );
            out.layers.last_mut().unwrap().scale = 0.8;
        }
    }
    if (9.0..14.).contains(&t) {
        let u = ((t - 9.) / 3.7).clamp(0., 1.);
        let leap = ((t - 12.7) / 1.3).clamp(0., 1.);
        out.add(
            "mermaid-swim",
            (t * 3.) as usize % 2,
            mix(mix(-60., 350., u), 480., leap),
            mix(180., 242., leap) - (leap * std::f32::consts::PI).sin() * 45.,
            false,
        );
        out.layers.last_mut().unwrap().scale = mix(0.55, 1., leap);
    }
    out
}

fn party_rescue(t: f32) -> Frame {
    let mut out = Frame::default();
    if t < 3. {
        out.beat = "watch waterskiing tourists";
        out.walk(t, 335., 390., 3., false);
    } else if t < 6. {
        out.beat = "wave at returning party boat";
        out.add("johnny-help", (t * 3.) as usize % 2, 390., 258., true);
    } else if t < 9. {
        out.beat = "party boat stops nearby";
        out.add("johnny-dialogue", 0, 390., 258., true);
    } else if t < 10.2 {
        out.beat = "dive toward would-be rescuers";
        let u = (t - 9.) / 1.2;
        out.add(
            "johnny-dive-sequence",
            [1, 2, 3, 4][((u * 4.) as usize).min(3)],
            mix(390., 285., u),
            mix(220., 205., u) - 40. * (std::f32::consts::PI * u).sin(),
            true,
        );
        out.layers.last_mut().unwrap().shadow = false;
    } else if t < 12. {
        out.beat = "swim to party boat";
        out.add(
            "johnny-water-help",
            0,
            mix(285., 225., (t - 10.2) / 1.8),
            215.,
            false,
        );
        out.layers.last_mut().unwrap().waterline = Some(210.);
    } else if t < 22.5 {
        out.beat = "join party then return to island";
    } else if t < 23.2 {
        out.beat = "step ashore after party";
        let u = (t - 22.5) / 0.7;
        out.add(
            "johnny-party",
            0,
            mix(346., 405., u),
            mix(205., 258., u),
            false,
        );
    } else if t < 24.8 {
        out.beat = "stagger on the beach";
        out.add(
            "johnny-party",
            ((t - 23.2) * 3.) as usize % 2,
            mix(405., 365., (t - 23.2) / 1.6),
            258.,
            true,
        );
    } else {
        out.beat = "tipsy castaway collapses to sleep";
        out.add(
            "johnny-party",
            if t < 25.2 {
                2
            } else if t < 25.7 {
                3
            } else {
                4
            },
            365.,
            267.,
            false,
        );
    }
    if t < 3. {
        let x = mix(760., -200., t / 3.);
        out.add("party-yacht", (t * 2.) as usize % 2, x, 180., true);
        out.add("waterskier", (t * 4.) as usize % 2, x + 150., 187., true);
        out.layers.last_mut().unwrap().scale = 0.6;
        out.ropes
            .push((Point::new(x + 60., 169.), Point::new(x + 137., 170.)));
    } else if (6.0..16.).contains(&t) {
        let x = if t < 9. {
            mix(-100., 285., (t - 6.) / 3.)
        } else if t < 13. {
            285.
        } else {
            mix(285., 780., (t - 13.) / 3.)
        };
        out.add(
            "party-yacht",
            if t < 12. {
                (t * 2.) as usize % 2
            } else {
                2 + (t * 2.) as usize % 2
            },
            x,
            200.,
            false,
        );
    } else if t >= 20. {
        let x = mix(760., -130., (t - 20.) / 4.8);
        out.add(
            "party-yacht",
            if t < 22.5 {
                2 + (t * 2.) as usize % 2
            } else {
                (t * 2.) as usize % 2
            },
            x,
            200.,
            true,
        );
    }
    out
}

fn mermaid_invitation(t: f32) -> Frame {
    let mut out = Frame::default();
    let cap;
    if t < 1. {
        out.beat = "rest in palm shade";
        out.add("johnny-rest", 0, 405., 258., true);
        cap = Some(Point::new(405., 194.));
    } else if t < 12. {
        out.beat = "mermaid swims toward island";
        out.add("johnny-sad", 0, 405., 263., true);
        cap = Some(Point::new(398., 219.));
    } else if t < 16. {
        out.beat = "go to greet mermaid";
        out.walk(t - 12., 405., 165., 4., false);
        cap = Some(Point::new(mix(405., 165., (t - 12.) / 4.), 194.));
    } else if t < 19. {
        out.beat = "greet visitor at shore";
        out.add("johnny-rest", 0, 165., 267., true);
        cap = Some(Point::new(165., 203.));
    } else {
        cap = if t < 21. {
            Some(Point::new(165., 209.))
        } else {
            None
        };
        let phase = if t < 21. {
            0
        } else if t < 22.8 {
            1
        } else if t < 23.7 {
            2
        } else if t < 25. {
            3
        } else if t < 35.5 {
            4
        } else {
            5
        };
        out.beat = if t < 21. {
            "receive flower lei"
        } else if t < 22.8 {
            "wear mermaid gift"
        } else if t < 25. {
            "toss hat in delight"
        } else if t < 35.5 {
            "invite mermaid to dinner and receive green light"
        } else {
            "wave goodbye to departing mermaid"
        };
        out.add("johnny-invitation", phase, 165., 267., false);
    }
    if let Some(p) = cap {
        out.add("sailor-cap", 0, p.x, p.y, true);
        out.layers.last_mut().unwrap().depth = Some(1000.);
    }
    if (23.7..25.).contains(&t) {
        let u = (t - 23.7) / 1.3;
        out.add(
            "sailor-cap",
            0,
            mix(172., 20., u),
            mix(204., 301., u) - 50. * (u * std::f32::consts::PI).sin(),
            false,
        );
    } else if t >= 25. {
        out.add(
            "sailor-cap",
            0,
            20. - (t - 25.) * 1.4,
            301. + (t * 2.).sin(),
            false,
        );
        out.layers.last_mut().unwrap().waterline = Some(302.);
    }
    if (2.0..12.0).contains(&t) {
        let u = (t - 2.) / 10.;
        out.add(
            "mermaid-swim",
            (t * 3.) as usize % 2,
            mix(-50., 110., u),
            mix(170., 270., u) - (u * std::f32::consts::TAU * 2.).sin().max(0.) * 18.,
            false,
        );
        out.layers.last_mut().unwrap().scale = mix(0.35, 0.75, u);
    } else if (12.0..38.).contains(&t) {
        let phase = if t < 19. {
            1
        } else if t < 21. {
            0
        } else if t < 35.5 {
            1
        } else {
            3
        };
        out.add("mermaid-gift", phase, 110., 270., false);
        out.layers.last_mut().unwrap().waterline = Some(266.);
    } else if t >= 38. {
        let u = (t - 38.) / 6.5;
        out.add(
            "mermaid-swim",
            (t * 3.) as usize % 2,
            mix(110., -100., u),
            mix(270., 175., u),
            true,
        );
        out.layers.last_mut().unwrap().scale = mix(0.75, 0.35, u);
    }
    if (27.0..35.5).contains(&t) {
        out.add("story-thought", 3, 190., 185., false);
        out.layers.last_mut().unwrap().depth = Some(1100.);
    }
    if (31.0..35.5).contains(&t) {
        out.add("story-thought", 6, 60., 210., true);
        let l = out.layers.last_mut().unwrap();
        l.scale = 0.6;
        l.depth = Some(1100.);
    }
    out
}

fn mermaid_breakup(t: f32) -> Frame {
    let mut out = Frame::default();
    if t < 1. {
        out.beat = "prepare raft";
        out.walk(t, 390., 475., 1., false);
    } else if t < 6. {
        out.beat = "tie logs for escape raft";
        out.add(
            "johnny-raft-build",
            [2, 3, 4, 5][((t - 1.) * 3.) as usize % 4],
            475.,
            264.,
            false,
        );
    } else {
        out.add("raft", 0, 475., 270., false);
        if t < 8. {
            out.beat = "approach visiting mermaid";
            out.walk(t - 6., 475., 535., 2., false);
        } else if t < 17. {
            out.beat = "explain wish to return to city";
            out.add(
                "johnny-dialogue",
                if t < 12. { 0 } else { 1 },
                535.,
                258.,
                false,
            );
        } else if t < 23. {
            out.beat = "mermaid rejects plan";
            out.add("johnny-dialogue", 2, 535., 258., false);
        } else {
            out.beat = "left alone after mermaid leaves";
            out.add("johnny-dialogue", 3, 535., 258., false);
        }
    }
    if (1.0..9.0).contains(&t) {
        let u = (t - 1.) / 8.;
        out.add(
            "mermaid-swim",
            (t * 3.) as usize % 2,
            mix(-60., 590., u / 0.75),
            if u < 0.75 {
                180.
            } else {
                mix(180., 278., (u - 0.75) / 0.25)
            },
            false,
        );
        out.layers.last_mut().unwrap().scale = mix(0.35, 0.75, u);
    } else if (9.0..22.).contains(&t) {
        out.add(
            "mermaid-gift",
            if t < 16. { 1 } else { 2 },
            590.,
            278.,
            true,
        );
        out.layers.last_mut().unwrap().waterline = Some(270.);
    } else if (22.0..27.).contains(&t) {
        let u = (t - 22.) / 5.;
        out.add(
            "mermaid-swim",
            (t * 3.) as usize % 2,
            if u < 0.35 {
                590.
            } else {
                mix(590., -100., (u - 0.35) / 0.65)
            },
            mix(278., 175., u / 0.35),
            true,
        );
        out.layers.last_mut().unwrap().scale = mix(0.75, 0.35, u);
        out.layers.last_mut().unwrap().depth = Some(180.);
    }
    if (12.0..16.5).contains(&t) {
        out.add("story-thought", 5, 540., 180., false);
        out.layers.last_mut().unwrap().depth = Some(1000.);
    }
    if (17.0..20.).contains(&t) {
        out.add("thought-question", 0, 535., 180., false);
        out.layers.last_mut().unwrap().scale = 0.5;
    }
    out
}

fn message_bottle(t: f32) -> Frame {
    let mut out = Frame::default();
    if t < 2.8 {
        out.beat = "walk to washed-up message bottle";
        out.walk(t, 390., 150., 2.8, false);
        out.add("bottle", 0, 122., 267., false);
        out.layers.last_mut().unwrap().scale = 1.4;
    } else if t < 25. {
        let phase = if t < 3.4 {
            0
        } else if t < 3.8 {
            1
        } else if t < 4.4 {
            2
        } else if t < 5. {
            3
        } else if t < 5.6 {
            4
        } else if t < 10. {
            5
        } else if t < 18. {
            7
        } else if t < 20.5 {
            6
        } else if t < 22.2 {
            8
        } else if t < 23. {
            9
        } else if t < 23.7 {
            10
        } else {
            11
        };
        out.beat = if t < 5.6 {
            "retrieve bottle and open message"
        } else if t < 10. {
            "read message"
        } else if t < 18. {
            "imagine the sender on a desert island"
        } else if t < 20.5 {
            "recognize his own SOS"
        } else if t < 23. {
            "put message back and wind up"
        } else if t < 23.7 {
            "throw message bottle back to sea"
        } else {
            "disappointed at his own returned plea"
        };
        out.add("johnny-message", phase, 150., 258., false);
        if (10.0..20.5).contains(&t) {
            let phase = if t < 12.4 {
                0
            } else if t < 14.5 {
                1
            } else if t < 18. {
                2
            } else {
                4
            };
            out.add("story-thought", phase, 100., 185., false);
            out.layers.last_mut().unwrap().depth = Some(1000.);
        }
    } else {
        out.beat = "return to the palm";
        out.walk(t - 25., 150., 390., 1604. / 60. - 25., false);
    }
    if (23.0..23.8).contains(&t) {
        let u = (t - 23.) / 0.8;
        out.add(
            "bottle",
            0,
            mix(117., 15., u),
            mix(215., 300., u) - 40. * (std::f32::consts::PI * u).sin(),
            false,
        );
        out.layers.last_mut().unwrap().scale = 1.4;
    } else if (23.8..24.4).contains(&t) {
        out.splash = Some((Point::new(15., 300.), t - 23.8));
    }
    out
}

fn office(t: f32) -> Frame {
    let mut out = Frame::default();
    if t < 49. / 60. {
        out.beat = "meanwhile";
        out.black_background = true;
        out.add("clock", 0, 320., 215., false);
        out.layers[0].scale = 5.;
    } else {
        let phase = if t < 8.2 {
            (t * 4.) as usize % 2
        } else if t < 17.95 {
            2
        } else {
            3
        };
        out.beat = if t < 8.2 {
            "work at office keyboard"
        } else if t < 11. {
            "recline and daydream"
        } else if t < 15.6 {
            "remember the castaway island"
        } else if t < 17.95 {
            "remember dinner with the mermaid"
        } else {
            "return reluctantly to work"
        };
        out.add("johnny-office-seated", phase, 343., 313., false);
        out.layers[0].shadow = false;
        if (11.0..17.95).contains(&t) {
            let thought = if t < 12.4 {
                0
            } else if t < 14.2 {
                1
            } else if t < 15.6 {
                2
            } else {
                3
            };
            out.add("story-thought", thought, 278., 145., false);
            out.layers.last_mut().unwrap().scale = 1.1;
            out.layers.last_mut().unwrap().depth = Some(1000.);
        }
    }
    out
}

fn sandcastle(t: f32) -> Frame {
    let mut out = Frame::default();
    if t < 1. {
        out.beat = "kneel to build sandcastle";
        out.walk(t, 350., 285., 1., false);
    } else if t < 5.5 {
        out.beat = "shape sandcastle towers";
        let phase = (((t - 1.) / 4.5 * 4.) as usize).min(3);
        out.add("johnny-sand-build", phase, 315., 265., false);
    } else {
        out.add("sandcastle-stages", 2, 315., 265., false);
        if t < 10. {
            out.beat = "admire castle as miniature ship approaches";
            out.add("johnny-idle", (t / 2.) as usize % 2, 280., 258., false);
        } else if t < 11.8 {
            out.beat = "cannon shots startle castle builder";
            out.add("johnny-gulp-reaction", 0, 280., 258., true);
        } else if t < 12.6 {
            out.beat = "run for the palm";
            out.walk(t - 11.8, 280., 415., 0.8, false);
            out.layers.last_mut().unwrap().clip = "johnny-run";
        } else if t < 14. {
            out.beat = "climb away from cannon fire";
            let p = Layout::for_background("ISLETEMP.SCR").palm_at((t - 12.6) / 1.4);
            out.add("johnny-climb", (t * 8.) as usize, p.x, p.y, false);
            out.layers.last_mut().unwrap().shadow = false;
        } else if t < 24.5 {
            out.beat = "crouch on palm crown";
            out.add("johnny-kong", 0, 469., 62., false);
            out.layers.last_mut().unwrap().shadow = false;
        } else if t < 36.8 {
            out.beat = "swat at circling miniature aircraft";
            let phase = if t < 25.5 {
                1
            } else {
                [1, 2, 1, 3][((t - 25.5) * 2.) as usize % 4]
            };
            out.add("johnny-kong", phase, 469., 82., false);
            out.layers.last_mut().unwrap().shadow = false;
        } else if t < 38.1 {
            out.beat = "lose balance and tumble into sea";
            let u = (t - 36.8) / 1.3;
            out.add(
                "johnny-dive-sequence",
                [5, 6, 7, 3][((u * 4.) as usize).min(3)],
                mix(469., 600., u),
                mix(40., 305., u * u),
                false,
            );
            let l = out.layers.last_mut().unwrap();
            l.shadow = false;
            l.waterline = Some(302.);
        } else {
            out.beat = "splash below the circling aircraft";
            out.splash = Some((Point::new(600., 302.), t - 38.1));
        }
    }
    if t >= 5.5 {
        out.add("galleon", 0, mix(-90., 225., (t - 5.5) / 4.), 180., false);
        out.layers.last_mut().unwrap().scale = 0.55;
    }
    for launch in [10., 11., 15., 17., 19., 21., 23.] {
        let u = (t - launch) / 0.6;
        if (0.0..1.0).contains(&u) {
            out.projectiles.push(Point::new(
                mix(250., 430., u),
                mix(169., 95., u) - 12. * (u * std::f32::consts::PI).sin(),
            ));
        }
    }
    if t >= 25. {
        for i in 0..3 {
            let a = (t - 25.) * 2.1 + i as f32 * 2.0944;
            out.add(
                "rescue-biplane",
                (t * 8.) as usize % 2,
                469. + a.cos() * 110.,
                36. + a.sin() * 24.,
                a.sin() > 0.,
            );
            let l = out.layers.last_mut().unwrap();
            l.scale = 0.26;
            l.depth = Some(if a.sin() < 0. { 10. } else { 100. });
        }
    }
    out
}

fn plane_rescue(t: f32) -> Frame {
    let mut out = Frame::default();
    if t < 4.5 {
        out.beat = "stroll while distant aircraft passes";
        out.walk(t, 350., 405., 4.5, false);
    } else if t < 6.5 {
        out.beat = "jump and wave at aircraft";
        out.add(
            "johnny-help",
            (t * 3.) as usize % 2,
            405.,
            258. - 18. * ((t - 4.5) * std::f32::consts::PI * 1.5).sin().abs(),
            false,
        );
    } else if t < 7.2 {
        out.beat = "shake a coconut loose";
        let phase = (t * 4.) as usize % 2;
        let hand_y = if phase == 0 { 130. } else { 155. };
        out.add(
            "johnny-tree-shake",
            phase,
            415.,
            258. - (370. - hand_y) / 5.3,
            false,
        );
        out.layers.last_mut().unwrap().shadow = false;
    } else if t < 8. {
        out.beat = "watch falling coconut";
        out.add("johnny-coconut-actions", 0, 380., 258., false);
    } else if t < 8.4 {
        out.beat = "pick up coconut to signal plane";
        out.add("johnny-coconut-actions", 1, 380., 258., false);
    } else if t < 9. {
        out.beat = "hold coconut";
        out.add("johnny-coconut-throwing", 0, 380., 258., false);
    } else if t < 9.7 {
        out.beat = "wind up coconut throw";
        out.add(
            "johnny-coconut-throwing",
            if t < 9.4 { 1 } else { 2 },
            380.,
            258.,
            false,
        );
    } else if t < 10.65 {
        out.beat = "throw coconut at passing plane";
        out.add("johnny-coconut-throwing", 3, 380., 258., false);
    } else if t < 11.5 {
        out.beat = "realize plane is damaged";
        out.add("johnny-coconut-throwing", 4, 380., 258., false);
    } else {
        out.beat = "pilot parachutes while Johnny regrets missed rescue";
        out.add("johnny-coconut-throwing", 7, 380., 264., false);
    }
    if t < 8. {
        let u = ((t - 7.2) / 0.8).clamp(0., 1.);
        out.add(
            "coconut-whole",
            0,
            mix(459., 403., u),
            mix(91., 258., u * u),
            false,
        );
    } else if (9.7..10.35).contains(&t) {
        let u = (t - 9.7) / 0.65;
        out.add(
            "coconut-whole",
            0,
            mix(407., 586., u),
            mix(205., 104., u) - 30. * (std::f32::consts::PI * u).sin(),
            false,
        );
    }
    if t < 6. || (7.0..11.6).contains(&t) {
        let (x, y, scale, phase) = if t < 6. {
            (mix(-60., 740., t / 6.), 45., 0.3, (t * 8.) as usize % 2)
        } else if t < 10.35 {
            (
                mix(-100., 548., (t - 7.) / 3.35),
                105.,
                1.,
                (t * 8.) as usize % 2,
            )
        } else if t < 10.65 {
            (548., 105., 1., 2)
        } else {
            let u = (t - 10.65) / 0.95;
            (mix(548., 690., u), mix(105., 325., u * u), 1., 3)
        };
        // Draw the small pilot behind the fuselage, so only the upper body
        // appears above the cockpit until the bailout.
        if t < 10.35 {
            out.add("pilot-fall", 0, x, y - 2. * scale, false);
            let l = out.layers.last_mut().unwrap();
            l.scale = 0.35 * scale;
            l.depth = Some(90.);
        }
        out.add("rescue-biplane", phase, x, y, false);
        let l = out.layers.last_mut().unwrap();
        l.scale = scale;
        l.depth = Some(100.);
    }
    if (11.6..12.2).contains(&t) {
        out.splash = Some((Point::new(690., 305.), t - 11.6));
    }
    if t >= 10.4 {
        let (clip, x, y) = if t < 10.7 {
            ("pilot-fall", 548., mix(105., 133., (t - 10.4) / 0.3))
        } else if t < 11.0 {
            ("pilot-deploy", 550., mix(133., 145., (t - 10.7) / 0.3))
        } else {
            let u = ((t - 11.) / 2.15).clamp(0., 1.);
            ("pilot-parachute", mix(550., 590., u), mix(145., 312., u))
        };
        out.add(clip, 0, x, y, false);
        let l = out.layers.last_mut().unwrap();
        l.scale = 0.55;
        l.waterline = Some(300.);
        l.depth = Some(310.);
    }
    out
}

fn ambient(t: f32) -> Frame {
    let mut out = Frame::default();
    out.beat = "idle and look over the sea";
    let (clip, phase) = if (8.0..10.0).contains(&t) || (26.0..28.0).contains(&t) {
        ("johnny-look", 0)
    } else if (18.0..21.0).contains(&t) {
        ("johnny-thinking", 0)
    } else {
        ("johnny-idle", (t / 2.) as usize % 2)
    };
    out.add(clip, phase, 390., 258., false);
    out
}

fn close_ship(t: f32) -> Frame {
    let mut out = Frame::default();
    if t < 3. {
        out.beat = "walk to watch distant ship";
        out.walk(t, 390., 165., 3., false);
    } else if t < 4. {
        out.beat = "notice distant ship";
        out.add("johnny-look", 0, 165., 258., true);
    } else if t < 7.5 {
        out.beat = "jump and wave for rescue";
        out.add(
            "johnny-help",
            (t * 3.) as usize % 2,
            165.,
            258. - 18. * ((t - 4.) * std::f32::consts::PI * 1.5).sin().abs(),
            true,
        );
    } else if t < 9. {
        out.beat = "realize ship is coming too close";
        out.add("johnny-gulp-reaction", 0, 165., 258., true);
    } else if t < 10.3 {
        out.beat = "run from approaching bow";
        out.walk(t - 9., 165., 535., 1.3, false);
        out.layers.last_mut().unwrap().clip = "johnny-run";
    } else if t < 11.3 {
        out.beat = "jump clear of passing ship";
        let u = t - 10.3;
        out.add(
            "johnny-dive-jump",
            0,
            mix(535., 760., u),
            258. - 75. * (std::f32::consts::PI * u * 0.7).sin(),
            false,
        );
        out.layers.last_mut().unwrap().shadow = false;
    } else {
        out.beat = "huge ship passes across foreground";
    }
    if t >= 2.5 {
        let (x, y, scale) = if t < 7.5 {
            (mix(-70., 145., (t - 2.5) / 5.), 180., 0.25)
        } else if t < 10.5 {
            let u = (t - 7.5) / 3.;
            (
                mix(145., -550., u * u),
                mix(180., 500., u * u),
                mix(0.25, 8., u * u),
            )
        } else {
            (mix(-550., 620., (t - 10.5) / 2.72), 500., 8.)
        };
        out.add("tanker", 0, x, y, false);
        let l = out.layers.last_mut().unwrap();
        l.scale = scale;
        l.depth = Some(if t < 10.5 { 170. } else { 1000. });
    }
    out
}

fn shark_gulp(t: f32) -> Frame {
    let mut out = Frame::default();
    if t < 2.8 {
        out.beat = "carry fishing pole to shore";
        out.walk(t, 410., 525., 2.8, true);
    } else if t < 3.4 {
        out.beat = "shark leaps toward fisherman";
        out.add("johnny-gulp-reaction", 0, 525., 258., false);
    } else if t < 6.5 {
        out.beat = if t < 4.4 {
            "shark swallows fisherman"
        } else {
            "shark cannot stomach fisherman"
        };
    } else if t < 7.3 {
        out.beat = "shark spits Johnny safely onto sand";
        let u = (t - 6.5) / 0.8;
        out.add(
            "johnny-gulp-reaction",
            1,
            mix(502., 455., u),
            mix(232., 255., u) - 12. * (std::f32::consts::PI * u).sin(),
            false,
        );
        out.layers[0].shadow = false;
    } else {
        out.beat = if t < 8.8 {
            "cough after shark escape"
        } else {
            "sit dazed after shark leaves"
        };
        out.add(
            "johnny-gulp-reaction",
            if t < 8.8 { 2 } else { 3 },
            455.,
            264.,
            false,
        );
    }
    if t >= 2.8 {
        out.add("fallen-fishing-pole", 0, 530., 266., false);
    }
    if (2.9..8.0).contains(&t) {
        let (phase, x, y) = if t < 3.2 {
            (
                0,
                mix(625., 550., (t - 2.9) / 0.3),
                mix(325., 270., (t - 2.9) / 0.3),
            )
        } else if t < 3.6 {
            (1, mix(550., 540., (t - 3.2) / 0.4), 267.)
        } else if t < 4.4 {
            (2, 540., 267.)
        } else if t < 5.1 {
            (3, 540., 267.)
        } else if t < 6.5 {
            (4, 540., 267.)
        } else if t < 7.0 {
            (5, 540., 267.)
        } else if t < 7.4 {
            (
                6,
                mix(540., 580., (t - 7.) / 0.4),
                mix(267., 300., (t - 7.) / 0.4),
            )
        } else {
            (
                7,
                mix(580., 625., (t - 7.4) / 0.6),
                mix(300., 395., (t - 7.4) / 0.6),
            )
        };
        out.add("shark-gulp", phase, x, y, false);
        let l = out.layers.last_mut().unwrap();
        l.depth = Some(if t < 6.5 { 280. } else { 245. });
        if t < 3.2 || t >= 7.4 {
            l.waterline = Some(304.);
        }
        if t >= 7.4 {
            out.splash = Some((Point::new(595., 304.), t - 7.4));
        }
    }
    out
}

fn coconut_throw(t: f32) -> Frame {
    if t < 12. {
        let mut out = coconuts(t);
        for l in &mut out.layers {
            if l.clip == "coconut-whole" {
                l.point.x -= 35. * ((t - 3.) / 6.6).clamp(0., 1.);
            } else if l.clip != "johnny-tree-shake" {
                l.point.x -= 35.;
            }
        }
        if (3.0..4.0).contains(&t) {
            out.layers.retain(|l| !l.clip.starts_with("johnny-"));
            out.walk(t - 3., 405., 370., 1., false);
        }
        return out;
    }
    let mut out = Frame::default();
    if t < 20. {
        out.beat = "wind up to crack coconut against palm";
        let phase = if t < 13. {
            0
        } else if t < 15. {
            1
        } else {
            2
        };
        out.add("johnny-coconut-throwing", phase, 380., 258., false);
    } else if t < 21. {
        out.beat = "throw coconut against palm";
        out.add("johnny-coconut-throwing", 3, 380., 258., false);
        if t < 20.2 {
            out.add(
                "coconut-whole",
                0,
                mix(407., 432., (t - 20.) / 0.2),
                mix(205., 202., (t - 20.) / 0.2),
                false,
            );
        } else {
            let u = ((t - 20.2) / 0.8).clamp(0., 1.);
            out.add(
                "coconut-half",
                0,
                mix(432., 403., u),
                mix(202., 253., u * u),
                false,
            );
            out.add(
                "coconut-half",
                0,
                mix(432., 392., u),
                mix(202., 264., u * u) - 12. * (std::f32::consts::PI * u).sin(),
                true,
            );
        }
    } else if t < 22.5 {
        out.beat = "kneel beside cracked coconut";
        out.add("johnny-rise", 0, 380., 258., false);
    } else if t < 42. {
        out.beat = "drink milk from opened coconut";
        let phase = if t < 23.5 {
            5
        } else {
            6 + ((t - 23.5) / 1.7) as usize % 2
        };
        out.add("johnny-coconut-actions", phase, 403., 264., false);
        out.layers[0].shadow = false;
    } else if t < 43. {
        out.beat = "stand after coconut drink";
        out.add("johnny-rise", 0, 380., 258., false);
    } else {
        out.beat = "leave opened shells beside rock";
        out.walk(t - 43., 380., 405., 1., false);
    }
    if t < 21. {
        out.add("cracking-rock", 0, 403., 264., false);
    } else if t < 22.5 || t >= 42. {
        out.add("cracking-rock-halves", 0, 403., 264., false);
    }
    out
}

fn bathing(t: f32) -> Frame {
    let mut out = Frame::default();
    let phase = if t < 12.5 {
        [0, 1, 2, 1][(t * 1.4) as usize % 4]
    } else if t < 14. {
        3
    } else if t < 18. {
        4
    } else if t < 21. {
        5
    } else {
        6
    };
    out.beat = if t < 12.5 {
        "wash hair in sea"
    } else if t < 14. {
        "notice gull stealing shorts"
    } else if t < 21. {
        "protest as gull takes shorts to palm"
    } else {
        "wait in water without clothes"
    };
    out.add("johnny-bath-theft", phase, 112., 303., false);
    out.layers[0].shadow = false;
    out.layers[0].waterline = Some(300.);
    out.splash = Some((Point::new(112., 300.), 0.5));
    out.add(
        if t < 13.2 {
            "clothes-stick"
        } else {
            "empty-clothes-stick"
        },
        0,
        150.,
        270.,
        false,
    );
    if t < 2.5 {
        out.add(
            "gull",
            (t * 8.) as usize,
            mix(720., 469., t / 2.5),
            mix(95., 52., t / 2.5),
            true,
        );
        out.layers.last_mut().unwrap().scale = 0.8;
    } else if t < 9. {
        out.add("gull-perch", 0, 469., 52., false);
    } else if t < 13.2 {
        let u = (t - 9.) / 4.2;
        out.add(
            "gull",
            (t * 8.) as usize,
            mix(469., 160., u),
            mix(52., 243., u) - 65. * (std::f32::consts::PI * u).sin(),
            true,
        );
        out.layers.last_mut().unwrap().scale = 0.8;
    } else if t < 17. {
        let u = (t - 13.2) / 3.8;
        out.add(
            "gull-shorts",
            (t * 6.) as usize % 2,
            mix(160., 469., u),
            mix(250., 52., u),
            false,
        );
    } else {
        out.add("gull-shorts", 2, 469., 52., false);
    }
    out
}

fn spyglass(t: f32, passing_boat: bool) -> Frame {
    let mut out = Frame::default();
    let (phase, left) = if passing_boat {
        if t < 0.4 {
            (0, true)
        } else if t < 0.8 {
            (1, true)
        } else if t < 3.8 {
            (2, true)
        } else if t < 4.3 {
            (0, false)
        } else if t < 6.8 {
            (2, false)
        } else if t < 7.7 {
            (5, false)
        } else if t < 8.6 {
            (6, false)
        } else {
            (7, false)
        }
    } else {
        if t < 0.4 {
            (0, true)
        } else if t < 0.7 {
            (1, true)
        } else if t < 1.8 {
            (2, true)
        } else if t < 2.6 {
            (3, true)
        } else if t < 3.3 {
            (4, true)
        } else if t < 3.7 {
            (0, false)
        } else if t < 4.7 {
            (2, false)
        } else if t < 5.4 {
            (3, false)
        } else if t < 6.4 {
            (4, false)
        } else {
            (5, false)
        }
    };
    out.beat = if phase < 2 {
        "raise telescope"
    } else if phase < 5 {
        "scan horizon through telescope"
    } else {
        "lower telescope and react"
    };
    out.add(
        "johnny-spyglass",
        phase,
        if passing_boat { 165. } else { 515. },
        258.,
        left,
    );
    if passing_boat && (2.4..6.4).contains(&t) {
        out.beat = "follow passing boat with telescope";
        let x = mix(740., -100., (t - 2.4) / 4.);
        out.add("clown-boat", (t * 5.) as usize % 4, x, 182., false);
        out.layers.last_mut().unwrap().scale = 1.0;
    }
    out
}

fn gull_visit(t: f32) -> Frame {
    let mut out = Frame::default();
    // The standalone gull helper needs the reading actor supplied by its parent.
    // Its central joke is the bird stealing and READING Johnny's book.
    if t < 12. {
        out.beat = "read while gull watches";
        out.add("johnny-read", (t / 2.5) as usize % 4, 400., 264., false);
    } else if t < 14.4 {
        out.beat = "notice diving gull";
        out.add("johnny-book-theft", 1, 400., 264., false);
    } else if t < 16. {
        out.beat = "gull takes book from hands";
        out.add("johnny-book-theft", 2, 400., 264., false);
    } else if t < 18. {
        out.beat = "reach after stolen book";
        out.add("johnny-book-theft", 3, 400., 264., false);
    } else if t < 20. {
        out.beat = "stand to demand book back";
        out.add("reading-log", 0, 400., 264., false);
        out.add("johnny-book-theft", 4, 400., 264., false);
    } else if t < 23. || (35.0..38.).contains(&t) {
        out.beat = "scold bird reading stolen book";
        out.add("reading-log", 0, 400., 264., false);
        out.add("johnny-book-theft", 5, 400., 264., false);
    } else if t < 24. || (34.2..35.).contains(&t) {
        out.beat = "sit beside empty reading log";
        out.add("reading-log", 0, 400., 264., false);
        out.add("johnny-rise", 0, 380., 264., false);
    } else {
        out.beat = "wait while gull keeps book";
        out.add("reading-log", 0, 400., 264., false);
        out.add("johnny-coconut-throwing", 7, 380., 264., false);
    }
    if t < 4.2 {
        out.add(
            "gull",
            (t * 8.) as usize,
            mix(-40., 469., t / 4.2),
            mix(38., 52., t / 4.2),
            false,
        );
        out.layers.last_mut().unwrap().scale = 0.8;
    } else if t < 12. {
        out.add("gull-perch", (t / 1.5) as usize % 2, 469., 52., false);
    } else if t < 14.4 {
        let u = (t - 12.) / 2.4;
        out.add(
            "gull",
            (t * 8.) as usize,
            mix(469., 416., u) - 90. * (std::f32::consts::PI * u).sin(),
            mix(52., 212., u),
            false,
        );
        out.layers.last_mut().unwrap().scale = 0.8;
    } else if t < 18. {
        let u = (t - 14.4) / 3.6;
        out.add(
            "gull-book",
            (t * 6.) as usize % 3,
            mix(416., 469., u) + 100. * (std::f32::consts::PI * u).sin(),
            mix(228., 52., u),
            false,
        );
        out.layers.last_mut().unwrap().depth = Some(300.);
    } else {
        out.add(
            "gull-reading",
            if t < 18.6 {
                0
            } else if t < 19.3 {
                1
            } else {
                2 + ((t - 19.3) / 1.8) as usize % 2
            },
            469.,
            52.,
            false,
        );
    }
    out
}

fn lilliput_ship(t: f32) -> Frame {
    let mut out = Frame::default();
    out.beat = "small galleon sails past the island";
    if t < 3.5 {
        out.walk(t, 390., 335., 3.5, false);
    } else {
        out.add("johnny-rest", 0, 335., 258., false);
    }
    if (4.0..18.3).contains(&t) {
        let x = if t < 9. {
            mix(-100., 160., (t - 4.) / 5.)
        } else if t < 14. {
            160.
        } else {
            mix(160., -100., (t - 14.) / 4.3)
        };
        out.add("galleon", 0, x, 183. + (t * 1.2).sin() * 0.7, t >= 14.);
        out.layers.last_mut().unwrap().scale = 0.65;
    }
    out
}

fn fire_helper(t: f32) -> Frame {
    let mut out = Frame::default();
    out.beat = "campfire supporting animation";
    if t < 5.5 {
        out.add("driftwood", 0, 350., 261., false);
    } else if t < 31. {
        out.add("campfire", (t * 8.) as usize, 350., 261., false);
    } else {
        out.add("driftwood", 0, 350., 261., false);
        out.smoke = Some(Point::new(350., 251.));
    }
    out
}

fn coconuts(t: f32) -> Frame {
    let mut out = Frame::default();
    if t < 3. {
        out.beat = "shake palm to loosen coconut";
        let phase = (t * 4.) as usize % 2;
        let hand_y = if phase == 0 { 130. } else { 155. };
        out.add(
            "johnny-tree-shake",
            phase,
            415.,
            258. - (370. - hand_y) / 5.3,
            false,
        );
        out.layers[0].shadow = false;
    } else if t < 9.6 {
        out.beat = "watch coconut bounce";
        out.add("johnny-coconut-actions", 0, 405., 258., false);
    } else if t < 10.5 {
        out.beat = "pick up coconut";
        out.add("johnny-coconut-actions", 1, 415., 258., false);
    } else if t < 12. {
        out.beat = "hold coconut beside cracking rock";
        out.add("johnny-coconut-actions", 2, 415., 258., false);
    } else if t >= 26.5 {
        out.beat = "stand after drinking";
        out.add(
            if t < 27.2 {
                "johnny-rise"
            } else {
                "johnny-rest"
            },
            0,
            415.,
            258.,
            false,
        );
    } else {
        let (beat, phase) = if t < 14. {
            ("brace coconut over rock", 3)
        } else if t < 15. {
            (
                "crack coconut",
                if ((t - 14.) * 4.) as usize % 2 == 0 {
                    4
                } else {
                    3
                },
            )
        } else if t < 16. {
            ("inspect opened coconut", 5)
        } else if t < 17. {
            ("raise coconut half", 6)
        } else if t < 25. {
            ("drink coconut milk", 6 + ((t - 17.) / 1.5) as usize % 2)
        } else {
            ("rest with coconut halves", 6)
        };
        out.beat = beat;
        out.add("johnny-coconut-actions", phase, 438., 264., false);
        out.layers[0].shadow = false;
    }
    if t < 9.6 {
        let (x, y) = if t < 3. {
            (459. + (t * 14.).sin() * 1.2, 91.)
        } else if t < 3.7 {
            let a = (t - 3.) / 0.7;
            (mix(459., 365., a), mix(91., 258., a * a))
        } else {
            let a = ((t - 3.7) / 5.9).clamp(0., 1.);
            (
                mix(365., 438., a),
                258. - 45. * (1. - a) * (a * std::f32::consts::PI * 5.).sin().abs(),
            )
        };
        out.add("coconut-whole", 0, x, y, false);
    }
    if t < 12. || t >= 26.5 {
        out.add(
            if t < 12. {
                "cracking-rock"
            } else {
                "cracking-rock-halves"
            },
            0,
            438.,
            264.,
            false,
        );
    }
    out
}

fn octopus_catch(t: f32) -> Frame {
    let mut out = Frame::default();
    if t < 2.8 {
        out.beat = "carry pole to shore";
        out.walk(t, 410., 145., 2.8, true);
    } else if t < 20. {
        let ends = [8.8, 11.2, 13.6, 16., 18.4, 20.];
        let cycle = ends.iter().position(|&end| t < end).unwrap_or(5);
        let start = [2.8, 8.8, 11.2, 13.6, 16., 18.4][cycle];
        let u = t - start;
        let cast = if cycle == 0 { 1.2 } else { 0.6 };
        let pull = if cycle == 0 {
            3.4
        } else if cycle == 5 {
            0.9
        } else {
            0.8
        };
        let pull_duration = if cycle == 0 { 1. } else { 0.6 };
        let lift_end = if cycle == 0 { 5.2 } else { 1.8 };
        let release = if cycle == 0 { 5.5 } else { 2. };
        let (clip, phase) = if u < cast {
            out.beat = "cast for catch";
            ("johnny-fishing-cast", ((u / cast * 6.) as usize).min(5))
        } else if u < pull {
            out.beat = "wait for catch";
            ("johnny-fishing-cast", 5)
        } else if u < pull + pull_duration || cycle == 5 {
            out.beat = "reel in catch";
            ("johnny-fishing-cast", 6 + ((u - pull) * 5.) as usize % 2)
        } else if u < lift_end {
            out.beat = "lift small fish";
            ("johnny-fishing-gag", 4)
        } else {
            out.beat = "toss fish onto sand";
            ("johnny-fishing-gag", if u < release { 6 } else { 7 })
        };
        out.add(clip, phase, 145., 258., out.beat != "toss fish onto sand");
        if clip == "johnny-fishing-cast" {
            let a = ((u - cast * 0.5) / (cast * 0.5)).clamp(0., 1.);
            let mut end = Point::new(
                mix(95., 23., a),
                mix(206., 298., a) - 42. * (std::f32::consts::PI * a).sin(),
            );
            if u >= pull {
                let a = ((u - pull) / pull_duration).clamp(0., 1.);
                end = Point::new(mix(23., 117., a), mix(298., 212., a));
            }
            out.line = Some(FishingLine {
                actor: 0,
                end,
                taut: u >= pull,
                in_water: u >= cast && u < pull,
            });
        }
        if cycle < 5 && u >= release {
            let a = ((u - release) / (ends[cycle] - start - release)).clamp(0., 1.);
            out.add(
                "fish-green",
                (a * 8.) as usize % 3,
                mix(175., 195. + cycle as f32 * 11., a),
                mix(217., 258., a) - 28. * (std::f32::consts::PI * a).sin(),
                false,
            );
        }
    } else if t < 21.3 {
        out.beat = "octopus grabs Johnny";
        out.add(
            "johnny-octopus",
            ((t - 20.) * 4.) as usize % 2,
            145.,
            258.,
            false,
        );
    } else if t < 23. {
        out.beat = "run from octopus";
        out.walk(t - 21.3, 145., 415., 1.7, false);
        out.layers[0].clip = "johnny-run";
    } else if t < 25. {
        out.beat = "escape up palm";
        let p = Layout::for_background("ISLETEMP.SCR").palm_at((t - 23.) / 2. * 0.65);
        out.add("johnny-climb", ((t - 23.) * 7.) as usize, p.x, p.y, false);
        out.layers[0].shadow = false;
    } else if t < 26.5 {
        out.beat = "wait above octopus";
        let p = Layout::for_background("ISLETEMP.SCR").palm_at(0.65);
        out.add("johnny-climb", 0, p.x, p.y, false);
        out.layers[0].shadow = false;
    } else if t < 28. {
        out.beat = "climb down after octopus leaves";
        let p = Layout::for_background("ISLETEMP.SCR").palm_at(0.65 * (1. - (t - 26.5) / 1.5));
        out.add("johnny-climb", ((t - 26.5) * 7.) as usize, p.x, p.y, false);
        out.layers[0].shadow = false;
    } else if t < 28.5 {
        out.beat = "return feet to sand";
        out.add(
            "johnny-climb",
            2,
            415.,
            mix(225., 258., (t - 28.) / 0.5),
            false,
        );
    } else {
        out.beat = "shake fist after octopus escapes";
        out.add("johnny-book-theft", 5, 415., 258., true);
    }
    for (i, end) in [8.8, 11.2, 13.6, 16., 18.4].iter().enumerate() {
        if t >= *end {
            out.add("fish-green", i % 3, 195. + i as f32 * 11., 258., false);
        }
    }
    if t >= 20. {
        out.add("fallen-fishing-pole", 0, 157., 264., true);
    }
    if (21.3..26.5).contains(&t) {
        let a = ((t - 21.3) / 4.).clamp(0., 1.);
        let x = mix(145., 600., a);
        let y = if x < 530. {
            253.
        } else {
            mix(253., 327., (x - 530.) / 70.)
        };
        out.add("octopus", ((t - 21.3) * 7.) as usize, x, y, false);
        let l = out.layers.last_mut().unwrap();
        l.depth = Some(268.);
        if x >= 530. {
            l.waterline = Some(294.);
        }
        if t >= 25.3 {
            out.splash = Some((Point::new(600., 294.), t - 25.3));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    #[test]
    fn every_browser_story_uses_registered_complete_pose_art() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let assets = crate::resources::AssetStore::load(root.join("original")).unwrap();
        let raw = assets
            .raw
            .iter()
            .filter(|(name, _)| name.ends_with(".TTM"))
            .map(|(name, bytes)| crate::animation::Clip::parse(name, bytes).unwrap())
            .collect::<Vec<_>>();
        let clips = crate::scheduler::build_hd2d_clips(&assets, &raw).unwrap();
        let catalog = std::fs::read_to_string(root.join("assets/hd2d-v2/scene-art.tsv")).unwrap();
        let names: BTreeSet<_> = catalog
            .lines()
            .skip(1)
            .map(|row| row.split('\t').next().unwrap())
            .collect();
        assert_eq!(clips.len(), 41);
        for clip in clips {
            for tick in 1..=clip.frames.len() {
                if clip.name == "THEEND.TTM" {
                    for layer in crate::hd2d_ending::frame(tick as f32 / 60.).layers {
                        assert!(names.contains(layer.clip), "ending: {}", layer.clip);
                    }
                    continue;
                }
                let story = frame(&clip.name, tick)
                    .unwrap_or_else(|| panic!("{} fell back to generic source poses", clip.name));
                assert!(!story.beat.is_empty(), "{} tick {tick}", clip.name);
                for layer in &story.layers {
                    assert!(names.contains(layer.clip), "{}: {}", clip.name, layer.clip);
                    assert!(layer.point.x.is_finite() && layer.point.y.is_finite());
                    assert!(layer.scale.is_finite() && layer.scale > 0.);
                }
                if let Some(line) = story.line {
                    assert!(line.actor < story.layers.len());
                }
                for (index, _) in story.scores {
                    assert!(index < story.layers.len());
                }
            }
        }
    }
    #[test]
    fn fishing_has_two_complete_catches_and_one_visible_actor() {
        for (name, ticks) in [("MJFISH.TTM", 1950), ("MJFISHC.TTM", 1690)] {
            let mut last = "";
            let mut casts = 0;
            let mut releases = 0;
            let mut inspected = false;
            for tick in 1..=ticks {
                let f = frame(name, tick).unwrap();
                assert_eq!(
                    f.layers
                        .iter()
                        .filter(|l| l.clip.starts_with("johnny-"))
                        .count(),
                    1
                );
                if f.beat != last {
                    if f.beat == "cast" {
                        casts += 1;
                        inspected = false;
                    }
                    if f.beat == "inspect small catch" {
                        inspected = true;
                    }
                    if f.beat == "throw fish back" {
                        assert!(inspected);
                        releases += 1;
                    }
                }
                if let Some(line) = f.line {
                    assert_eq!(f.layers[line.actor].clip, "johnny-fishing-cast");
                }
                if f.beat == "carry pole back to shade" {
                    assert_eq!(releases, 2);
                }
                last = f.beat;
            }
            assert_eq!(casts, 2);
            assert_eq!(releases, 2);
        }
    }
    #[test]
    fn campfire_meal_is_causal_and_reaches_recovery() {
        let mut seen = BTreeSet::new();
        for tick in 1..=3297 {
            let f = frame("MJFIRE.TTM", tick).unwrap();
            assert_eq!(
                f.layers
                    .iter()
                    .filter(|l| l.clip.starts_with("johnny-"))
                    .count(),
                1
            );
            if f.beat == "roast fish over fire" {
                assert!(seen.contains("fire catches"));
            }
            if f.beat == "breathe flame" {
                assert!(seen.contains("bite hot fish"));
            }
            if f.beat == "return to shade" {
                assert!(seen.contains("recover"));
            }
            seen.insert(f.beat);
        }
        assert!(seen.contains("return to shade"));
    }
}
