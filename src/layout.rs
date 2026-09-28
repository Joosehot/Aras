//! Cutting layout (a marker): every piece placed on single-layer fabric at
//! full width, grain parallel to the selvedge. Pieces cut on the fold are
//! unfolded, pairs are placed twice (the second mirrored, so you get a left
//! and a right). Rib pieces go on their own, narrower fabric.
//!
//! Packing is a bottom-left skyline over bounding boxes, largest first,
//! with name order breaking ties, so the layout is deterministic.

use crate::config::Config;
use crate::geom::{self, Pt, Rect};
use crate::pattern::Pattern;
use std::cmp::Ordering;

#[derive(Clone, Debug, PartialEq)]
pub struct Placement {
    pub piece: usize,
    pub copy: u32,
    pub mirror: bool,
    pub unfold: bool,
    /// Added to the (mirrored) local coordinates.
    pub shift: Pt,
    /// Where it sits on the fabric.
    pub rect: Rect,
}

impl Placement {
    pub fn place(&self, p: Pt) -> Pt {
        let q = if self.mirror { p.mirror_x() } else { p };
        q + self.shift
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Sheet {
    pub fabric: String,
    /// Colourway of this fabric ("" when colours aren't known yet).
    pub label: String,
    pub rib: bool,
    pub width: f64,
    pub length: f64,
    pub placed: Vec<Placement>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Marker {
    pub sheets: Vec<Sheet>,
}

impl Marker {
    /// Fabric area used, m².
    pub fn area_m2(&self) -> f64 {
        self.sheets.iter().map(|s| s.width * s.length).sum::<f64>() / 1e6
    }
}

struct Item {
    piece: usize,
    copy: u32,
    mirror: bool,
    unfold: bool,
    bbox: Rect,
    name: String,
}

#[derive(Clone, Copy)]
struct Seg {
    x: f64,
    w: f64,
    y: f64,
}

fn pack(mut items: Vec<Item>, width: f64, gap: f64) -> (Vec<Placement>, f64) {
    items.sort_by(|a, b| {
        b.bbox
            .height()
            .partial_cmp(&a.bbox.height())
            .unwrap_or(Ordering::Equal)
            .then(b.bbox.width().partial_cmp(&a.bbox.width()).unwrap_or(Ordering::Equal))
            .then(a.name.cmp(&b.name))
            .then(a.copy.cmp(&b.copy))
    });
    let mut sky = vec![Seg { x: 0.0, w: width, y: 0.0 }];
    let mut out = Vec::new();
    let mut length: f64 = 0.0;
    for it in items {
        let (w, h) = (it.bbox.width(), it.bbox.height());
        let mut best: Option<(f64, f64)> = None;
        for s in &sky {
            let x = s.x;
            if x + w > width + 1e-9 && x > 0.0 {
                continue;
            }
            let y = sky.iter().filter(|t| t.x < x + w + gap && t.x + t.w > x).map(|t| t.y).fold(0.0, f64::max);
            if best.is_none_or(|(bx, by)| y < by - 1e-9 || (y < by + 1e-9 && x < bx)) {
                best = Some((x, y));
            }
        }
        let (x, y) = best.expect("the skyline always has a segment at x = 0");
        let rect = Rect { min: geom::pt(x, y), max: geom::pt(x + w, y + h) };
        length = length.max(y + h);
        // New skyline: cut out [x, x + w + gap) and put the piece's top there.
        let (l, r) = (x, x + w + gap);
        let mut next = Vec::new();
        for s in &sky {
            if s.x < l {
                next.push(Seg { x: s.x, w: (s.x + s.w).min(l) - s.x, y: s.y });
            }
            if s.x + s.w > r {
                let nx = s.x.max(r);
                next.push(Seg { x: nx, w: s.x + s.w - nx, y: s.y });
            }
        }
        next.push(Seg { x: l, w: (r - l).min(width.max(r) - l), y: y + h + gap });
        next.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap_or(Ordering::Equal));
        sky.clear();
        for s in next.into_iter().filter(|s| s.w > 1e-9) {
            match sky.last_mut() {
                Some(last) if (last.y - s.y).abs() < 1e-9 => last.w += s.w,
                _ => sky.push(s),
            }
        }
        out.push(Placement {
            piece: it.piece,
            copy: it.copy,
            mirror: it.mirror,
            unfold: it.unfold,
            shift: geom::pt(x - it.bbox.min.x, y - it.bbox.min.y),
            rect,
        });
    }
    out.sort_by_key(|p| (p.piece, p.copy));
    (out, length)
}

fn item(p: &Pattern, i: usize, copy: u32, mirror: bool, unfold: bool) -> Item {
    let piece = &p.pieces[i];
    let (seam, hem) = p.allowances(piece);
    let (_, cut) = piece.outlines(seam, hem, unfold);
    let pts: Vec<Pt> = cut.iter().map(|q| if mirror { q.mirror_x() } else { *q }).collect();
    Item { piece: i, copy, mirror, unfold, bbox: geom::bbox(&pts), name: piece.name.clone() }
}

/// The cutting layout: main fabric, then rib.
pub fn marker(p: &Pattern, cfg: &Config) -> Marker {
    marker_by(p, cfg, |_, _| String::new())
}

/// One fabric sheet per (material, `colour(piece)`), in order of first use.
pub fn marker_by(p: &Pattern, cfg: &Config, colour: impl Fn(usize, &crate::pattern::Piece) -> String) -> Marker {
    let gap = cfg.layout("gap");
    let mut groups: Vec<(bool, String, Vec<usize>)> = Vec::new();
    for rib in [false, true] {
        for (i, piece) in p.pieces.iter().enumerate().filter(|(_, x)| x.rib == rib) {
            let label = colour(i, piece);
            match groups.iter_mut().find(|(r, l, _)| *r == rib && *l == label) {
                Some(g) => g.2.push(i),
                None => groups.push((rib, label, vec![i])),
            }
        }
    }
    let mut sheets = Vec::new();
    for (rib, label, idx) in groups {
        let mut items = Vec::new();
        for i in idx {
            let piece = &p.pieces[i];
            for copy in 0..piece.cut.count {
                items.push(item(p, i, copy, !piece.cut.fold && copy % 2 == 1, piece.cut.fold));
            }
        }
        let fabric = if rib { "rib".to_string() } else { p.fabric.clone() };
        let width = cfg.fabric(&fabric).width;
        let (placed, length) = pack(items, width, gap);
        sheets.push(Sheet { fabric, label, rib, width, length, placed });
    }
    Marker { sheets }
}

/// The printable pattern: every piece once, halves on the fold.
pub fn pattern_sheet(p: &Pattern, cfg: &Config) -> Sheet {
    let items = (0..p.pieces.len()).map(|i| item(p, i, 0, false, false)).collect();
    let width = cfg.layout("sheet_width");
    let (placed, length) = pack(items, width, cfg.layout("gap") * 4.0);
    Sheet { fabric: "paper".into(), label: String::new(), rib: false, width, length, placed }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_util::ok;

    fn overlaps(a: &Rect, b: &Rect) -> bool {
        a.min.x < b.max.x - 1e-6 && b.min.x < a.max.x - 1e-6 && a.min.y < b.max.y - 1e-6 && b.min.y < a.max.y - 1e-6
    }

    #[test]
    fn pieces_fit_the_fabric_and_never_overlap() {
        let cfg = Config::builtin();
        for s in ["make a hoodie", "make a shirt", "make baggy pants"] {
            let m = marker(&ok(s, &[]), &cfg);
            for sheet in &m.sheets {
                for (i, a) in sheet.placed.iter().enumerate() {
                    assert!(a.rect.min.x >= -1e-6 && a.rect.max.x <= sheet.width + 1e-6, "{s}: off the fabric");
                    for b in &sheet.placed[i + 1..] {
                        assert!(!overlaps(&a.rect, &b.rect), "{s}: overlap");
                    }
                }
            }
        }
    }

    #[test]
    fn pairs_are_placed_twice_and_mirrored() {
        let cfg = Config::builtin();
        let p = ok("make a t-shirt", &[]);
        let m = marker(&p, &cfg);
        let sleeve = p.pieces.iter().position(|x| x.name == "sleeve").unwrap();
        let copies: Vec<_> = m.sheets[0].placed.iter().filter(|x| x.piece == sleeve).collect();
        assert_eq!(copies.len(), 2);
        assert_ne!(copies[0].mirror, copies[1].mirror);
    }
}
