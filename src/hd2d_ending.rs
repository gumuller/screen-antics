//! Ending choreography in the new parchment illustration's own coordinates.
//! The recovered source supplies its start and duration, not obsolete geometry.
use crate::hd2d_layout::Point;
#[derive(Clone, Debug)]
pub struct Layer {
    pub clip: &'static str,
    pub phase: usize,
    pub point: Point,
    pub scale: [f32; 2],
    pub flip: bool,
    pub opacity: f32,
}
pub struct EndingFrame {
    pub clean_opacity: f32,
    pub layers: Vec<Layer>,
}
fn ease(t: f32) -> f32 {
    let t = t.clamp(0., 1.);
    t * t * (3. - 2. * t)
}
fn mix(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t.clamp(0., 1.)
}
pub fn frame(t: f32) -> EndingFrame {
    let fade = 1. - ease((t - 12.) / 0.9);
    let mut result = EndingFrame {
        clean_opacity: fade,
        layers: vec![],
    };
    let mut add = |clip, phase, x, y, sx, sy, flip, opacity| {
        result.layers.push(Layer {
            clip,
            phase,
            point: Point::new(x, y),
            scale: [sx, sy],
            flip,
            opacity,
        })
    };
    if t < 6.6 {
        add(
            "biplane",
            0,
            550. - 72. * t,
            142. + (t * 1.7).sin(),
            0.48,
            0.48,
            true,
            1. - ease((t - 6.) / 0.6),
        );
    }
    if t >= 2. && t < 3. {
        let u = t - 2.;
        add(
            "johnny-skydive",
            0,
            mix(406., 357., u),
            mix(147., 178., u * u),
            0.30,
            0.30,
            true,
            1.,
        );
    } else if (3.0..7.2).contains(&t) {
        let u = (t - 3.) / 4.2;
        let x = mix(357., 333., u) + (u * std::f32::consts::TAU).sin() * 3.;
        let feet = mix(185., 251., u);
        let open = ease((t - 3.) / 0.35);
        // Approach the foreground island in perspective. The seated actor at
        // the end must have the same stature as the painted final silhouette.
        let size = mix(0.30, 0.95, ease(u));
        let canopy = size * 2.4;
        add(
            "parachute",
            0,
            x,
            feet - size * 63.,
            canopy,
            canopy * (0.21 + 0.79 * open),
            false,
            1.,
        );
        add("johnny-parachute", 0, x, feet, size, size, false, 1.);
    } else if (7.2..8.2).contains(&t) {
        let u = ease(t - 7.2);
        add(
            "parachute",
            0,
            mix(333., 307., u),
            mix(191.15, 253., u),
            2.28,
            2.28 * (1. - u) + 0.10 * u,
            false,
            1.,
        );
        add("johnny-land", 0, 333., 251., 0.95, 0.95, false, 1.);
    } else if (8.2..10.6).contains(&t) {
        add(
            "parachute",
            0,
            307.,
            253.,
            2.28,
            0.10,
            false,
            1. - ease((t - 9.7) / 0.9),
        );
        add("johnny-wave", 0, 333., 251., 0.95, 0.95, false, 1.);
    } else if (10.6..11.8).contains(&t) {
        let u = (t - 10.6) / 1.2;
        add(
            "johnny-walk",
            (u * 12.) as usize,
            mix(333., 364., u),
            mix(251., 244., u),
            0.95,
            0.95,
            false,
            1.,
        );
    } else if (11.8..12.9).contains(&t) {
        add("johnny-read", 0, 364., 244., 1., 1., false, fade);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ending_has_one_actor_and_lands_before_title() {
        for tick in 0..904 {
            let f = frame(tick as f32 / 60.);
            assert!(
                f.layers
                    .iter()
                    .filter(|l| l.clip.starts_with("johnny-"))
                    .count()
                    <= 1
            );
            assert!(f.layers.iter().all(|l| l.clip != "helicopter"));
            assert!(f
                .layers
                .iter()
                .all(|l| l.point.x.is_finite() && l.point.y.is_finite()));
            if tick < 720 {
                assert_eq!(f.clean_opacity, 1.);
            }
        }
        assert!(frame(7.3)
            .layers
            .iter()
            .any(|l| l.clip == "johnny-land" && l.point.y == 251.));
        assert!(frame(13.).layers.is_empty());
        assert_eq!(frame(13.).clean_opacity, 0.);
    }
    #[test]
    fn parachute_descent_is_continuous() {
        let mut previous = None;
        for tick in 180..432 {
            let f = frame(tick as f32 / 60.);
            let p = f
                .layers
                .iter()
                .find(|l| l.clip == "johnny-parachute")
                .unwrap()
                .point;
            if let Some(old) = previous {
                let old: Point = old;
                assert!((p.x - old.x).abs() < 1.);
                assert!((p.y - old.y).abs() < 1.);
            }
            previous = Some(p);
        }
    }
}
