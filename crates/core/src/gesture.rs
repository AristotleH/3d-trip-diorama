use std::collections::BTreeMap;

/// Tracks captured pointers; adding/removing a finger resets the gesture baseline.
#[derive(Default)]
pub struct Gesture {
    pointers: BTreeMap<i32, (f64, f64)>,
    baseline: Option<(f64, f64, f64, f64)>,
}

#[derive(Debug, PartialEq)]
pub enum Motion {
    Orbit(f32, f32),
    TwoFinger { dx: f32, dy: f32, ratio: f32, rotation: f32 },
}

impl Gesture {
    pub fn start(&mut self, id: i32, x: f64, y: f64) {
        self.pointers.insert(id, (x, y));
        self.baseline = self.two_finger_state();
    }

    pub fn end(&mut self, id: i32) {
        self.pointers.remove(&id);
        self.baseline = self.two_finger_state();
    }

    fn two_finger_state(&self) -> Option<(f64, f64, f64, f64)> {
        if self.pointers.len() != 2 { return None; }
        let mut points = self.pointers.values();
        let a = points.next().unwrap();
        let b = points.next().unwrap();
        Some(((a.0 + b.0) * 0.5, (a.1 + b.1) * 0.5,
            (a.0 - b.0).hypot(a.1 - b.1), (b.1 - a.1).atan2(b.0 - a.0)))
    }

    pub fn update(&mut self, id: i32, x: f64, y: f64) -> Option<Motion> {
        let old = *self.pointers.get(&id)?;
        self.pointers.insert(id, (x, y));
        if self.pointers.len() == 1 {
            Some(Motion::Orbit((x - old.0) as f32, (y - old.1) as f32))
        } else {
            None
        }
    }

    /// Sample both fingers together once per frame. Pointer events arrive one
    /// finger at a time; zooming on each event makes parallel drags wobble.
    pub fn take_two_finger_motion(&mut self) -> Option<Motion> {
        let current = self.two_finger_state();
        let previous = self.baseline;
        self.baseline = current;
        let (x, y, span, angle) = current?;
        let (old_x, old_y, old_span, old_angle) = previous?;
        if current == previous { return None; }
        // atan2 wraps at ±π; use the shortest signed turn across that seam.
        let turn = angle - old_angle;
        let rotation = if old_span > 1.0 && span > 1.0 {
            turn.sin().atan2(turn.cos()) as f32
        } else { 0.0 };
        Some(Motion::TwoFinger {
            rotation,
            dx: (x - old_x) as f32,
            dy: (y - old_y) as f32,
            ratio: if old_span > 1.0 && span > 1.0 { (old_span / span) as f32 } else { 1.0 },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twist_orbits_in_both_directions_without_pan_or_zoom() {
        for sign in [-1.0, 1.0] {
            let mut g = Gesture::default();
            g.start(1, -50.0, 0.0);
            g.start(2, 50.0, 0.0);
            g.update(1, 0.0, -50.0 * sign);
            g.update(2, 0.0, 50.0 * sign);
            let Some(Motion::TwoFinger { dx, dy, ratio, rotation }) = g.take_two_finger_motion() else { panic!() };
            assert_eq!((dx, dy, ratio), (0.0, 0.0, 1.0));
            assert!((rotation - sign as f32 * std::f32::consts::FRAC_PI_2).abs() < 0.00001);
            assert_eq!(g.take_two_finger_motion(), None);
        }
    }

    #[test]
    fn twist_crosses_angle_seam_without_a_full_turn() {
        for sign in [-1.0, 1.0] {
            let mut g = Gesture::default();
            g.start(1, 0.0, 0.0);
            g.start(2, -100.0, sign);
            g.update(2, -100.0, -sign);
            let Some(Motion::TwoFinger { rotation, .. }) = g.take_two_finger_motion() else { panic!() };
            assert!((rotation - 0.02 * sign as f32).abs() < 0.00001);
        }
    }

    #[test]
    fn twist_pan_and_pinch_combine_and_new_finger_resets_angle() {
        let mut g = Gesture::default();
        g.start(1, -50.0, 0.0);
        g.start(2, 50.0, 0.0);
        g.update(1, 20.0, -70.0);
        g.update(2, 20.0, 130.0);
        let Some(Motion::TwoFinger { dx, dy, ratio, rotation }) = g.take_two_finger_motion() else { panic!() };
        assert_eq!((dx, dy, ratio), (20.0, 30.0, 0.5));
        assert!((rotation - std::f32::consts::FRAC_PI_2).abs() < 0.00001);
        g.end(2);
        g.start(3, -80.0, -70.0);
        assert_eq!(g.take_two_finger_motion(), None);
    }

    #[test]
    fn parallel_two_finger_drag_pans_without_zoom_or_orbit() {
        let mut g = Gesture::default();
        g.start(1, 0.0, 0.0);
        g.start(2, 100.0, 0.0);
        assert_eq!(g.update(1, 30.0, 20.0), None);
        assert_eq!(g.update(2, 130.0, 20.0), None);
        assert_eq!(g.take_two_finger_motion(), Some(Motion::TwoFinger { dx: 30.0, dy: 20.0, ratio: 1.0, rotation: 0.0 }));
        assert_eq!(g.take_two_finger_motion(), None);
    }

    #[test]
    fn symmetric_pinch_zooms_without_pan() {
        let mut g = Gesture::default();
        g.start(1, -50.0, 0.0);
        g.start(2, 50.0, 0.0);
        g.update(1, -100.0, 0.0);
        g.update(2, 100.0, 0.0);
        assert_eq!(g.take_two_finger_motion(), Some(Motion::TwoFinger { dx: 0.0, dy: 0.0, ratio: 0.5, rotation: 0.0 }));
    }

    #[test]
    fn drag_and_pinch_can_happen_together_then_return_to_orbit() {
        let mut g = Gesture::default();
        g.start(1, 0.0, 0.0);
        g.start(2, 100.0, 0.0);
        g.update(1, -40.0, 20.0);
        g.update(2, 160.0, 20.0);
        assert_eq!(g.take_two_finger_motion(), Some(Motion::TwoFinger { dx: 10.0, dy: 20.0, ratio: 0.5, rotation: 0.0 }));
        g.end(2);
        assert_eq!(g.take_two_finger_motion(), None);
        assert_eq!(g.update(1, -39.0, 22.0), Some(Motion::Orbit(1.0, 2.0)));
    }

    #[test]
    fn cancel_and_extra_fingers_reset_baseline() {
        let mut g = Gesture::default();
        g.start(1, 0.0, 0.0);
        g.start(2, 100.0, 0.0);
        g.start(3, 200.0, 0.0);
        g.update(2, 150.0, 0.0);
        assert_eq!(g.take_two_finger_motion(), None);
        g.end(3);
        assert_eq!(g.take_two_finger_motion(), None);
        g.end(2);
        assert_eq!(g.update(2, 200.0, 0.0), None);
        g.start(2, 200.0, 0.0);
        assert_eq!(g.take_two_finger_motion(), None);
    }

    #[test]
    fn coincident_fingers_pan_without_invalid_zoom() {
        let mut g = Gesture::default();
        g.start(1, 0.0, 0.0);
        g.start(2, 0.0, 0.0);
        g.update(2, 10.0, 0.0);
        assert_eq!(g.take_two_finger_motion(), Some(Motion::TwoFinger { dx: 5.0, dy: 0.0, ratio: 1.0, rotation: 0.0 }));
    }
}
