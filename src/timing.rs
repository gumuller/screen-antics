use std::time::{Duration, Instant};
pub const PRESENT_INTERVAL: Duration = Duration::from_nanos(1_000_000_000 / 60);
pub fn script_interval(ticks: u32, original: bool) -> Duration {
    let tick = if original {
        Duration::from_millis(50)
    } else {
        PRESENT_INTERVAL
    };
    tick.saturating_mul(ticks.max(1))
}
/// Keep deadlines on the clock's timeline; render work does not extend each frame.
pub fn next_present(deadline: Instant, now: Instant) -> Instant {
    let next = deadline + PRESENT_INTERVAL;
    if next > now {
        next
    } else {
        now + PRESENT_INTERVAL
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sixty_ticks_take_one_second() {
        let t = script_interval(60, false);
        assert!(t.abs_diff(Duration::from_secs(1)) < Duration::from_micros(1));
        assert_eq!(script_interval(60, true), Duration::from_secs(3));
    }
    #[test]
    fn render_work_does_not_accumulate_drift() {
        let start = Instant::now();
        assert_eq!(
            next_present(start, start + Duration::from_millis(4)),
            start + PRESENT_INTERVAL
        );
        assert!(
            next_present(start, start + Duration::from_secs(2)) > start + Duration::from_secs(2)
        );
    }
}
