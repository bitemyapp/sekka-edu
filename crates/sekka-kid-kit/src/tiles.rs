//! Tiles of one width in rows: as many to a row as are at least the
//! minimum wide, the rest wrapping onto rows of their own, the rows
//! balanced (five tiles go three and two, not four and one). A report's
//! figures stay whole in a narrow window instead of losing their ends.

use aizome_kit::Element;
use aizome_kit::layout::{Constraints, Point, Size};
use aizome_kit::widget::{LayoutCx, Widget};

/// `children` as tiles at least `min` wide, `spacing` apart both ways.
pub fn tiles<M>(children: Vec<Element<M>>, min: f64, spacing: f64) -> Element<M> {
    Element::new(Tiles { min, spacing }, children)
}

struct Tiles {
    min: f64,
    spacing: f64,
}

impl Tiles {
    /// How many to a row in `width`, for `n` tiles.
    fn per_row(&self, n: usize, width: f64) -> usize {
        let fit = (((width + self.spacing) / (self.min + self.spacing)).floor() as usize).clamp(1, n.max(1));
        n.div_ceil(n.div_ceil(fit))
    }
}

impl Widget for Tiles {
    fn name(&self) -> &'static str {
        "tiles"
    }

    fn layout(&mut self, cx: &mut LayoutCx, bc: Constraints) -> Size {
        let n = cx.child_count();
        if n == 0 {
            return bc.constrain(Size::ZERO);
        }
        let width =
            if bc.max.width.is_finite() { bc.max.width } else { n as f64 * (self.min + self.spacing) - self.spacing };
        let per = self.per_row(n, width);
        let w = ((width - self.spacing * (per - 1) as f64) / per as f64).floor().max(0.0);
        let mut y = 0.0;
        for first in (0..n).step_by(per) {
            let row = first..(first + per).min(n);
            // A row is as tall as its tallest tile, and every tile in it
            // takes that height, so their faces line up.
            let loose = Constraints { min: Size::new(w, 0.0), max: Size::new(w, f64::INFINITY) };
            let h = row.clone().map(|i| cx.layout_child(i, loose).height).fold(0.0, f64::max);
            for (k, i) in row.enumerate() {
                cx.layout_child(i, Constraints::tight(Size::new(w, h)));
                cx.place_child(i, Point::new(k as f64 * (w + self.spacing), y));
            }
            y += h + self.spacing;
        }
        bc.constrain(Size::new(width, y - self.spacing))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_are_balanced() {
        let t = Tiles { min: 125.0, spacing: 10.0 };
        // Five fit in a wide window; three and two in a narrow one.
        assert_eq!(t.per_row(5, 700.0), 5);
        assert_eq!(t.per_row(5, 600.0), 3);
        assert_eq!(t.per_row(5, 472.0), 3);
        assert_eq!(t.per_row(5, 300.0), 2);
        assert_eq!(t.per_row(5, 100.0), 1);
        assert_eq!(t.per_row(4, 560.0), 4);
        assert_eq!(t.per_row(4, 500.0), 2);
    }
}
