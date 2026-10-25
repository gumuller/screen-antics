//! Semantic interpretation of source visuals. Original indexed pixels are used
//! only to classify authored components, never as textures in the new renderer.
use crate::resources::Image;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Support {
    Ground,
    Sky,
    Water,
    Tree,
    Contact,
    Pair,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Hero,
    Suzy,
    Mermaid,
    Crew,
    Pair,
    Prop,
    Vehicle,
    Animal,
}
#[derive(Clone, Copy, Debug)]
pub struct Binding {
    pub clip: &'static str,
    pub support: Support,
    pub group: Group,
    pub priority: u8,
}
#[derive(Clone, Copy, Debug)]
pub enum Role {
    Sprite(Binding),
    PaintedScenery,
    IntegratedPart,
    WaterEffect,
    Smoke,
    Idea,
    Question,
    SleepText,
    Spark,
    Projectile,
    Score(&'static str),
    Unknown,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Shape {
    pub bbox: [u32; 4],
    pub opaque: usize,
    pub red: f32,
    pub gray: f32,
    pub green: f32,
    pub foot_x: f32,
}
impl Shape {
    pub fn analyze(image: &Image, palette: &[[u8; 3]; 256]) -> Self {
        let mut shape = Self {
            bbox: [image.width, image.height, 0, 0],
            ..Self::default()
        };
        for y in 0..image.height {
            for x in 0..image.width {
                let color = image.pixels[(y * image.width + x) as usize];
                if color == 0 {
                    continue;
                }
                shape.opaque += 1;
                let [r, g, b] = palette[color as usize].map(|v| v as f32);
                shape.red += if r > g * 1.35 && r > b * 1.35 { 1. } else { 0. };
                shape.gray += if r.max(g).max(b) - r.min(g).min(b) < 30. && r > 70. {
                    1.
                } else {
                    0.
                };
                shape.green += if g > r * 1.3 && g > b * 1.2 { 1. } else { 0. };
                shape.bbox[0] = shape.bbox[0].min(x);
                shape.bbox[1] = shape.bbox[1].min(y);
                shape.bbox[2] = shape.bbox[2].max(x + 1);
                shape.bbox[3] = shape.bbox[3].max(y + 1);
            }
        }
        let count = shape.opaque.max(1) as f32;
        shape.red /= count;
        shape.gray /= count;
        shape.green /= count;
        let mut total = 0.;
        for y in shape.bbox[3].saturating_sub((shape.height() / 10).max(3))..shape.bbox[3] {
            for x in 0..image.width {
                if image.pixels[(y * image.width + x) as usize] != 0 {
                    shape.foot_x += x as f32 + 0.5;
                    total += 1.;
                }
            }
        }
        if total > 0. {
            shape.foot_x /= total;
        }
        shape
    }
    pub fn width(self) -> u32 {
        self.bbox[2].saturating_sub(self.bbox[0])
    }
    pub fn height(self) -> u32 {
        self.bbox[3].saturating_sub(self.bbox[1])
    }
    pub fn hero(self) -> bool {
        self.height() >= 30
            && self.red > 0.025
            && self.gray > 0.03
            && self.width() < self.height() * 3
    }
}
fn sprite(clip: &'static str, support: Support, group: Group, priority: u8) -> Role {
    Role::Sprite(Binding {
        clip,
        support,
        group,
        priority,
    })
}
fn hero(clip: &'static str) -> Role {
    sprite(clip, Support::Ground, Group::Hero, 10)
}
fn prop(clip: &'static str, support: Support) -> Role {
    sprite(clip, support, Group::Prop, 1)
}
fn animal(clip: &'static str, support: Support) -> Role {
    sprite(clip, support, Group::Animal, 2)
}
fn pair(clip: &'static str) -> Role {
    sprite(clip, Support::Pair, Group::Pair, 30)
}
pub fn bind(name: &str, frame: usize, s: Shape) -> Role {
    use Support::*;
    if s.opaque == 0 {
        return Role::IntegratedPart;
    }
    match name {
        "BACKGRND.BMP" | "CLOUDS.BMP" | "TRUNK.BMP" | "ENDCRDTS.BMP" => Role::PaintedScenery,
        "JOHNWALK.BMP" | "MEXCWALK.BMP" => hero("johnny-walk"),
        "MJJOG1.BMP" => hero(match frame {
            5 => "johnny-rise",
            6 => "johnny-bend",
            7..=9 => "johnny-stretch-left",
            10..=13 => "johnny-stretch-right",
            1..=4 => "johnny-stretch-up",
            _ => "johnny-rest",
        }),
        "MJJOG2.BMP" => hero("johnny-run"),
        "MJREAD.BMP" => match frame {
            0..=19 | 21..=23 | 26 => hero("johnny-read"),
            27..=28 => hero("johnny-land"),
            29 => hero("johnny-rise"),
            30 => hero("johnny-book-stand"),
            31..=34 => Role::WaterEffect,
            35 => Role::Spark,
            _ => Role::IntegratedPart,
        },
        "MJTELE.BMP" => hero(if frame < 5 {
            "johnny-idle"
        } else {
            "johnny-telescope"
        }),
        "MJTELE2.BMP" => {
            if s.height() >= 25 {
                hero("johnny-telescope")
            } else {
                Role::IntegratedPart
            }
        }
        "MJCOCO.BMP" => match frame {
            0..=1 => hero("johnny-coconut-throw"),
            2..=6 | 8..=9 | 15 => hero("johnny-coconut-pick"),
            10 | 14 | 32 => hero("johnny-coconut-hold"),
            11..=13 => hero("johnny-coconut-wind"),
            20 => hero("johnny-coconut-dizzy"),
            21..=23 | 31 => hero("johnny-coconut-hit"),
            36..=37 => hero("johnny-coconut-drink"),
            _ => Role::IntegratedPart,
        },
        "COCOHEAD.BMP" => {
            if frame == 8 {
                hero("johnny-coconut-hit")
            } else {
                Role::IntegratedPart
            }
        }
        "COCONUTS.BMP" => prop(if frame < 4 { "coconut" } else { "coconut-open" }, Contact),
        "DRUNKJON.BMP" => {
            if s.height() >= 28 {
                hero(if frame < 9 {
                    "johnny-drink"
                } else if frame < 18 {
                    "johnny-fall"
                } else {
                    "johnny-recover"
                })
            } else {
                prop("bottle", Contact)
            }
        }
        "MJBATH.BMP" => match frame {
            0..=16 | 35..=36 => sprite("johnny-bath", Water, Group::Hero, 10),
            17..=28 => hero("johnny-walk"),
            30..=31 => Role::WaterEffect,
            _ => Role::IntegratedPart,
        },
        "MJDIVE.BMP" => match frame {
            1..=6 => sprite("johnny-climb", Tree, Group::Hero, 10),
            9..=11 => sprite("johnny-dive-ready", Tree, Group::Hero, 10),
            12..=13 | 21 => sprite("johnny-dive", Water, Group::Hero, 10),
            14..=15 | 23 => sprite("johnny-dive-down", Water, Group::Hero, 10),
            16..=18 => Role::WaterEffect,
            _ => Role::PaintedScenery,
        },
        "MJBOTTLE.BMP" => {
            if s.hero() {
                hero("johnny-reach")
            } else if s.opaque > 8 {
                prop("bottle", Water)
            } else {
                Role::WaterEffect
            }
        }
        "MJBTL2.BMP" => {
            if s.hero() {
                hero("johnny-drink")
            } else {
                prop("bottle", Contact)
            }
        }
        "FISHMAN.BMP" | "GJCATCH1.BMP" => hero("johnny-fishing-cast"),
        "MJFISH1.BMP" => match frame {
            0..=21 | 27 => hero("johnny-fishing-cast"),
            24..=26 => Role::WaterEffect,
            _ => Role::IntegratedPart,
        },
        "MJFISH2.BMP" => match frame {
            0..=2 | 4..=5 => sprite("johnny-fishing-gag", Ground, Group::Hero, 20),
            6..=12 => hero("johnny-fishing-gag"),
            3 | 13..=14 => hero("johnny-fishing-cast"),
            _ => Role::IntegratedPart,
        },
        "MJFISH3.BMP" => {
            if s.hero() {
                hero("johnny-scared")
            } else if s.height() > 30 && s.red > 0.4 {
                animal("octopus", Water)
            } else {
                prop("fish-tropical", Contact)
            }
        }
        "GJCATCH2.BMP" => match frame {
            0 | 3 => animal("octopus", Water),
            1 | 4 => hero("johnny-fish-pull"),
            2 => prop("fish-tropical", Ground),
            7..=9 | 12..=19 => hero("johnny-octopus"),
            20..=21 => hero("johnny-fall"),
            24 => hero("johnny-land"),
            25..=39 => animal("octopus", Ground),
            10 => prop("fishing-rod", Ground),
            _ => Role::WaterEffect,
        },
        "GJCATCH3.BMP" => {
            if s.hero() {
                hero("johnny-fish")
            } else {
                prop("fishing-rod", Contact)
            }
        }
        "GJFFFOOD.BMP" => {
            if s.hero() {
                hero("johnny-fish")
            } else if s.gray > 0.25 && s.width() > 25 {
                animal("shark", Water)
            } else {
                Role::WaterEffect
            }
        }
        "GJDIVE.BMP" => match frame {
            0..=1 | 5..=8 => sprite("johnny-dive", Water, Group::Hero, 10),
            2..=4 | 9..=12 => sprite("johnny-dive-down", Water, Group::Hero, 10),
            16..=18 => animal("dolphin", Ground),
            19..=21 => animal("crab", Ground),
            24..=26 => animal("fish-tropical", Ground),
            27..=29 => animal("gull", Ground),
            31..=39 => Role::Score(
                ["5.6", "5.2", "5.1", "7.5", "10", "5.5", "5.8", "5.4", "5.7"][frame - 31],
            ),
            _ => Role::WaterEffect,
        },
        "SHARK.BMP" => match frame {
            0..=1 => hero("johnny-idle"),
            2..=4 => hero("johnny-walk"),
            5 | 7 | 10..=12 => hero("johnny-fall"),
            13..=22 | 38 => hero("johnny-recover"),
            25..=35 => animal("shark", Water),
            23..=24 | 36..=37 => Role::WaterEffect,
            _ => Role::IntegratedPart,
        },
        "SHARKWLK.BMP" => {
            if frame >= 8 {
                hero("johnny-walk")
            } else {
                Role::IntegratedPart
            }
        }
        "GJANGRY.BMP" => {
            if s.hero() {
                hero("johnny-angry")
            } else {
                animal("gull", Sky)
            }
        }
        "GJGULL1.BMP" => {
            if frame >= 29 {
                prop("nest", Tree)
            } else {
                animal("gull", Sky)
            }
        }
        "GJGULL1A.BMP" => match frame {
            0..=38 => animal("gull", Sky),
            39..=40 => hero("johnny-scared"),
            41..=43 => hero("johnny-help"),
            _ => hero("johnny-read"),
        },
        "GJGULL3A.BMP" => match frame {
            16..=20 | 33..=41 => hero("johnny-bird-sit"),
            21..=28 => hero("johnny-bird-swat"),
            42..=46 => animal("gull", Sky),
            _ => Role::IntegratedPart,
        },
        "GJGULL2.BMP" => match frame {
            0..=1 | 6..=15 | 17 | 35 => Role::IntegratedPart,
            _ => animal("gull", Sky),
        },
        "GJGULL2A.BMP" => sprite("johnny-bath", Water, Group::Hero, 10),
        "GJGULL3.BMP" => match frame {
            0..=1 | 8..=16 => hero("johnny-bird-sit"),
            2..=7 => hero("johnny-bird-swat"),
            _ => animal("gull", Sky),
        },
        "GJHOT.BMP" => match frame {
            0..=7 | 10..=11 | 14 | 21..=22 => hero("johnny-hot"),
            15..=16 => hero("johnny-land"),
            17..=19 => hero("johnny-sleep"),
            _ => Role::IntegratedPart, // Fan/arm layers are part of the complete pose.
        },
        "GJKINGKO.BMP" => hero(if frame < 8 {
            "johnny-dig"
        } else {
            "johnny-scared"
        }),
        "MJSANDC.BMP" => hero(if (3..8).contains(&frame) {
            "johnny-run"
        } else {
            "johnny-dig"
        }),
        "GJCASTLE.BMP" => hero(if frame < 4 {
            "johnny-idle"
        } else {
            "johnny-angry"
        }),
        "SANDCAST.BMP" => prop("sandcastle", Ground),
        "MJRAFT2.BMP" => {
            if s.hero() {
                hero("johnny-build")
            } else {
                prop("driftwood", Ground)
            }
        }
        "MRAFT.BMP" => prop("raft-building", Ground),
        "SRAFT.BMP" => prop(if frame == 0 { "raft" } else { "fishing-rod" }, Water),
        "SJRAFT1.BMP" => match frame {
            0..=1 => hero("johnny-build"),
            2 => hero("johnny-rise"),
            3..=4 => hero("johnny-reach"),
            5..=6 => hero("johnny-idle"),
            _ => Role::IntegratedPart,
        },
        "SJWORK.BMP" => {
            if s.hero() {
                hero("johnny-build")
            } else {
                Role::IntegratedPart
            }
        }
        "SLEEP.BMP" => hero("johnny-sleep"),
        "STNDLAY.BMP" => match frame {
            0..=2 => hero("johnny-land"),
            3..=6 => hero("johnny-sleep"),
            7 => hero("johnny-recover"),
            _ => Role::PaintedScenery,
        },
        "FIRE1.BMP" => match frame {
            0 => prop("driftwood", Ground),
            1..=4 | 22..=26 => Role::Smoke,
            5..=21 => prop("campfire", Ground),
            _ => Role::Spark,
        },
        "FIRE2.BMP" | "FIRE5.BMP" => Role::IntegratedPart,
        "FIRE3.BMP" => hero("johnny-firecraft"),
        "FIRE4.BMP" => match frame {
            0..=4 | 7..=8 | 10..=20 | 22 | 24 | 26..=34 => {
                sprite("johnny-hot-meal", Ground, Group::Hero, 20)
            }
            _ => Role::IntegratedPart,
        },
        "FIRE.BMP" => match frame {
            0..=9 | 13..=18 | 22 | 25..=26 | 31..=32 => hero("johnny-firecraft"),
            _ => Role::IntegratedPart,
        },
        "GJNAT1.BMP" => match frame {
            1..=7 | 13..=14 | 17..=19 => hero("johnny-help"),
            8..=12 | 15..=16 | 20..=23 => hero("johnny-wave"),
            _ => Role::IntegratedPart,
        },
        "GJNAT1LI.BMP" => match frame {
            3..=6 => hero("johnny-dizzy"),
            11 => hero("johnny-wave"),
            0 | 2 => Role::Spark,
            8..=10 => Role::Smoke,
            _ => Role::IntegratedPart,
        },
        "GJNAT3.BMP" => match frame {
            // The small upper bodies belong to the passing boat's crew.
            // Treating them as Johnny moved the hero into the sky.
            0..=18 | 46..=49 | 52..=55 => sprite("sailor", Water, Group::Crew, 8),
            19..=31 | 33..=38 | 50..=51 => hero("johnny-wave"),
            39..=40 => hero("johnny-land"),
            45 => Role::Spark,
            _ => Role::IntegratedPart,
        },
        "GJPROW.BMP" => sprite("liner", Water, Group::Vehicle, 5),
        "GJRUNAWA.BMP" => {
            if s.hero() {
                hero("johnny-run")
            } else {
                Role::IntegratedPart
            }
        }
        "GJVIS3.BMP" => match frame {
            0 => sprite("motorboat", Water, Group::Vehicle, 5),
            1..=3 => Role::WaterEffect,
            4 => sprite("helicopter", Sky, Group::Vehicle, 5),
            5 => sprite("biplane", Sky, Group::Vehicle, 5),
            6 => hero("johnny-telescope"),
            _ => Role::IntegratedPart,
        },
        "GJVIS5.BMP" => hero("johnny-help"),
        "GJVIS52.BMP" => match frame {
            0 | 5..=8 => sprite("biplane", Sky, Group::Vehicle, 5),
            9 | 11..=15 => sprite("parachute", Sky, Group::Vehicle, 5),
            10 | 16 => Role::WaterEffect,
            _ => Role::IntegratedPart,
        },
        "GJVIS6.BMP" => {
            if s.hero() {
                hero("johnny-help")
            } else {
                Role::WaterEffect
            }
        }
        "GJBIPLAN.BMP" => match frame {
            0..=9 => sprite("biplane", Sky, Group::Vehicle, 5),
            10..=15 => sprite("sailor-walk", Ground, Group::Crew, 3),
            16..=19 | 24..=27 => sprite("rowboat", Water, Group::Vehicle, 5),
            _ => Role::IntegratedPart,
        },
        "THEEND1.BMP" => {
            if s.hero() {
                hero("johnny-help")
            } else if s.width() > 30 {
                sprite("helicopter", Sky, Group::Vehicle, 5)
            } else {
                Role::IntegratedPart
            }
        }
        "BOAT.BMP" => sprite("motorboat", Water, Group::Vehicle, 5),
        "TANKER.BMP" => sprite("tanker", Water, Group::Vehicle, 5),
        "SHIPS.BMP" => match frame {
            0..=2 => Role::IntegratedPart,
            3..=6 => Role::IntegratedPart, // Animated lower hull of the same galleon.
            7..=9 => sprite("galleon", Water, Group::Vehicle, 5),
            14..=16 => Role::Spark,
            17 => Role::Smoke,
            18..=20 => Role::Projectile,
            _ => Role::IntegratedPart,
        },
        "LILIPUTS.BMP" => sprite("sailor", Ground, Group::Crew, 3),
        "LILFISH.BMP" => animal("fish-tropical", Water),
        "JATA.BMP" => hero("johnny-idle"),
        "JCHANGE.BMP" => Role::IntegratedPart,
        "WOULDBE.BMP" => match frame {
            10..=15 => animal("dolphin", Water),
            0 | 16..=22 => sprite("sailor", Water, Group::Crew, 3),
            30..=39 => sprite("suzy-sit", Water, Group::Crew, 3),
            23..=29 | 40 | 42 => sprite("sailor", Water, Group::Crew, 3),
            1..=9 | 41 => Role::IntegratedPart,
            43..=44 | 46 | 49..=50 => Role::WaterEffect,
            _ => sprite("mermaid-swim", Water, Group::Mermaid, 10),
        },
        "JOHNWOUL.BMP" => match frame {
            0 | 2 => hero("johnny-idle"),
            1 | 6 => hero("johnny-land"),
            3 => hero("johnny-dive-jump"),
            4 => hero("johnny-fall"),
            5 => hero("johnny-help"),
            7 => hero("johnny-run"),
            8 => sprite("johnny-dive-down", Water, Group::Hero, 10),
            13..=14 => sprite("johnny-bath", Water, Group::Hero, 10),
            _ => Role::IntegratedPart,
        },
        "MJ_AMB.BMP" => {
            if s.height() > 30 {
                hero("johnny-idle")
            } else {
                Role::IntegratedPart
            }
        }
        "SHKNFIST.BMP" => hero("johnny-fist"),
        "LITEBULB.BMP" => {
            if s.hero() {
                hero("johnny-help")
            } else if frame < 2 {
                Role::Idea
            } else {
                Role::Question
            }
        }
        "THNKBUBL.BMP" => prop(
            if frame == 5 {
                "thought-clock"
            } else {
                "thought-island"
            },
            Contact,
        ),
        "ZZZZS.BMP" => Role::SleepText,
        "MEANWHIL.BMP" => {
            if frame == 0 {
                prop("clock", Contact)
            } else {
                Role::IntegratedPart
            }
        }
        "HOLIDAY.BMP" => prop(
            if frame < 2 {
                "pumpkin"
            } else if frame == 2 {
                "christmas-tree"
            } else {
                "lantern"
            },
            Ground,
        ),
        "SPLASH.BMP" => Role::WaterEffect,
        "SSUZY1.BMP" => {
            if frame == 11 {
                prop("bottle", Contact)
            } else {
                sprite(
                    if frame < 4 { "suzy-read" } else { "suzy-sit" },
                    Ground,
                    Group::Suzy,
                    10,
                )
            }
        }
        "SSUZY2.BMP" => {
            if frame == 1 {
                prop("bottle", Contact)
            } else {
                sprite(
                    if frame < 4 {
                        "suzy-read"
                    } else if frame < 6 {
                        "suzy-sit"
                    } else {
                        "suzy-walk"
                    },
                    Ground,
                    Group::Suzy,
                    10,
                )
            }
        }
        "SSUZY3.BMP" => match frame {
            0..=4 | 24..=27 => sprite("suzy-walk", Ground, Group::Suzy, 10),
            5 => prop("bottle", Water),
            6..=11 | 14..=15 => sprite("suzy-wave", Ground, Group::Suzy, 10),
            12..=13 => sprite("suzy-sit", Ground, Group::Suzy, 10),
            16..=17 => Role::Smoke,
            18 | 29 => prop("thought-island", Contact),
            19..=22 => sprite("johnny-bath", Water, Group::Hero, 10),
            28 => prop("fish-silver", Contact),
            30 => sprite("suzy-read", Ground, Group::Suzy, 10),
            _ => Role::IntegratedPart,
        },
        "SJMSUZY1.BMP" => hero(if frame < 5 {
            "johnny-flower"
        } else {
            "johnny-scared"
        }),
        "SJMSUZY2.BMP" => {
            if frame < 4 {
                sprite("suzy-read", Ground, Group::Suzy, 10)
            } else {
                pair("suzy-embrace")
            }
        }
        "SJMSUZY3.BMP" => pair(if frame < 6 {
            "suzy-accept"
        } else {
            "suzy-kiss"
        }),
        "SJBRAKUP.BMP" => hero("johnny-sad"),
        "SJGFTASK.BMP" => {
            if s.hero() {
                hero("johnny-flower")
            } else {
                Role::IntegratedPart
            }
        }
        "SJGFTJMP.BMP" => hero("johnny-help"),
        "SJGFTSHY.BMP" => hero("johnny-flower"),
        "SJGFTXCH.BMP" => hero("johnny-reach"),
        "SBREAKUP.BMP" => match frame {
            0..=10 | 13 => sprite("mermaid", Water, Group::Mermaid, 10),
            11..=12 | 14..=20 => sprite("mermaid-swim", Water, Group::Mermaid, 10),
            21..=28 => hero("johnny-sad"),
            29..=32 => hero("johnny-land"),
            33 => hero("johnny-sad"),
            _ => Role::PaintedScenery,
        },
        "SLEVEJC1.BMP" => match frame {
            2..=6 => hero("johnny-build"),
            7 | 10 => hero("johnny-walk"),
            8..=9 => prop("raft", Water),
            11 => hero("johnny-idle"),
            _ => Role::IntegratedPart,
        },
        "SLEVEJC2.BMP" => match frame {
            0..=2 => hero("johnny-sad"),
            3..=7 => hero("johnny-run"),
            8..=9 => hero("johnny-reach"),
            _ => Role::IntegratedPart,
        },
        "SLEVEJC3.BMP" => match frame {
            0..=1 => hero("johnny-run"),
            2 => hero("johnny-land"),
            3..=7 => sprite("johnny-skydive", Water, Group::Hero, 10),
            _ => sprite("johnny-row", Water, Group::Hero, 10),
        },
        "SLEVEJM1.BMP" => {
            if s.green > 0.08 {
                sprite("mermaid", Water, Group::Mermaid, 10)
            } else {
                Role::WaterEffect
            }
        }
        "SLEVEJM2.BMP" | "SLEVEJM3.BMP" => {
            if s.green > 0.08 {
                sprite("mermaid", Water, Group::Mermaid, 10)
            } else if s.gray > 0.15 {
                animal("dolphin", Water)
            } else {
                Role::WaterEffect
            }
        }
        "SMDATE1.BMP" | "SMGIFT.BMP" | "SMGLIMSE.BMP" => {
            if s.green > 0.04 && s.height() > 20 {
                sprite("mermaid", Water, Group::Mermaid, 10)
            } else {
                Role::WaterEffect
            }
        }
        "SMGFTWAV.BMP" | "SMDATE10.BMP" => Role::WaterEffect,
        "SMDATE2.BMP" => hero("johnny-help"),
        "SMDATE3.BMP" => sprite(
            if frame < 13 {
                "johnny-tux-walk"
            } else {
                "johnny-tux"
            },
            Ground,
            Group::Hero,
            15,
        ),
        "SMDATE4.BMP" => sprite("johnny-tux-walk", Ground, Group::Hero, 15),
        "SMDATE5.BMP" => pair(if frame < 6 {
            "mermaid-dance"
        } else {
            "mermaid-dance-turn"
        }),
        "SMDATE6.BMP" => {
            if s.green > 0.04 {
                sprite("mermaid", Ground, Group::Mermaid, 10)
            } else if s.hero() {
                sprite("johnny-tux", Ground, Group::Hero, 15)
            } else {
                Role::IntegratedPart
            }
        }
        "SMDATE7.BMP" => {
            if frame == 0 {
                prop("dinner-table", Ground)
            } else {
                prop("gramophone", Ground)
            }
        }
        "SMDATE8.BMP" => match frame {
            0..=2 => sprite("johnny-tux-sit", Ground, Group::Hero, 15),
            3..=4 => sprite("mermaid-sit", Ground, Group::Mermaid, 10),
            _ => Role::IntegratedPart,
        },
        "SMDATE9.BMP" => pair("mermaid-embrace"),
        "SMDATE11.BMP" => match frame {
            0..=8 => Role::IntegratedPart,
            9..=12 => sprite("johnny-tux-walk", Ground, Group::Hero, 15),
            14..=18 => sprite("johnny-tux-dance", Ground, Group::Hero, 15),
            _ => sprite("johnny-tux", Ground, Group::Hero, 15),
        },
        "SMDATE12.BMP" => {
            if frame < 2 {
                sprite("mermaid-embrace", Water, Group::Pair, 20)
            } else {
                Role::WaterEffect
            }
        }
        _ => Role::Unknown,
    }
}
/// Poses for concrete gestures follow the authored source beat. Cycling every
/// replacement on wall time loses casts, throws and cause/effect ordering.
pub fn authored_phase(name: &str, frame: usize) -> Option<usize> {
    Some(match name {
        "MJFISH1.BMP" => match frame {
            0..=3 => 0,
            4 => 1,
            5..=6 => 2,
            7 => 3,
            8..=9 => 4,
            10..=13 => 5,
            14 => 6,
            15..=21 => 7,
            27 => 6,
            _ => return None,
        },
        "FISHMAN.BMP" => 6,
        "MJFISH2.BMP" => match frame {
            0 | 4 => 0,
            1 => 1,
            2 => 2,
            5 => 3,
            6..=8 => 4,
            9 => 5,
            10 => 6,
            11..=12 => 7,
            3 | 13..=14 => 5,
            _ => return None,
        },
        "GJCATCH1.BMP" => {
            if frame > 12 {
                7
            } else {
                5
            }
        }
        "FIRE.BMP" => match frame {
            0..=3 | 13..=14 => 0,
            4 => 1,
            5 => 3,
            6..=8 => 6,
            9 => 4,
            15..=18 | 22 | 25..=26 => 7,
            31 => 2,
            32 => 3,
            _ => return None,
        },
        "FIRE3.BMP" => [4, 5, 4][frame.min(2)],
        "FIRE4.BMP" => match frame {
            0 => 0,
            1 | 15 => 1,
            8 | 17 | 22 | 24 => 2,
            2..=4 | 7 | 10..=14 | 16 | 18..=20 | 34 => 3,
            26 | 28 => 5,
            27 => 4,
            29 | 32 => 6,
            30..=31 => 7,
            33 => 8,
            _ => return None,
        },
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resources::AssetStore;
    #[test]
    fn flat_bodies_and_boat_crew_keep_their_roles_without_color_heuristics() {
        let shape = Shape {
            opaque: 100,
            bbox: [0, 0, 70, 18],
            ..Shape::default()
        };
        for (sheet, frame) in [("GJHOT.BMP", 19), ("STNDLAY.BMP", 5)] {
            let Role::Sprite(binding) = bind(sheet, frame, shape) else {
                panic!("Missing lying pose")
            };
            assert_eq!(binding.clip, "johnny-sleep");
        }
        let Role::Sprite(binding) = bind("GJNAT3.BMP", 2, shape) else {
            panic!("Missing boat crew")
        };
        assert_eq!(binding.group, Group::Crew);
        assert_eq!(binding.support, Support::Water);
        assert!(matches!(bind("GJHOT.BMP", 12, shape), Role::IntegratedPart));
    }
    #[test]
    fn every_source_family_has_an_explicit_new_visual_or_scene_role() {
        let assets =
            AssetStore::load(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("original"))
                .unwrap();
        let art = crate::hd2d_art::Art::load(
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/hd2d-v2"),
        )
        .unwrap();
        for (name, frames) in &assets.sprites {
            for (index, image) in frames.iter().enumerate() {
                let shape = Shape::analyze(image, &assets.palette);
                if let Role::Sprite(binding) = bind(name, index, shape) {
                    assert!(
                        art.clips.contains_key(binding.clip),
                        "Missing replacement for {name}:{index}: {}",
                        binding.clip
                    );
                }
                assert!(
                    !matches!(bind(name, index, shape), Role::Unknown),
                    "Unmapped source {name}:{index}"
                );
            }
        }
    }
}
