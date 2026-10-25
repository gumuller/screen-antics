//! Time-based scene dissolves and the original TTM directional wipes.
use crate::animation::{Transition, TransitionKind};
use std::time::{Duration, Instant};
pub struct Effect {
    pub start: Instant,
    duration: Duration,
    event: Option<Transition>,
    old: Vec<u32>,
    mixed: Vec<u32>,
}
impl Effect {
    pub fn new(old: &[u32], event: Option<Transition>) -> Self {
        let duration = Duration::from_millis(event.as_ref().map_or(350, |e| e.duration_ms) as u64);
        Self {
            start: Instant::now(),
            duration,
            event,
            old: old.to_vec(),
            mixed: vec![0; old.len()],
        }
    }
    pub fn duration(&self) -> Duration {
        self.duration
    }
    pub fn done(&self) -> bool {
        self.start.elapsed() >= self.duration
    }
    pub fn frame<'a>(
        &'a mut self,
        target: &'a [u32],
        width: usize,
        height: usize,
        stage: [f64; 3],
    ) -> &'a [u32] {
        if self.old.len() != target.len() {
            return target;
        }
        let p =
            (self.start.elapsed().as_secs_f64() / self.duration.as_secs_f64().max(0.001)).min(1.);
        let [scale, ox, oy] = stage;
        for y in 0..height {
            for x in 0..width {
                let at = y * width + x;
                self.mixed[at] = if let Some(event) = &self.event {
                    let [rx, ry, rw, rh] = event.rect;
                    let lx = (x as f64 - ox) / scale;
                    let ly = (y as f64 - oy) / scale;
                    if lx < rx as f64
                        || ly < ry as f64
                        || lx >= (rx + rw) as f64
                        || ly >= (ry + rh) as f64
                    {
                        target[at]
                    } else {
                        let u = (lx - rx as f64) / rw.max(1) as f64;
                        let v = (ly - ry as f64) / rh.max(1) as f64;
                        match event.kind {
                            TransitionKind::FadeIn => mix(0xff000000, target[at], p),
                            TransitionKind::FadeOut => mix(self.old[at], 0xff000000, p),
                            kind => {
                                if revealed(kind, u, v, x, y, p) {
                                    target[at]
                                } else {
                                    self.old[at]
                                }
                            }
                        }
                    }
                } else {
                    mix(self.old[at], target[at], p)
                };
            }
        }
        &self.mixed
    }
}
fn mix(a: u32, b: u32, p: f64) -> u32 {
    let t = (p * 255.) as u32;
    let mut out = 0xff000000;
    for shift in [0, 8, 16] {
        out |= ((((a >> shift) & 255) * (255 - t) + ((b >> shift) & 255) * t) / 255) << shift;
    }
    out
}
fn revealed(kind: TransitionKind, u: f64, v: f64, x: usize, y: usize, p: f64) -> bool {
    if p >= 1. {
        return true;
    }
    if p <= 0. {
        return false;
    }
    match kind {
        TransitionKind::Immediate => true,
        TransitionKind::LeftToRight => u < p,
        TransitionKind::RightToLeft => 1. - u < p,
        TransitionKind::TopToBottom => v < p,
        TransitionKind::BottomToTop => 1. - v < p,
        TransitionKind::InsideOut => (u - 0.5).abs().max((v - 0.5).abs()) * 2. < p,
        TransitionKind::OutsideIn => 1. - (u - 0.5).abs().max((v - 0.5).abs()) * 2. < p,
        TransitionKind::Interlaced => (y % 10) as f64 / 10. < p,
        TransitionKind::Dissolve => {
            let hash = (x as u32).wrapping_mul(73856093) ^ (y as u32).wrapping_mul(19349663);
            (hash % 65536) as f64 / 65536. < p
        }
        _ => true,
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn directional_wipes_reveal_correct_half() {
        assert!(revealed(TransitionKind::LeftToRight, 0.25, 0., 0, 0, 0.5));
        assert!(!revealed(TransitionKind::LeftToRight, 0.75, 0., 0, 0, 0.5));
        assert!(revealed(TransitionKind::RightToLeft, 0.75, 0., 0, 0, 0.5));
        assert!(!revealed(TransitionKind::RightToLeft, 0.25, 0., 0, 0, 0.5));
    }
    #[test]
    fn fade_endpoints_are_exact() {
        assert_eq!(mix(0xff123456, 0xffabcdef, 0.), 0xff123456);
        assert_eq!(mix(0xff123456, 0xffabcdef, 1.), 0xffabcdef);
    }
    #[test]
    fn wipe_region_uses_registered_panorama_stage() {
        let old = vec![0xffff0000; 100 * 50];
        let target = vec![0xff00ff00; 100 * 50];
        let mut effect = Effect::new(
            &old,
            Some(Transition {
                kind: TransitionKind::LeftToRight,
                rect: [0, 0, 2, 1],
                duration_ms: 1000,
            }),
        );
        effect.start = Instant::now() - Duration::from_millis(500);
        let frame = effect.frame(&target, 100, 50, [10., 30., 20.]);
        assert_eq!(frame[25 * 100 + 31], 0xff00ff00);
        assert_eq!(frame[25 * 100 + 46], 0xffff0000);
        assert_eq!(frame[25 * 100 + 20], 0xff00ff00);
    }
}
