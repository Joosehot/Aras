//! A pattern piece as a cloth panel: the whole piece (a half cut on the fold
//! or with a centre-back seam is mirrored into both halves, the centre
//! closed), its outline sampled every `spacing` mm with each sample knowing
//! which seam it lies on and where along it, an even lattice inside, and a
//! Delaunay triangulation (Bowyer-Watson) of the lot.

use crate::geom::{self, pt, Pt};
use crate::pattern::{EdgeKind, Piece};

/// Where a boundary point lies: the edge's name, the half (+1 right as
/// drafted, -1 the mirrored left), and 0..1 along the edge as drafted.
#[derive(Clone, Debug)]
pub struct OnEdge {
    pub edge: &'static str,
    pub side: i8,
    pub t: f64,
}

pub struct Panel {
    pub name: String,
    pub p: Vec<Pt>,
    pub on: Vec<Option<OnEdge>>,
    pub tri: Vec<[usize; 3]>,
}

fn is_centre(e: &str, kind: EdgeKind) -> bool {
    kind == EdgeKind::Fold || matches!(e, "cb" | "cf")
}

pub fn build(piece: &Piece, spacing: f64) -> Panel {
    // The right half's outline without the centre edge, as samples.
    let mut half: Vec<(Pt, Option<OnEdge>)> = Vec::new();
    for e in piece.edges.iter().filter(|e| !is_centre(e.name, e.kind)) {
        let len = e.len();
        let n = (len / spacing).ceil().max(1.0) as usize;
        for i in 0..n {
            let s = len * i as f64 / n as f64;
            half.push((geom::at_length(&e.pts, s).0, Some(OnEdge { edge: e.name, side: 1, t: s / len })));
        }
    }
    // the half ends where the centre edge starts: add that corner
    if let Some(c) = piece.edges.iter().find(|e| is_centre(e.name, e.kind)) {
        half.push((c.pts[0], None));
    }
    // The whole outline: the right half, then the left half mirrored and
    // run backwards. Points on the centre line are shared, not doubled.
    let mut p: Vec<Pt> = Vec::new();
    let mut on: Vec<Option<OnEdge>> = Vec::new();
    for (q, o) in &half {
        p.push(*q);
        on.push(o.clone());
    }
    for (q, o) in half.iter().rev() {
        if q.x.abs() < 1e-6 {
            continue;
        }
        p.push(pt(-q.x, q.y));
        on.push(o.as_ref().map(|o| OnEdge { side: -1, ..o.clone() }));
    }
    let outline = p.clone();
    // the lattice inside, staggered rows, clear of the outline
    let bb = geom::bbox(&outline);
    let h = spacing * 0.866;
    let mut row = 0;
    let mut y = bb.min.y + h;
    while y < bb.max.y {
        let off = if row % 2 == 0 { 0.0 } else { spacing / 2.0 };
        let mut x = bb.min.x + off + spacing / 2.0;
        while x < bb.max.x {
            let q = pt(x, y);
            if inside(&outline, q) && dist_to_outline(&outline, q) > spacing * 0.45 {
                p.push(q);
                on.push(None);
            }
            x += spacing;
        }
        y += h;
        row += 1;
    }
    let tri = delaunay(&p).into_iter().filter(|t| inside(&outline, centroid(&p, t))).collect();
    Panel { name: piece.name.clone(), p, on, tri }
}

fn centroid(p: &[Pt], t: &[usize; 3]) -> Pt {
    pt((p[t[0]].x + p[t[1]].x + p[t[2]].x) / 3.0, (p[t[0]].y + p[t[1]].y + p[t[2]].y) / 3.0)
}

fn inside(poly: &[Pt], q: Pt) -> bool {
    let mut c = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > q.y) != (b.y > q.y) && q.x < a.x + (q.y - a.y) / (b.y - a.y) * (b.x - a.x) {
            c = !c;
        }
    }
    c
}

fn dist_to_outline(poly: &[Pt], q: Pt) -> f64 {
    let n = poly.len();
    (0..n)
        .map(|i| {
            let (a, b) = (poly[i], poly[(i + 1) % n]);
            let ab = b - a;
            let t = (((q - a).x * ab.x + (q - a).y * ab.y) / (ab.x * ab.x + ab.y * ab.y).max(1e-12)).clamp(0.0, 1.0);
            q.dist(a + ab * t)
        })
        .fold(f64::INFINITY, f64::min)
}

/// Bowyer-Watson Delaunay triangulation.
fn delaunay(p: &[Pt]) -> Vec<[usize; 3]> {
    let bb = geom::bbox(p);
    let d = (bb.width() + bb.height()) * 10.0;
    let c = pt((bb.min.x + bb.max.x) / 2.0, (bb.min.y + bb.max.y) / 2.0);
    let mut pts = p.to_vec();
    let n = pts.len();
    pts.push(pt(c.x - d, c.y - d));
    pts.push(pt(c.x + d, c.y - d));
    pts.push(pt(c.x, c.y + d));
    let circum = |t: &[usize; 3]| {
        let (a, b, cc) = (pts[t[0]], pts[t[1]], pts[t[2]]);
        let dd = 2.0 * (a.x * (b.y - cc.y) + b.x * (cc.y - a.y) + cc.x * (a.y - b.y));
        let ux = ((a.x * a.x + a.y * a.y) * (b.y - cc.y) + (b.x * b.x + b.y * b.y) * (cc.y - a.y) + (cc.x * cc.x + cc.y * cc.y) * (a.y - b.y)) / dd;
        let uy = ((a.x * a.x + a.y * a.y) * (cc.x - b.x) + (b.x * b.x + b.y * b.y) * (a.x - cc.x) + (cc.x * cc.x + cc.y * cc.y) * (b.x - a.x)) / dd;
        (pt(ux, uy), (a.x - ux).powi(2) + (a.y - uy).powi(2))
    };
    let mut tris: Vec<([usize; 3], Pt, f64)> = vec![{
        let t = [n, n + 1, n + 2];
        let (cc, r2) = circum(&t);
        (t, cc, r2)
    }];
    for i in 0..n {
        let q = pts[i];
        let (bad, good): (Vec<_>, Vec<_>) = tris.into_iter().partition(|(_, cc, r2)| (q.x - cc.x).powi(2) + (q.y - cc.y).powi(2) < *r2);
        tris = good;
        // the hole's boundary: edges of bad triangles that only one of them has
        let mut edges: Vec<(usize, usize)> = Vec::new();
        for (t, _, _) in &bad {
            for k in 0..3 {
                let e = (t[k], t[(k + 1) % 3]);
                if let Some(j) = edges.iter().position(|f| (f.0 == e.1 && f.1 == e.0) || *f == e) {
                    edges.swap_remove(j);
                } else {
                    edges.push(e);
                }
            }
        }
        for (a, b) in edges {
            let t = [a, b, i];
            let (cc, r2) = circum(&t);
            tris.push((t, cc, r2));
        }
    }
    tris.into_iter().map(|(t, _, _)| t).filter(|t| t.iter().all(|&k| k < n)).collect()
}
