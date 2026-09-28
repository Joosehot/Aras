//! Rectangles: bands, cuffs, strips and waistbands. Their length runs along
//! x, which for rib is the stretch direction (across the fabric).

use crate::geom::pt;
use crate::pattern::{Cut, Edge, EdgeKind, Mark, Piece};

/// A plain rectangle `w` by `h` with seam allowance all round.
pub fn rect(name: &str, cut: Cut, w: f64, h: f64) -> Piece {
    Piece::new(
        name,
        cut,
        vec![
            Edge::new("top", EdgeKind::Seam, vec![pt(0.0, 0.0), pt(w, 0.0)]),
            Edge::new("end_b", EdgeKind::Seam, vec![pt(w, 0.0), pt(w, h)]),
            Edge::new("bottom", EdgeKind::Seam, vec![pt(w, h), pt(0.0, h)]),
            Edge::new("end_a", EdgeKind::Seam, vec![pt(0.0, h), pt(0.0, 0.0)]),
        ],
    )
}

/// A band folded in half lengthwise: cut twice its finished height, with the
/// fold line marked.
pub fn folded_band(name: &str, cut: Cut, length: f64, finished: f64) -> Piece {
    rect(name, cut, length, finished * 2.0).mark(Mark::Line { pts: vec![pt(0.0, finished), pt(length, finished)], dashed: true })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folded_band_is_double_height() {
        let b = folded_band("band", Cut::ONE, 400.0, 15.0);
        assert_eq!(b.len("top"), 400.0);
        assert_eq!(b.len("end_a"), 30.0);
    }
}
