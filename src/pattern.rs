//! A drafted pattern: pieces made of named edges, plus the construction
//! checks that prove the pieces go together.

use crate::geom::{self, Pt};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EdgeKind {
    /// Sewn to another edge: gets the fabric's seam allowance.
    Seam,
    /// Turned up and stitched: gets the hem allowance.
    Hem,
    /// Placed on the fabric fold: no allowance, the piece is cut double.
    Fold,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Edge {
    pub name: &'static str,
    pub kind: EdgeKind,
    /// Overrides the fabric's allowance for this kind (casings, facings).
    pub sa: Option<f64>,
    /// Stitching line, start to end. The next edge starts where this one ends.
    pub pts: Vec<Pt>,
}

impl Edge {
    pub fn new(name: &'static str, kind: EdgeKind, pts: Vec<Pt>) -> Edge {
        Edge { name, kind, sa: None, pts }
    }
    pub fn sa(mut self, mm: f64) -> Edge {
        self.sa = Some(mm);
        self
    }
    pub fn len(&self) -> f64 {
        geom::length(&self.pts)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cut {
    pub count: u32,
    pub fold: bool,
}

impl Cut {
    pub const ONE: Cut = Cut { count: 1, fold: false };
    pub const PAIR: Cut = Cut { count: 2, fold: false };
    pub const FOLD: Cut = Cut { count: 1, fold: true };
    pub fn describe(&self) -> String {
        let n = if self.count == 1 { "Cut 1".to_string() } else { format!("Cut {}", self.count) };
        if self.fold {
            format!("{n} on fold")
        } else {
            n
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Mark {
    /// Notch(es) on an edge, `at` mm from its start.
    Notch { edge: &'static str, at: f64, count: u32 },
    /// Internal line: dart legs, placement lines, slits, fold lines.
    Line { pts: Vec<Pt>, dashed: bool },
}

#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub name: String,
    pub cut: Cut,
    /// Cut from the rib fabric instead of the main one.
    pub rib: bool,
    pub edges: Vec<Edge>,
    pub marks: Vec<Mark>,
    /// The rule that drafted it and the words that asked for it.
    pub rule: String,
    pub words: String,
    /// Where a print starts (underarm/crotch line, centre front), so prints
    /// match across seams. `None` is the origin.
    pub anchor: Option<Pt>,
}

impl Piece {
    pub fn new(name: impl Into<String>, cut: Cut, edges: Vec<Edge>) -> Piece {
        let p = Piece { name: name.into(), cut, rib: false, edges, marks: Vec::new(), rule: String::new(), words: String::new(), anchor: None };
        for w in p.edges.windows(2) {
            let (a, b) = (w[0].pts[w[0].pts.len() - 1], w[1].pts[0]);
            debug_assert!(a.dist(b) < 1e-6, "{}: {} does not meet {}", p.name, w[0].name, w[1].name);
        }
        if cut.fold {
            let f = p.edges.last().expect("edges");
            debug_assert!(f.kind == EdgeKind::Fold, "{}: fold edge must be last", p.name);
            debug_assert!(f.pts.iter().all(|q| q.x.abs() < 1e-6), "{}: fold must lie on x = 0", p.name);
        }
        p
    }
    pub fn rib(mut self) -> Piece {
        self.rib = true;
        self
    }
    pub fn by(mut self, rule: &str, words: &str) -> Piece {
        self.rule = rule.to_string();
        self.words = words.to_string();
        self
    }
    pub fn anchored(mut self, at: Pt) -> Piece {
        self.anchor = Some(at);
        self
    }
    pub fn mark(mut self, m: Mark) -> Piece {
        self.marks.push(m);
        self
    }
    pub fn edge(&self, name: &str) -> &Edge {
        self.edges.iter().find(|e| e.name == name).unwrap_or_else(|| panic!("{} has no edge {name}", self.name))
    }
    pub fn has_edge(&self, name: &str) -> bool {
        self.edges.iter().any(|e| e.name == name)
    }
    pub fn len(&self, edge: &str) -> f64 {
        self.edge(edge).len()
    }

    /// The closed stitching outline and, per segment, the edge it belongs to.
    pub fn outline(&self) -> (Vec<Pt>, Vec<usize>) {
        let mut pts = Vec::new();
        let mut owner = Vec::new();
        for (i, e) in self.edges.iter().enumerate() {
            for &p in &e.pts[..e.pts.len() - 1] {
                pts.push(p);
                owner.push(i);
            }
        }
        (pts, owner)
    }

    /// Stitching and cutting outlines. With `unfold`, a piece cut on the
    /// fold is mirrored into its whole shape (the fold edge disappears).
    pub fn outlines(&self, seam: f64, hem: f64, unfold: bool) -> (Vec<Pt>, Vec<Pt>) {
        let allowance = |e: &Edge| {
            e.sa.unwrap_or(match e.kind {
                EdgeKind::Seam => seam,
                EdgeKind::Hem => hem,
                EdgeKind::Fold => 0.0,
            })
        };
        let (mut pts, owner) = self.outline();
        let mut dist: Vec<f64> = owner.iter().map(|&i| allowance(&self.edges[i])).collect();
        if unfold && self.cut.fold {
            // Drop the fold edge: the chain runs fold-top -> ... -> fold-bottom.
            let fold = self.edges.len() - 1;
            let keep: Vec<usize> = (0..pts.len()).filter(|&k| owner[k] != fold).collect();
            let mut chain: Vec<Pt> = keep.iter().map(|&k| pts[k]).collect();
            let mut d: Vec<f64> = keep.iter().map(|&k| dist[k]).collect();
            chain.push(self.edges[fold].pts[0]);
            // Mirror back up: segment k of the mirror runs the other way.
            let n = chain.len();
            let mut whole = chain.clone();
            let mut wd = d.clone();
            for k in (1..n - 1).rev() {
                whole.push(chain[k].mirror_x());
            }
            for k in (0..n - 1).rev() {
                wd.push(d[k]);
            }
            d.clear();
            pts = whole;
            dist = wd;
        }
        let cut = geom::offset(&pts, &dist);
        (pts, cut)
    }
}

/// One construction check. Hard checks must pass for the pattern to be output.
#[derive(Clone, Debug, PartialEq)]
pub struct Check {
    pub name: String,
    pub detail: String,
    pub ok: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    pub title: String,
    pub size: String,
    pub fabric: String,
    pub seam: f64,
    pub hem: f64,
    pub rib_seam: f64,
    pub rib_hem: f64,
    pub pieces: Vec<Piece>,
    pub checks: Vec<Check>,
    pub notes: Vec<String>,
}

impl Pattern {
    pub fn piece(&self, name: &str) -> &Piece {
        self.pieces.iter().find(|p| p.name == name).unwrap_or_else(|| panic!("no piece {name}"))
    }
    pub fn allowances(&self, p: &Piece) -> (f64, f64) {
        if p.rib {
            (self.rib_seam, self.rib_hem)
        } else {
            (self.seam, self.hem)
        }
    }
    pub fn ok(&self) -> bool {
        self.checks.iter().all(|c| c.ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::pt;

    fn half_rect() -> Piece {
        Piece::new(
            "panel",
            Cut::FOLD,
            vec![
                Edge::new("top", EdgeKind::Seam, vec![pt(0.0, 0.0), pt(100.0, 0.0)]),
                Edge::new("side", EdgeKind::Seam, vec![pt(100.0, 0.0), pt(100.0, 200.0)]),
                Edge::new("hem", EdgeKind::Hem, vec![pt(100.0, 200.0), pt(0.0, 200.0)]),
                Edge::new("fold", EdgeKind::Fold, vec![pt(0.0, 200.0), pt(0.0, 0.0)]),
            ],
        )
    }

    #[test]
    fn half_piece_has_no_allowance_on_the_fold() {
        let (_, cut) = half_rect().outlines(10.0, 30.0, false);
        let r = geom::bbox(&cut);
        assert_eq!((r.min.x, r.max.x, r.min.y, r.max.y), (0.0, 110.0, -10.0, 230.0));
    }

    #[test]
    fn unfolded_piece_is_whole() {
        let (stitch, cut) = half_rect().outlines(10.0, 30.0, true);
        assert_eq!(geom::bbox(&stitch).width(), 200.0);
        let r = geom::bbox(&cut);
        assert_eq!((r.min.x, r.max.x), (-110.0, 110.0));
        assert!(!geom::self_intersects(&cut));
    }
}
