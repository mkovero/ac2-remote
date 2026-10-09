use eframe::egui::{Event, PointerButton, Pos2, Rect};

#[derive(Default)]
pub struct Swipe {
    start: Option<Pos2>,
    pinched: bool,
}

impl Swipe {
    pub fn pinched(&self) -> bool {
        self.pinched
    }
    pub fn update(&mut self, events: &[Event], rect: Rect, multi_touch: bool) -> i32 {
        if multi_touch {
            self.pinched = true;
        }
        let mut step = 0;
        for event in events {
            if let Event::PointerButton {
                pos,
                button: PointerButton::Primary,
                pressed,
                ..
            } = event
            {
                if *pressed {
                    self.start = rect.contains(*pos).then_some(*pos);
                    self.pinched = multi_touch;
                } else {
                    if let Some(start) = self.start.take() {
                        let delta = *pos - start;
                        if !self.pinched
                            && delta.x.abs() >= 60.0
                            && delta.x.abs() > delta.y.abs() * 1.5
                        {
                            step = if delta.x < 0.0 { 1 } else { -1 };
                        }
                    }
                }
            }
        }
        step
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::{Modifiers, pos2};

    fn button(x: f32, y: f32, pressed: bool) -> Event {
        Event::PointerButton {
            pos: pos2(x, y),
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::default(),
        }
    }
    fn rect() -> Rect {
        Rect::from_min_max(pos2(0.0, 0.0), pos2(400.0, 400.0))
    }

    #[test]
    fn touch_release_then_pointer_gone_changes_measurement_both_directions() {
        let mut swipe = Swipe::default();
        swipe.update(&[button(300.0, 100.0, true)], rect(), false);
        assert_eq!(
            swipe.update(
                &[button(100.0, 110.0, false), Event::PointerGone],
                rect(),
                false
            ),
            1
        );
        swipe.update(&[button(100.0, 100.0, true)], rect(), false);
        assert_eq!(
            swipe.update(
                &[button(300.0, 110.0, false), Event::PointerGone],
                rect(),
                false
            ),
            -1
        );
    }

    #[test]
    fn pinch_vertical_short_and_outside_gestures_do_not_navigate() {
        for (start, end, pinched) in [
            ((300.0, 100.0), (100.0, 100.0), true),
            ((100.0, 100.0), (110.0, 300.0), false),
            ((100.0, 100.0), (120.0, 100.0), false),
            ((500.0, 100.0), (100.0, 100.0), false),
        ] {
            let mut swipe = Swipe::default();
            swipe.update(&[button(start.0, start.1, true)], rect(), false);
            swipe.update(&[], rect(), pinched);
            assert_eq!(
                swipe.update(&[button(end.0, end.1, false)], rect(), false),
                0
            );
        }
    }
}
