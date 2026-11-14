//! Shared geometry interpolation for compositor window transitions.

use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationCurve {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl AnimationCurve {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "linear" => Some(Self::Linear),
            "ease-in" => Some(Self::EaseIn),
            "ease-out" => Some(Self::EaseOut),
            "ease-in-out" => Some(Self::EaseInOut),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Linear => "linear",
            Self::EaseIn => "ease-in",
            Self::EaseOut => "ease-out",
            Self::EaseInOut => "ease-in-out",
        }
    }

    fn apply(self, progress: f64) -> f64 {
        let progress = progress.clamp(0.0, 1.0);
        match self {
            Self::Linear => progress,
            Self::EaseIn => progress * progress,
            Self::EaseOut => 1.0 - (1.0 - progress) * (1.0 - progress),
            Self::EaseInOut if progress < 0.5 => 2.0 * progress * progress,
            Self::EaseInOut => 1.0 - (-2.0 * progress + 2.0).powi(2) / 2.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl AnimationRect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self { x, y, width, height }
    }

    pub fn rounded_location(self) -> (i32, i32) {
        (self.x.round() as i32, self.y.round() as i32)
    }

    pub fn approximately_eq(self, other: Self) -> bool {
        const EPSILON: f64 = 0.01;
        (self.x - other.x).abs() < EPSILON
            && (self.y - other.y).abs() < EPSILON
            && (self.width - other.width).abs() < EPSILON
            && (self.height - other.height).abs() < EPSILON
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GeometryAnimation {
    pub from: AnimationRect,
    pub to: AnimationRect,
    pub started_at: Instant,
    pub duration: Duration,
    pub curve: AnimationCurve,
}

impl GeometryAnimation {
    pub fn new(
        from: AnimationRect,
        to: AnimationRect,
        started_at: Instant,
        duration: Duration,
        curve: AnimationCurve,
    ) -> Self {
        Self { from, to, started_at, duration, curve }
    }

    pub fn progress(self, now: Instant) -> f64 {
        if self.duration.is_zero() {
            return 1.0;
        }

        (now.saturating_duration_since(self.started_at).as_secs_f64()
            / self.duration.as_secs_f64())
        .clamp(0.0, 1.0)
    }

    pub fn is_finished(self, now: Instant) -> bool {
        self.progress(now) >= 1.0
    }

    pub fn sample(self, now: Instant) -> AnimationRect {
        let amount = self.curve.apply(self.progress(now));
        let interpolate = |start: f64, end: f64| start + (end - start) * amount;

        AnimationRect {
            x: interpolate(self.from.x, self.to.x),
            y: interpolate(self.from.y, self.to.y),
            width: interpolate(self.from.width, self.to.width),
            height: interpolate(self.from.height, self.to.height),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64, width: f64, height: f64) -> AnimationRect {
        AnimationRect::new(x, y, width, height)
    }

    #[test]
    fn linear_animation_starts_at_origin_and_finishes_at_target() {
        let start = Instant::now();
        let animation = GeometryAnimation::new(
            rect(0.0, 10.0, 100.0, 80.0),
            rect(100.0, 50.0, 200.0, 120.0),
            start,
            Duration::from_millis(200),
            AnimationCurve::Linear,
        );

        assert_eq!(animation.sample(start), animation.from);
        assert_eq!(animation.sample(start + Duration::from_millis(200)), animation.to);
        assert!(animation.is_finished(start + Duration::from_millis(200)));
    }

    #[test]
    fn ease_out_moves_more_at_the_start_than_linear() {
        let start = Instant::now();
        let animation = GeometryAnimation::new(
            rect(0.0, 0.0, 100.0, 100.0),
            rect(100.0, 0.0, 100.0, 100.0),
            start,
            Duration::from_millis(100),
            AnimationCurve::EaseOut,
        );

        assert!(animation.sample(start + Duration::from_millis(25)).x > 25.0);
    }

    #[test]
    fn zero_duration_finishes_immediately() {
        let start = Instant::now();
        let animation = GeometryAnimation::new(
            rect(0.0, 0.0, 10.0, 10.0),
            rect(20.0, 30.0, 40.0, 50.0),
            start,
            Duration::ZERO,
            AnimationCurve::Linear,
        );

        assert!(animation.is_finished(start));
        assert_eq!(animation.sample(start), animation.to);
    }

    #[test]
    fn easing_curve_names_round_trip() {
        for curve in [
            AnimationCurve::Linear,
            AnimationCurve::EaseIn,
            AnimationCurve::EaseOut,
            AnimationCurve::EaseInOut,
        ] {
            assert_eq!(AnimationCurve::parse(curve.as_str()), Some(curve));
        }
        assert_eq!(AnimationCurve::parse("bounce"), None);
    }

    #[test]
    fn geometry_comparison_uses_a_small_tolerance() {
        assert!(rect(1.0, 2.0, 3.0, 4.0).approximately_eq(rect(1.005, 2.0, 3.0, 4.0)));
        assert!(!rect(1.0, 2.0, 3.0, 4.0).approximately_eq(rect(1.1, 2.0, 3.0, 4.0)));
    }
}
