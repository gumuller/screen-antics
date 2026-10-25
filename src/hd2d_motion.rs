//! Continuous actor roots and action-local animation clocks. Native TTM poses
//! arrive at 5–10 Hz; their opaque bounding boxes are not movement directions.
use crate::hd2d_layout::Point;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Copy, Debug)]
pub struct Pose {
    pub point: Point,
    pub phase: usize,
    pub flip: bool,
}
#[derive(Clone, Debug)]
struct Track {
    clip: String,
    family: String,
    started: f64,
    sampled: f64,
    output: Point,
    from: Point,
    target: Point,
    changed: f64,
    duration: f64,
    distance: f32,
    facing_anchor: f32,
    flip: bool,
}
#[derive(Clone, Debug)]
#[allow(dead_code)] // Consumed by the standalone continuous-motion recorder.
pub struct Sample {
    pub key: String,
    pub clip: String,
    pub point: Point,
    pub phase: usize,
    pub flip: bool,
}
#[derive(Default)]
pub struct Motion {
    tracks: BTreeMap<String, Track>,
    assigned: BTreeSet<String>,
    scales: BTreeMap<String, (f32, f64)>,
}
fn stationary_family(clip: &str) -> Option<&str> {
    if clip.starts_with("johnny-coconut-") {
        return Some("coconut");
    }
    if clip.starts_with("johnny-bird-") {
        return Some("bird-sitting");
    }
    if clip.starts_with("johnny-stretch")
        || ["johnny-rest", "johnny-rise", "johnny-bend"].contains(&clip)
    {
        return Some("warmup");
    }
    if clip.starts_with("johnny-fish") {
        return Some("fishing");
    }
    if clip == "johnny-firecraft" {
        return Some("firecraft");
    }
    if clip == "johnny-hot-meal" || clip == "johnny-roast" {
        return Some("meal");
    }
    match clip {
        "johnny-idle" | "johnny-read" | "johnny-office" | "johnny-drink" | "johnny-dig"
        | "johnny-build" | "johnny-sleep" | "johnny-recover" | "johnny-ignite" | "johnny-tend"
        | "suzy-read" | "suzy-sit" | "johnny-tux-sit" | "mermaid-sit" => Some(clip),
        "johnny-bath" | "johnny-telescope" | "johnny-angry" | "johnny-fist" | "johnny-thinking"
        | "johnny-hot" | "johnny-tired" | "johnny-look" | "johnny-flower" | "johnny-wave"
        | "johnny-book-stand" | "johnny-help" | "johnny-scared" | "johnny-reach" => Some(clip),
        _ => None,
    }
}
fn lerp(a: Point, b: Point, t: f32) -> Point {
    Point::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}
impl Motion {
    pub fn clear(&mut self) {
        self.tracks.clear();
        self.assigned.clear();
        self.scales.clear();
    }
    pub fn scale(&mut self, key: &str, target: f32, time: f64) -> f32 {
        let entry = self.scales.entry(key.into()).or_insert((target, time));
        let dt = time - entry.1;
        if !(0.0..=0.75).contains(&dt) {
            *entry = (target, time);
        } else {
            entry.0 += (target - entry.0) * (1. - (-dt / 0.12).exp()) as f32;
            entry.1 = time;
        }
        entry.0
    }
    pub fn begin_frame(&mut self) {
        self.assigned.clear();
    }
    pub fn entity_key(&mut self, family: &str, target: Point, time: f64) -> String {
        let prefix = format!("{family}#");
        let mut best = None;
        for (key, track) in &self.tracks {
            if !key.starts_with(&prefix)
                || self.assigned.contains(key)
                || time - track.sampled > 0.5
            {
                continue;
            }
            let distance = (track.target.x - target.x).hypot(track.target.y - target.y);
            if distance < 160. && best.as_ref().is_none_or(|(_, d)| distance < *d) {
                best = Some((key.clone(), distance));
            }
        }
        let key = best.map(|(key, _)| key).unwrap_or_else(|| {
            (0..)
                .map(|i| format!("{prefix}{i}"))
                .find(|k| !self.tracks.contains_key(k))
                .unwrap()
        });
        self.assigned.insert(key.clone());
        key
    }
    pub fn sample(
        &mut self,
        key: &str,
        clip: &str,
        target: Point,
        time: f64,
        frames: usize,
        fps: f32,
        flip: bool,
    ) -> Pose {
        let family = stationary_family(clip).unwrap_or(clip);
        let fresh = || Track {
            clip: clip.into(),
            family: family.into(),
            started: time,
            sampled: time,
            output: target,
            from: target,
            target,
            changed: time,
            duration: 0.12,
            distance: 0.,
            facing_anchor: target.x,
            flip,
        };
        let track = self.tracks.entry(key.into()).or_insert_with(fresh);
        if time < track.sampled || time - track.sampled > 0.75 {
            *track = fresh();
        }
        // Finish the preceding segment before accepting a new authored key.
        let t = ((time - track.changed) / track.duration).clamp(0., 1.) as f32;
        let current = lerp(track.from, track.target, t);
        let clip_changed = track.clip != clip;
        let stationary = stationary_family(clip).is_some();
        let target = if stationary && track.family == family {
            track.target
        } else {
            target
        };
        let moved =
            (target.x - track.target.x).abs() > 0.05 || (target.y - track.target.y).abs() > 0.05;
        if moved {
            let interval = time - track.changed;
            let speed = match clip {
                "johnny-climb" => 110.,
                "johnny-dive" | "johnny-dive-down" => 480.,
                "johnny-run" => 270.,
                _ if key == "Hero" || key == "Suzy" || key == "Mermaid" || key == "Pair" => 180.,
                _ => 900.,
            };
            track.duration = interval
                .clamp(0.12, 0.22)
                .max((target.x - current.x).hypot(target.y - current.y) as f64 / speed);
            track.from = current;
            track.target = target;
            track.changed = time;
        }
        if clip_changed {
            track.clip = clip.into();
            track.family = family.into();
            track.started = time;
            track.distance = 0.;
            track.flip = flip;
            track.facing_anchor = current.x;
        }
        let point = if time == track.sampled {
            track.output
        } else {
            current
        };
        track.distance +=
            ((point.x - track.output.x).powi(2) + (point.y - track.output.y).powi(2)).sqrt();
        let locomotion = clip.ends_with("-walk") || clip == "johnny-run";
        if locomotion {
            // A turn needs actual travel, not a differently shaped arm or foot.
            let dx = point.x - track.facing_anchor;
            if dx.abs() > 8. {
                track.flip = dx < 0.;
                track.facing_anchor = point.x;
            }
        } else if !stationary {
            track.flip = flip;
        }
        let age = (time - track.started).max(0.);
        let phase = if clip == "johnny-read" {
            let t = age % 9.;
            if t < 3. {
                0
            } else if t < 4. {
                1
            } else if t < 4.5 {
                2
            } else if t < 5.5 {
                3
            } else if t < 7.5 {
                0
            } else {
                4
            }
        } else if locomotion {
            let stride = if clip == "johnny-run" { 42. } else { 28. };
            (track.distance / stride * frames as f32) as usize
        } else {
            ((time - track.started).max(0.) * fps as f64) as usize
        };
        track.output = point;
        track.sampled = time;
        Pose {
            point,
            phase: phase % frames.max(1),
            flip: track.flip,
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn a_new_source_key_cannot_accelerate_a_long_action_transition() {
        let mut m = Motion::default();
        m.sample(
            "Hero",
            "johnny-walk",
            Point::new(0., 260.),
            0.,
            8,
            8.,
            false,
        );
        let mut previous = 0.;
        for i in 1..120 {
            let target = Point::new(400. + (i / 8) as f32, 260.);
            let p = m.sample("Hero", "johnny-climb", target, i as f64 / 60., 6, 6., false);
            assert!((p.point.x - previous).abs() <= 110. / 60. + 0.001);
            previous = p.point.x;
        }
    }
    #[test]
    fn several_birds_have_independent_tracks_and_pause_reuses_them() {
        let mut m = Motion::default();
        for t in [0., 0.1, 0.1, 0.2] {
            m.begin_frame();
            let a = m.entity_key("gull", Point::new(100., 50.), t);
            m.sample(&a, "gull", Point::new(100., 50.), t, 6, 10., false);
            let b = m.entity_key("gull", Point::new(150., 70.), t);
            m.sample(&b, "gull", Point::new(150., 70.), t, 6, 10., true);
            assert_ne!(a, b);
            assert_eq!(m.tracks.len(), 2);
        }
    }
    #[test]
    fn held_reader_does_not_turn_or_drift_with_changing_native_bounds() {
        let mut m = Motion::default();
        for i in 0..300 {
            let p = m.sample(
                "Hero",
                "johnny-read",
                Point::new(400. + (i % 5) as f32, 260. + (i % 3) as f32),
                i as f64 / 60.,
                6,
                4.,
                false,
            );
            assert!(!p.flip);
            assert_eq!(p.point.x, 400.);
            assert_eq!(p.point.y, 260.);
        }
    }
    #[test]
    fn sparse_walk_keys_produce_continuous_roots_and_one_real_turn() {
        let mut m = Motion::default();
        let mut previous = 0.;
        let mut max_step = 0f32;
        let mut flips = 0;
        let mut facing = false;
        for i in 0..240 {
            let x = if i < 120 {
                (i / 12) * 12
            } else {
                120 - ((i - 120) / 12) * 12
            } as f32;
            let p = m.sample(
                "Hero",
                "johnny-walk",
                Point::new(x, 260.),
                i as f64 / 60.,
                6,
                8.,
                false,
            );
            max_step = max_step.max((p.point.x - previous).abs());
            if p.flip != facing {
                flips += 1;
                facing = p.flip;
            }
            previous = p.point.x;
        }
        assert!(max_step < 1.1, "root step {max_step}");
        assert_eq!(flips, 1);
    }
    #[test]
    fn action_clock_starts_at_entry_and_pause_is_exact() {
        let mut m = Motion::default();
        let a = m.sample(
            "Hero",
            "johnny-read",
            Point::new(400., 260.),
            90.,
            6,
            4.,
            false,
        );
        assert_eq!(a.phase, 0);
        let b = m.sample(
            "Hero",
            "johnny-read",
            Point::new(403., 259.),
            90.,
            6,
            4.,
            false,
        );
        assert_eq!(a.point.x, b.point.x);
        assert_eq!(a.phase, b.phase);
    }
}
