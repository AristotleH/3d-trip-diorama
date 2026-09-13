use std::collections::BTreeMap;

/// Tracks captured pointers; adding/removing a finger resets the gesture baseline.
#[derive(Default)]
pub struct Gesture {
    pointers: BTreeMap<i32, (f64, f64)>,
}

#[derive(Debug, PartialEq)]
pub enum Motion {
    Orbit(f32, f32),
    Zoom(f32),
}

impl Gesture {
    pub fn start(&mut self, id: i32, x: f64, y: f64) {
        self.pointers.insert(id, (x, y));
    }

    pub fn end(&mut self, id: i32) {
        self.pointers.remove(&id);
    }

    fn span(&self) -> f64 {
        let mut points = self.pointers.values();
        let a = points.next().unwrap();
        let b = points.next().unwrap();
        (a.0 - b.0).hypot(a.1 - b.1)
    }

    pub fn update(&mut self, id: i32, x: f64, y: f64) -> Option<Motion> {
        let old = *self.pointers.get(&id)?;
        let count = self.pointers.len();
        let old_span = if count == 2 { self.span() } else { 0.0 };
        self.pointers.insert(id, (x, y));
        match count {
            1 => Some(Motion::Orbit((x - old.0) as f32, (y - old.1) as f32)),
            2 => {
                let span = self.span();
                if old_span > 1.0 && span > 1.0 {
                    Some(Motion::Zoom((old_span / span) as f32))
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drag_pinch_and_return_to_drag_without_jump() {
        let mut g = Gesture::default();
        g.start(1, 0.0, 0.0);
        assert_eq!(g.update(1, 10.0, 5.0), Some(Motion::Orbit(10.0, 5.0)));
        g.start(2, 30.0, 5.0);
        assert_eq!(g.update(2, 50.0, 5.0), Some(Motion::Zoom(0.5)));
        assert_eq!(g.update(2, 30.0, 5.0), Some(Motion::Zoom(2.0)));
        g.end(2);
        assert_eq!(g.update(1, 11.0, 6.0), Some(Motion::Orbit(1.0, 1.0)));
    }

    #[test]
    fn cancel_ignores_later_moves_and_allows_a_fresh_gesture() {
        let mut g = Gesture::default();
        g.start(1, 10.0, 10.0);
        g.end(1);
        assert_eq!(g.update(1, 100.0, 100.0), None);
        g.start(1, 100.0, 100.0);
        assert_eq!(g.update(1, 101.0, 102.0), Some(Motion::Orbit(1.0, 2.0)));
    }

    #[test]
    fn coincident_and_extra_fingers_do_not_zoom() {
        let mut g = Gesture::default();
        g.start(1, 0.0, 0.0);
        g.start(2, 0.0, 0.0);
        assert_eq!(g.update(2, 10.0, 0.0), None);
        g.start(3, 20.0, 0.0);
        assert_eq!(g.update(3, 30.0, 0.0), None);
        g.end(3);
        assert_eq!(g.update(2, 20.0, 0.0), Some(Motion::Zoom(0.5)));
    }
}
