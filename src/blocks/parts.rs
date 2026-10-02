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

/// A polyline moved sideways by `w`: to the right of travel in this
/// y-down frame (the normal of a step d is (-d.y, d.x)).
fn offset_line(pts: &[crate::geom::Pt], w: f64) -> Vec<crate::geom::Pt> {
    let n = pts.len();
    let normal = |a: crate::geom::Pt, b: crate::geom::Pt| {
        let d = (b - a).unit();
        pt(-d.y, d.x)
    };
    (0..n)
        .map(|i| {
            let nn = match (i.checked_sub(1), (i + 1 < n).then_some(i + 1)) {
                (Some(a), Some(b)) => (normal(pts[a], pts[i]) + normal(pts[i], pts[b])).unit(),
                (None, Some(b)) => normal(pts[i], pts[b]),
                (Some(a), None) => normal(pts[a], pts[i]),
                (None, None) => pt(0.0, 0.0),
            };
            pts[i] + nn * w
        })
        .collect()
}

/// A neck facing cut to a body piece's neckline, `w` wide: on the fold at
/// the centre unless the centre is a seam (a zip).
pub fn neck_facing(name: &str, body: &Piece, w: f64) -> Result<Piece, String> {
    let neck = body.edge("neck").pts.clone();
    let hps = *neck.last().expect("neck");
    let sh = body.edge("shoulder").pts.clone();
    let sdir = (sh[1] - sh[0]).unit();
    let mut inner = offset_line(&neck, w);
    let last = inner.len() - 1;
    inner[0] = pt(neck[0].x, neck[0].y + w);
    inner[last] = hps + sdir * w;
    let centre_seam = !body.cut.fold;
    let mut back = inner.clone();
    back.reverse();
    let mut edges = vec![
        Edge::new("neck", EdgeKind::Seam, neck.clone()),
        Edge::new("shoulder", EdgeKind::Seam, vec![hps, inner[last]]),
        Edge::new("inner", EdgeKind::Hem, back).sa(5.0),
    ];
    let cut = if centre_seam {
        edges.push(Edge::new("centre", EdgeKind::Seam, vec![inner[0], neck[0]]));
        Cut::PAIR
    } else {
        edges.push(Edge::new("centre", EdgeKind::Fold, vec![inner[0], neck[0]]));
        Cut::FOLD
    };
    Ok(Piece::new(name, cut, edges))
}

/// An armhole facing cut to a body piece's armhole, `w` wide, from the
/// shoulder seam to the side seam.
pub fn armhole_facing(name: &str, body: &Piece, w: f64) -> Result<Piece, String> {
    let arm = body.edge("armhole").pts.clone();
    let (sp, c) = (arm[0], *arm.last().expect("armhole"));
    let sh = body.edge("shoulder").pts.clone();
    let in_dir = (sh[0] - sh[1]).unit();
    let side = &body.edge("side").pts;
    let (side_pt, _) = crate::geom::at_length(side, w);
    let mut inner = offset_line(&arm, w);
    let last = inner.len() - 1;
    inner[0] = sp + in_dir * w;
    inner[last] = side_pt;
    let mut back = inner.clone();
    back.reverse();
    let edges = vec![
        Edge::new("armhole", EdgeKind::Seam, arm),
        Edge::new("side", EdgeKind::Seam, vec![c, side_pt]),
        Edge::new("inner", EdgeKind::Hem, back).sa(5.0),
        Edge::new("shoulder", EdgeKind::Seam, vec![inner[0], sp]),
    ];
    Ok(Piece::new(name, Cut::PAIR, edges))
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
