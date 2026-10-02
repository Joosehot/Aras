//! The mannequin: a tailor's dress form built from an Aras size, the way a
//! dressmaker fits on one. Neck to upper thigh, no head or arms, on a pole.
//!
//! Heights come from the size: the waist at inside leg + body rise, the
//! seat hip_depth below it, the nape nape_to_waist above it, the underarm
//! scye_depth below the nape, the neck point just above the nape and the
//! shoulder points out along the shoulder and down its slope. Each level is
//! a rounded rectangle (a superellipse) whose girth is the size's girth
//! there; the levels are blended smoothly into one closed surface.

use crate::config::Measurements;

pub type V3 = [f64; 3];

pub struct Body {
    pub v: Vec<V3>,
    pub t: Vec<[usize; 3]>,
    /// Neck point (shoulder seam at the neck) height, mm from the floor.
    pub neck_y: f64,
    pub waist_y: f64,
    /// The body's front-to-back centre.
    pub z0: f64,
    /// Measured on the finished form, for the report: (waist, chest, seat).
    pub girths: (f64, f64, f64),
}

/// One level: height, half width, half depth, forward shift of its centre,
/// and how square its section is (2 = ellipse, higher = boxier).
#[derive(Clone, Copy)]
struct Level {
    y: f64,
    a: f64,
    b: f64,
    z: f64,
    n: f64,
    /// The back half's width over the front's: the shoulder blades.
    w: f64,
}

const SEGMENTS: usize = 160;

fn section(l: &Level) -> Vec<(f64, f64)> {
    (0..SEGMENTS)
        .map(|i| {
            let t = 2.0 * std::f64::consts::PI * i as f64 / SEGMENTS as f64;
            let (c, s) = (t.cos(), t.sin());
            let e = 2.0 / l.n;
            // the back widens gradually from the side seam, no step at the sides
            let a = l.a * (1.0 + (l.w - 1.0) * (-s).max(0.0));
            (a * c.signum() * c.abs().powf(e), l.z + l.b * s.signum() * s.abs().powf(e))
        })
        .collect()
}

fn perimeter(pts: &[(f64, f64)]) -> f64 {
    (0..pts.len()).map(|i| {
        let (p, q) = (pts[i], pts[(i + 1) % pts.len()]);
        ((p.0 - q.0).powi(2) + (p.1 - q.1).powi(2)).sqrt()
    }).sum()
}

/// A level with the given girth, keeping its width-to-depth ratio.
fn sized(y: f64, girth: f64, ratio: f64, z: f64, n: f64) -> Level {
    let unit = Level { y, a: 1.0, b: ratio, z: 0.0, n, w: 1.0 };
    let p = perimeter(&section(&unit));
    let a = girth / p;
    Level { y, a, b: a * ratio, z, n, w: 1.0 }
}

/// The same level with its back half `w` times as wide as its front.
fn back(l: Level, w: f64) -> Level {
    Level { w, ..l }
}

/// Catmull-Rom through the levels, sampled every `step` mm of height.
fn blend(keys: &[Level], step: f64) -> Vec<Level> {
    let mut out = Vec::new();
    for i in 0..keys.len() - 1 {
        let (p0, p1, p2, p3) = (keys[i.saturating_sub(1)], keys[i], keys[i + 1], keys[(i + 2).min(keys.len() - 1)]);
        let n = ((p2.y - p1.y) / step).ceil().max(1.0) as usize;
        for k in 0..n {
            let t = k as f64 / n as f64;
            let cr = |a: f64, b: f64, c: f64, d: f64| 0.5 * (2.0 * b + (-a + c) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t * t + (-a + 3.0 * b - 3.0 * c + d) * t * t * t);
            out.push(Level { y: p1.y + (p2.y - p1.y) * t, a: cr(p0.a, p1.a, p2.a, p3.a), b: cr(p0.b, p1.b, p2.b, p3.b), z: cr(p0.z, p1.z, p2.z, p3.z), n: cr(p0.n, p1.n, p2.n, p3.n), w: cr(p0.w, p1.w, p2.w, p3.w) });
        }
    }
    out.push(*keys.last().expect("levels"));
    out
}

/// The torso's girth at height y (a slice through the form).
pub fn girth(v: &[V3], t: &[[usize; 3]], y: f64) -> f64 {
    let mut pts: Vec<(f64, f64)> = Vec::new();
    for tri in t {
        for k in 0..3 {
            let (a, b) = (v[tri[k]], v[tri[(k + 1) % 3]]);
            if (a[1] - y) * (b[1] - y) < 0.0 {
                let s = (y - a[1]) / (b[1] - a[1]);
                pts.push((a[0] + (b[0] - a[0]) * s, a[2] + (b[2] - a[2]) * s));
            }
        }
    }
    // the form's own section, not the pole: points away from the centre line
    let pts: Vec<(f64, f64)> = pts.into_iter().filter(|p| p.0.abs() > 20.0 || p.1.abs() > 20.0).collect();
    if pts.len() < 3 {
        return 0.0;
    }
    hull_perimeter(&pts)
}

fn hull_perimeter(pts: &[(f64, f64)]) -> f64 {
    let mut p = pts.to_vec();
    p.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    let cross = |o: (f64, f64), a: (f64, f64), b: (f64, f64)| (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0);
    let mut h: Vec<(f64, f64)> = Vec::new();
    for pass in 0..2 {
        let start = h.len();
        let it: Box<dyn Iterator<Item = &(f64, f64)>> = if pass == 0 { Box::new(p.iter()) } else { Box::new(p.iter().rev()) };
        for &q in it {
            while h.len() >= start + 2 && cross(h[h.len() - 2], h[h.len() - 1], q) <= 0.0 {
                h.pop();
            }
            h.push(q);
        }
        h.pop();
    }
    (0..h.len()).map(|i| {
        let (a, b) = (h[i], h[(i + 1) % h.len()]);
        ((a.0 - b.0).powi(2) + (a.1 - b.1).powi(2)).sqrt()
    }).sum()
}

/// The bust: two round domes on the front at the bust points, about 36%
/// of the way from the neck point to the waist (some 17 cm down on a size
/// M) and a tenth of the chest either side of the centre front, a hand's
/// breadth of flat breastbone left between them.
struct Bust {
    y: f64,
    x: f64,
    amp: f64,
    up: f64,
    down: f64,
    side: f64,
}

impl Bust {
    /// How far the surface rises at (x, y), mm: one round dome (its
    /// radii above, below and across can differ; on the form they are equal).
    fn rise(&self, x: f64, y: f64) -> f64 {
        let dy = y - self.y;
        let ry = dy / if dy > 0.0 { self.up } else { self.down };
        let rx = (x.abs() - self.x) / self.side;
        let r2 = rx * rx + ry * ry;
        if r2 >= 1.0 {
            0.0
        } else {
            self.amp * (1.0 - r2).powf(1.1)
        }
    }
}

/// The torso: levels blended, then the bust raised on the front.
fn torso(keys: &[Level], bust: &Bust) -> (Vec<V3>, Vec<[usize; 3]>, Vec<Level>) {
    let levels = blend(keys, 8.0);
    let mut v: Vec<V3> = Vec::new();
    let mut t: Vec<[usize; 3]> = Vec::new();
    for l in &levels {
        for (x, z) in section(l) {
            let h = if z > l.z { bust.rise(x, l.y) } else { 0.0 };
            // outward from the section's centre, mostly forward
            let (dx, dz) = (x, (z - l.z) * 2.0);
            let n = (dx * dx + dz * dz).sqrt().max(1e-9);
            v.push([x + h * dx / n * 0.15, l.y, z + h * dz / n]);
        }
    }
    let rings = levels.len();
    for r in 0..rings - 1 {
        for i in 0..SEGMENTS {
            let (a, b) = (r * SEGMENTS + i, r * SEGMENTS + (i + 1) % SEGMENTS);
            let (c, d) = (a + SEGMENTS, b + SEGMENTS);
            // outward: around the ring (x to z) and up
            t.push([a, c, b]);
            t.push([b, c, d]);
        }
    }
    // caps
    for (ring, up) in [(0, false), (rings - 1, true)] {
        let l = levels[ring];
        let centre = v.len();
        v.push([0.0, l.y, l.z]);
        for i in 0..SEGMENTS {
            let (a, b) = (ring * SEGMENTS + i, ring * SEGMENTS + (i + 1) % SEGMENTS);
            t.push(if up { [centre, b, a] } else { [centre, a, b] });
        }
    }
    (v, t, levels)
}

/// The dress form for a size.
pub fn form(m: &Measurements) -> Body {
    let waist_y = m.inside_leg + m.body_rise;
    let seat_y = waist_y - m.hip_depth;
    let nape_y = waist_y + m.nape_to_waist;
    let neck_y = nape_y + 20.0;
    let bust_y = neck_y - 0.36 * (neck_y - waist_y);
    let underbust = (m.chest - 120.0).max(m.waist + 10.0);
    let shoulder_drop = 45.0;
    let neck_r = m.neck / (2.0 * std::f64::consts::PI);
    let shoulder_x = m.neck / 5.0 + m.shoulder;
    let bust = Bust { y: bust_y, x: m.chest * 0.1, amp: 38.0, up: 76.0, down: 76.0, side: 76.0 };
    let keys = |rib: f64| {
        vec![
            // the bottom, rounding in below the seat
            Level { y: seat_y - 230.0, a: 40.0, b: 35.0, z: -5.0, n: 2.0, w: 1.0 },
            sized(seat_y - 200.0, m.seat * 0.84, 0.70, -5.0, 2.4),
            sized(seat_y - 110.0, m.seat * 0.98, 0.64, -12.0, 2.6),
            sized(seat_y, m.seat, 0.62, -10.0, 2.6),
            sized(waist_y, m.waist, 0.74, 0.0, 2.3),
            back(sized(bust_y - 95.0, underbust, 0.66, 4.0, 2.4), 1.04),
            // the rib cage under the bust; the domes bring it up to the chest
            back(sized(bust_y, rib, 0.60, 6.0, 2.6), 1.08),
            // the upper chest, wide under the shoulders where the armholes sit
            {
                // the shoulder blades: across the back a little more than the size's back width
                // halfway between the bust and the shoulders, clear of both
                let l = sized(neck_y - 130.0, m.chest * 0.98, 0.56, 2.0, 2.8);
                back(l, ((m.back_width / 2.0 + 25.0) / l.a).max(1.0))
            },
            // across the shoulders: wide and shallow, then up the slope to the neck
            Level { y: neck_y - shoulder_drop - 20.0, a: shoulder_x * 1.02, b: 95.0, z: 0.0, n: 3.0, w: 1.04 },
            Level { y: neck_y - shoulder_drop * 0.45, a: (shoulder_x + m.neck / 5.0) * 0.55, b: 75.0, z: -5.0, n: 2.6, w: 1.0 },
            Level { y: neck_y, a: neck_r * 1.25, b: neck_r * 1.1, z: -10.0, n: 2.0, w: 1.0 },
            Level { y: neck_y + 70.0, a: neck_r, b: neck_r, z: -12.0, n: 2.0, w: 1.0 },
            Level { y: neck_y + 90.0, a: neck_r * 0.6, b: neck_r * 0.6, z: -12.0, n: 2.0, w: 1.0 },
        ]
    };
    // the rib girth that makes a tape over the bust points read the chest:
    // a few secant steps, the tape's reading grows steadily with the ribs
    let tape = |rib: f64| {
        let (v, t, _) = torso(&keys(rib), &bust);
        girth(&v, &t, bust_y + 0.5)
    };
    let (mut r0, mut r1) = (m.chest * 0.85, m.chest * 0.95);
    let (mut g0, mut g1) = (tape(r0), tape(r1));
    for _ in 0..6 {
        if (g1 - m.chest).abs() < 0.05 || (g1 - g0).abs() < 1e-9 {
            break;
        }
        let r2 = r1 + (m.chest - g1) * (r1 - r0) / (g1 - g0);
        (r0, g0, r1) = (r1, g1, r2);
        g1 = tape(r1);
    }
    let (mut v, mut t, levels) = torso(&keys(r1), &bust);
    // the stand: a pole down to the floor and a round foot
    let pole_top = levels[0].y;
    let tube = |r: f64, y0: f64, y1: f64, v: &mut Vec<V3>, t: &mut Vec<[usize; 3]>| {
        let base = v.len();
        let n = 24;
        for y in [y0, y1] {
            for i in 0..n {
                let a = 2.0 * std::f64::consts::PI * i as f64 / n as f64;
                v.push([r * a.cos(), y, r * a.sin()]);
            }
        }
        for i in 0..n {
            let (a, b) = (base + i, base + (i + 1) % n);
            t.push([a, a + n, b]);
            t.push([b, a + n, b + n]);
        }
        for (ring, up) in [(0, false), (1, true)] {
            let c = v.len();
            v.push([0.0, if up { y1 } else { y0 }, 0.0]);
            for i in 0..n {
                let (a, b) = (base + ring * n + i, base + ring * n + (i + 1) % n);
                t.push(if up { [c, b, a] } else { [c, a, b] });
            }
        }
    };
    tube(14.0, 20.0, pole_top + 10.0, &mut v, &mut t);
    tube(170.0, 0.0, 20.0, &mut v, &mut t);
    // measured a hair off the levels, which are rings of the mesh
    let girths = (girth(&v, &t, waist_y + 0.5), girth(&v, &t, bust_y + 0.5), girth(&v, &t, seat_y + 0.5));
    Body { v, t, neck_y, waist_y, z0: 0.0, girths }
}
