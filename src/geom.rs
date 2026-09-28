//! Plane geometry in millimetres, y pointing down (the way a pattern is
//! drafted on paper: top of the garment first). Curves are cubic Béziers
//! sampled at a fixed count, so the same inputs always give the same points.

use std::ops::{Add, Mul, Sub};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
}

pub const fn pt(x: f64, y: f64) -> Pt {
    Pt { x, y }
}

impl Add for Pt {
    type Output = Pt;
    fn add(self, o: Pt) -> Pt {
        pt(self.x + o.x, self.y + o.y)
    }
}

impl Sub for Pt {
    type Output = Pt;
    fn sub(self, o: Pt) -> Pt {
        pt(self.x - o.x, self.y - o.y)
    }
}

impl Mul<f64> for Pt {
    type Output = Pt;
    fn mul(self, k: f64) -> Pt {
        pt(self.x * k, self.y * k)
    }
}

impl Pt {
    pub fn len(self) -> f64 {
        self.x.hypot(self.y)
    }
    pub fn dist(self, o: Pt) -> f64 {
        (self - o).len()
    }
    pub fn unit(self) -> Pt {
        let l = self.len();
        if l == 0.0 {
            self
        } else {
            self * (1.0 / l)
        }
    }
    pub fn cross(self, o: Pt) -> f64 {
        self.x * o.y - self.y * o.x
    }
    pub fn lerp(self, o: Pt, t: f64) -> Pt {
        self + (o - self) * t
    }
    pub fn mirror_x(self) -> Pt {
        pt(-self.x, self.y)
    }
}

/// Samples per Bézier segment. Fixed, so lengths are reproducible.
pub const CURVE_STEPS: usize = 24;

/// A cubic Bézier sampled into `CURVE_STEPS + 1` points, both ends included.
pub fn cubic(p0: Pt, p1: Pt, p2: Pt, p3: Pt) -> Vec<Pt> {
    (0..=CURVE_STEPS)
        .map(|i| {
            let t = i as f64 / CURVE_STEPS as f64;
            let u = 1.0 - t;
            p0 * (u * u * u) + p1 * (3.0 * u * u * t) + p2 * (3.0 * u * t * t) + p3 * (t * t * t)
        })
        .collect()
}

/// Joins polylines that share end points into one.
pub fn chain(parts: &[Vec<Pt>]) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::new();
    for part in parts {
        for &p in part {
            if out.last().is_none_or(|l| l.dist(p) > 1e-9) {
                out.push(p);
            }
        }
    }
    out
}

pub fn length(pts: &[Pt]) -> f64 {
    pts.windows(2).map(|w| w[0].dist(w[1])).sum()
}

/// Point at arc length `s` along a polyline (clamped to its ends), and the
/// unit direction of travel there.
pub fn at_length(pts: &[Pt], s: f64) -> (Pt, Pt) {
    let mut left = s.max(0.0);
    for w in pts.windows(2) {
        let d = w[0].dist(w[1]);
        if d > 0.0 && left <= d {
            return (w[0].lerp(w[1], left / d), (w[1] - w[0]).unit());
        }
        left -= d;
    }
    let n = pts.len();
    (pts[n - 1], (pts[n - 1] - pts[n - 2]).unit())
}

/// Shoelace area; positive means clockwise on screen (y down).
pub fn signed_area(poly: &[Pt]) -> f64 {
    let n = poly.len();
    (0..n).map(|i| poly[i].cross(poly[(i + 1) % n])).sum::<f64>() / 2.0
}

/// Outward offset of a closed polygon. `dist[i]` is the distance for the
/// segment from `poly[i]` to `poly[i + 1]`, so neighbouring edges can have
/// different seam allowances; corners are mitred, very sharp ones bevelled.
pub fn offset(poly: &[Pt], dist: &[f64]) -> Vec<Pt> {
    let n = poly.len();
    assert_eq!(n, dist.len());
    let sign = if signed_area(poly) >= 0.0 { 1.0 } else { -1.0 };
    let normal = |i: usize| {
        let d = (poly[(i + 1) % n] - poly[i]).unit();
        pt(d.y, -d.x) * sign
    };
    let mut out = Vec::with_capacity(n + 8);
    for j in 0..n {
        let i = (j + n - 1) % n;
        let (n1, n2) = (normal(i), normal(j));
        let (d1, d2) = (dist[i], dist[j]);
        let u1 = (poly[j] - poly[i]).unit();
        let u2 = (poly[(j + 1) % n] - poly[j]).unit();
        let a1 = poly[j] + n1 * d1;
        let a2 = poly[j] + n2 * d2;
        let c = u1.cross(u2);
        if c.abs() < 1e-9 {
            out.push(a1);
            if (d1 - d2).abs() > 1e-9 {
                out.push(a2);
            }
            continue;
        }
        let t = (a2 - a1).cross(u2) / c;
        let q = a1 + u1 * t;
        if q.dist(poly[j]) > 4.0 * d1.max(d2).max(1.0) {
            out.push(a1);
            out.push(a2);
        } else {
            out.push(q);
        }
    }
    remove_loops(dedupe(out))
}

/// Where segments `a-b` and `c-d` cross, if they do.
fn crossing(a: Pt, b: Pt, c: Pt, d: Pt) -> Option<Pt> {
    if !segments_cross(a, b, c, d) {
        return None;
    }
    let (r, s) = (b - a, d - c);
    let t = (c - a).cross(s) / r.cross(s);
    Some(a + r * t)
}

/// Offsetting toward a tight inside curve or corner leaves small
/// "swallowtail" loops; cut each one out at its crossing point.
fn remove_loops(mut pts: Vec<Pt>) -> Vec<Pt> {
    'again: loop {
        let n = pts.len();
        if n < 4 {
            return pts;
        }
        let window = (n / 3).max(2);
        for i in 0..n {
            for k in 2..=window.min(n - 2) {
                let j = (i + k) % n;
                if let Some(x) = crossing(pts[i], pts[(i + 1) % n], pts[j], pts[(j + 1) % n]) {
                    // Keep j+1 .. i (walking forward), then the crossing.
                    let mut next = Vec::with_capacity(n);
                    let mut idx = (j + 1) % n;
                    loop {
                        next.push(pts[idx]);
                        if idx == i {
                            break;
                        }
                        idx = (idx + 1) % n;
                    }
                    next.push(x);
                    pts = dedupe(next);
                    continue 'again;
                }
            }
        }
        return pts;
    }
}

fn dedupe(pts: Vec<Pt>) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::with_capacity(pts.len());
    for p in pts {
        if out.last().is_none_or(|l| l.dist(p) > 1e-6) {
            out.push(p);
        }
    }
    while out.len() > 1 && out[0].dist(out[out.len() - 1]) <= 1e-6 {
        out.pop();
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub min: Pt,
    pub max: Pt,
}

impl Rect {
    pub fn width(&self) -> f64 {
        self.max.x - self.min.x
    }
    pub fn height(&self) -> f64 {
        self.max.y - self.min.y
    }
}

pub fn bbox(pts: &[Pt]) -> Rect {
    let mut r = Rect { min: pts[0], max: pts[0] };
    for p in pts {
        r.min.x = r.min.x.min(p.x);
        r.min.y = r.min.y.min(p.y);
        r.max.x = r.max.x.max(p.x);
        r.max.y = r.max.y.max(p.y);
    }
    r
}

fn segments_cross(a: Pt, b: Pt, c: Pt, d: Pt) -> bool {
    let d1 = (b - a).cross(c - a);
    let d2 = (b - a).cross(d - a);
    let d3 = (d - c).cross(a - c);
    let d4 = (d - c).cross(b - c);
    ((d1 > 1e-9 && d2 < -1e-9) || (d1 < -1e-9 && d2 > 1e-9))
        && ((d3 > 1e-9 && d4 < -1e-9) || (d3 < -1e-9 && d4 > 1e-9))
}

/// True when two non-neighbouring edges of the closed polygon cross.
pub fn self_intersects(poly: &[Pt]) -> bool {
    let n = poly.len();
    for i in 0..n {
        for j in i + 2..n {
            if i == 0 && j == n - 1 {
                continue;
            }
            if segments_cross(poly[i], poly[(i + 1) % n], poly[j], poly[(j + 1) % n]) {
                return true;
            }
        }
    }
    false
}

/// Solves `f(x) = 0` for increasing `f` on `[lo, hi]` by bisection.
/// `None` when the root is outside the bracket.
pub fn solve(lo: f64, hi: f64, f: impl Fn(f64) -> f64) -> Option<f64> {
    let (mut lo, mut hi) = (lo, hi);
    if f(lo) > 0.0 || f(hi) < 0.0 {
        return None;
    }
    for _ in 0..60 {
        let mid = (lo + hi) / 2.0;
        if f(mid) > 0.0 {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    Some((lo + hi) / 2.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square() -> Vec<Pt> {
        vec![pt(0.0, 0.0), pt(100.0, 0.0), pt(100.0, 100.0), pt(0.0, 100.0)]
    }

    #[test]
    fn offset_grows_a_square_outward() {
        let o = offset(&square(), &[10.0; 4]);
        let r = bbox(&o);
        assert!((r.width() - 120.0).abs() < 1e-9 && (r.height() - 120.0).abs() < 1e-9);
    }

    #[test]
    fn offset_honours_per_edge_distance() {
        let o = offset(&square(), &[0.0, 10.0, 25.0, 10.0]);
        let r = bbox(&o);
        assert_eq!((r.min.y, r.max.y), (0.0, 125.0));
    }

    #[test]
    fn offset_into_a_notch_has_no_loops() {
        // A thin notch cut into a square: offsetting outward crosses itself
        // inside the notch unless the loop is removed.
        let poly = vec![pt(0.0, 0.0), pt(45.0, 0.0), pt(50.0, 30.0), pt(55.0, 0.0), pt(100.0, 0.0), pt(100.0, 100.0), pt(0.0, 100.0)];
        let o = offset(&poly, &[10.0; 7]);
        assert!(!self_intersects(&o), "{o:?}");
    }

    #[test]
    fn bowtie_self_intersects() {
        let bow = vec![pt(0.0, 0.0), pt(10.0, 10.0), pt(10.0, 0.0), pt(0.0, 10.0)];
        assert!(self_intersects(&bow));
        assert!(!self_intersects(&square()));
    }

    #[test]
    fn solve_finds_root() {
        let x = solve(0.0, 10.0, |x| x * x - 4.0).unwrap();
        assert!((x - 2.0).abs() < 1e-9);
        assert!(solve(3.0, 10.0, |x| x * x - 4.0).is_none());
    }
}
