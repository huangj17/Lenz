//! Selection geometry uses native screen coordinates: points on macOS, pixels on Windows.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScreenRect {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

impl ScreenRect {
    pub fn new(x: f64, y: f64, w: f64, h: f64) -> Option<Self> {
        ([x, y, w, h, x + w, y + h].iter().all(|v| v.is_finite()) && w > 0.0 && h > 0.0)
            .then_some(Self { x, y, w, h })
    }

    pub fn point(x: f64, y: f64) -> Self {
        Self {
            x,
            y,
            w: 0.0,
            h: 0.0,
        }
    }

    pub fn contains(&self, x: f64, y: f64) -> bool {
        x >= self.x && x < self.x + self.w && y >= self.y && y < self.y + self.h
    }
}

/// Prefer below the selection, then above, and finally clamp into the source monitor.
pub fn below_selection(
    selection: ScreenRect,
    monitor: ScreenRect,
    width: f64,
    height: f64,
    gap: f64,
) -> (f64, f64) {
    let x = (selection.x + selection.w / 2.0 - width / 2.0)
        .clamp(monitor.x, (monitor.x + monitor.w - width).max(monitor.x));
    let below = selection.y + selection.h + gap;
    let above = selection.y - gap - height;
    let y = if below + height <= monitor.y + monitor.h {
        below
    } else if above >= monitor.y {
        above
    } else {
        below.clamp(monitor.y, (monitor.y + monitor.h - height).max(monitor.y))
    };
    (
        x,
        y.clamp(monitor.y, (monitor.y + monitor.h - height).max(monitor.y)),
    )
}

#[derive(Clone, Copy)]
pub struct SelectionPlacement {
    pub selection: ScreenRect,
    pub monitor: ScreenRect,
    pub gap: f64,
    pub position: (f64, f64),
}

impl SelectionPlacement {
    /// Keep the original anchor through content resizes; a user move releases it.
    pub fn resize(&mut self, current: (f64, f64), width: f64, height: f64) -> Option<(f64, f64)> {
        if (current.0 - self.position.0).abs() > 2.0 || (current.1 - self.position.1).abs() > 2.0 {
            return None;
        }
        self.position = below_selection(self.selection, self.monitor, width, height, self.gap);
        Some(self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_anchor_survives_growth_and_returns_below_after_shrink() {
        let selection = ScreenRect::new(500.0, 500.0, 200.0, 40.0).unwrap();
        let monitor = ScreenRect::new(0.0, 0.0, 1200.0, 800.0).unwrap();
        let position = below_selection(selection, monitor, 360.0, 150.0, 12.0);
        assert_eq!(position, (420.0, 552.0));
        let mut placement = SelectionPlacement {
            selection,
            monitor,
            gap: 12.0,
            position,
        };
        assert_eq!(
            placement.resize(position, 360.0, 400.0),
            Some((420.0, 88.0))
        );
        assert_eq!(
            placement.resize((420.0, 88.0), 360.0, 150.0),
            Some(position)
        );
        assert_eq!(placement.resize((400.0, 300.0), 360.0, 150.0), None);
    }

    #[test]
    fn negative_origin_and_scaled_monitor_keep_popup_on_source_screen() {
        let monitor = ScreenRect::new(-2560.0, -200.0, 2560.0, 1440.0).unwrap();
        let selection = ScreenRect::new(-100.0, 1100.0, 50.0, 40.0).unwrap();
        assert_eq!(
            below_selection(selection, monitor, 720.0, 960.0, 24.0),
            (-720.0, 116.0)
        );
        let selection = ScreenRect::point(-2500.0, -190.0);
        assert_eq!(
            below_selection(selection, monitor, 3000.0, 1600.0, 24.0),
            (-2560.0, -200.0)
        );
    }

    #[test]
    fn invalid_accessibility_rectangles_are_ignored() {
        assert!(ScreenRect::new(0.0, 0.0, 0.0, 20.0).is_none());
        assert!(ScreenRect::new(f64::NAN, 0.0, 20.0, 20.0).is_none());
        assert!(ScreenRect::new(0.0, 0.0, 20.0, f64::INFINITY).is_none());
    }
}
