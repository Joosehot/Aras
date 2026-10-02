//! The cloth: panels placed around the body, sewn at their seams, and let
//! fall under gravity with position-based dynamics (Muller et al. 2007):
//! every step the particles move freely, then constraints are projected
//! over a few iterations: stretch along every triangle edge, bending
//! across every pair of triangles, the seams (pulled shut over the first
//! steps, as a dressmaker would pin then sew) and the body (no particle
//! closer to it than the cloth's thickness). Fixed order, no threads, no
//! randomness: the same garment always settles the same way.

use super::mannequin::{Body, V3};
use super::panel::Panel;
use super::sdf::Sdf;
use std::collections::HashMap;

pub struct Params {
    pub gap: f64,
    pub steps: usize,
    pub iterations: usize,
    pub dt: f64,
    pub damping: f64,
    pub stretch: f64,
    pub bend: f64,
    pub seam: f64,
    pub ramp: usize,
    pub thickness: f64,
    pub friction: f64,
}

/// A panel placed on the body: which piece, and where its first particle is in the cloth.
pub struct Placed {
    pub name: String,
    pub first: usize,
    pub count: usize,
}

pub struct Cloth {
    pub pos: Vec<V3>,
    prev: Vec<V3>,
    pub tri: Vec<[usize; 3]>,
    /// (a, b, rest length) for stretch, then bending
    stretch: Vec<(usize, usize, f64)>,
    bend: Vec<(usize, usize, f64)>,
    pub seams: Vec<(usize, usize)>,
    pub placed: Vec<Placed>,
    /// stitches per seam, for the report
    pub sewn: Vec<(String, usize)>,
    /// the shoulder seams pinned at the back shoulder's distance from the
    /// centre, as a dressmaker pins a dress to the form: free to settle up,
    /// down, forward and back, never to slide off the shoulder; a cowl's
    /// wider front shoulder is gathered in and its top drapes: (particle, x)
    pub pins: Vec<(usize, f64)>,
}

pub struct Report {
    pub particles: usize,
    pub triangles: usize,
    pub seam_pairs: usize,
    /// mean and largest seam gap after settling, mm
    pub seam_gap: (f64, f64),
    /// largest stretch of a triangle edge, as a share of its rest length
    pub max_stretch: f64,
    /// share of particles resting on the body
    pub contact: f64,
    /// deepest a particle sits inside the body, mm (0 = none)
    pub penetration: f64,
    pub settle: f64,
}

/// Which pattern edges are sewn together: (panel, edge) <-> (panel, edge),
/// and whether the second runs the other way.
const SEAMS: &[(&str, &str, &str, &str, bool)] = &[
    ("front", "shoulder", "back", "shoulder", false),
    ("front", "side", "back", "side", false),
    ("skirt front", "side", "skirt back", "side", false),
    ("front", "waist", "skirt front", "waist", true),
    ("back", "waist", "skirt back", "waist", true),
];

/// A panel hung flat in front of (or behind) the form, `gap` clear of it at
/// every height, as a dressmaker holds it up before pinning the seams; the
/// seams then pull it round the sides and over the shoulders.
fn place(panel: &Panel, body: &Body, front: bool, y_off: f64, gap: f64, depth_at: &dyn Fn(f64) -> f64) -> Vec<V3> {
    panel
        .p
        .iter()
        .map(|q| {
            let y3 = body.neck_y - (q.y + y_off);
            let z = depth_at(y3) + gap;
            [q.x, y3, if front { body.z0 + z } else { body.z0 - z }]
        })
        .collect()
}

fn dist(a: V3, b: V3) -> f64 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

impl Cloth {
    /// Panels on the body. `y_off` per panel moves a skirt down to the waist seam.
    pub fn new(panels: &[(Panel, f64)], body: &Body, p: &Params, depth_at: &dyn Fn(f64) -> f64) -> Cloth {
        let mut c = Cloth { pos: Vec::new(), prev: Vec::new(), tri: Vec::new(), stretch: Vec::new(), bend: Vec::new(), seams: Vec::new(), placed: Vec::new(), sewn: Vec::new(), pins: Vec::new() };
        for (panel, y_off) in panels {
            let front = !panel.name.contains("back");
            let first = c.pos.len();
            let pos3 = place(panel, body, front, *y_off, p.gap, depth_at);
            c.pos.extend(pos3.iter().copied());
            // stretch: rest lengths from the flat pattern
            let mut edges: HashMap<(usize, usize), Vec<usize>> = HashMap::new();
            for t in &panel.tri {
                // keep the outward winding consistent for front and back
                let t2 = if front { *t } else { [t[0], t[2], t[1]] };
                c.tri.push([t2[0] + first, t2[1] + first, t2[2] + first]);
                for k in 0..3 {
                    let (a, b) = (t[k].min(t[(k + 1) % 3]), t[k].max(t[(k + 1) % 3]));
                    edges.entry((a, b)).or_default().push(t[(k + 2) % 3]);
                }
            }
            let mut keys: Vec<_> = edges.keys().copied().collect();
            keys.sort();
            for (a, b) in keys {
                c.stretch.push((a + first, b + first, panel.p[a].dist(panel.p[b])));
                // bending: the two vertices across a shared edge
                if let [x, y] = edges[&(a, b)][..] {
                    c.bend.push((x + first, y + first, panel.p[x].dist(panel.p[y])));
                }
            }
            c.placed.push(Placed { name: panel.name.clone(), first, count: panel.p.len() });
        }
        c.prev = c.pos.clone();
        // the back shoulder, from the neck out: (t, |x|), per side
        if let Some(bi) = panels.iter().position(|(x, _)| x.name == "back") {
            let back = &panels[bi].0;
            let line: Vec<(f64, f64)> = back.on.iter().zip(&back.p).filter_map(|(o, q)| o.as_ref().filter(|o| o.edge == "shoulder" && o.side == 1).map(|o| (o.t, q.x.abs()))).collect();
            if !line.is_empty() {
                for (i, (panel, _)) in panels.iter().enumerate() {
                    for (k, o) in panel.on.iter().enumerate() {
                        let Some(o) = o.as_ref().filter(|o| o.edge == "shoulder") else { continue };
                        let x = line.iter().min_by(|a, b| (a.0 - o.t).abs().total_cmp(&(b.0 - o.t).abs())).expect("line").1;
                        c.pins.push((c.placed[i].first + k, x * panel.p[k].x.signum()));
                    }
                }
            }
        }
        // seams: each point of one edge to the point of the other nearest along it
        for &(pa, ea, pb, eb, flip) in SEAMS {
            let (Some(ia), Some(ib)) = (panels.iter().position(|(x, _)| x.name == pa), panels.iter().position(|(x, _)| x.name == pb)) else { continue };
            let before = c.seams.len();
            for side in [1i8, -1] {
                let pick = |i: usize, e: &str| -> Vec<(usize, f64)> {
                    let (panel, first) = (&panels[i].0, c.placed[i].first);
                    panel.on.iter().enumerate().filter_map(|(k, o)| o.as_ref().filter(|o| o.edge == e && o.side == side).map(|o| (k + first, o.t))).collect()
                };
                let (a, mut b) = (pick(ia, ea), pick(ib, eb));
                if flip {
                    for x in b.iter_mut() {
                        x.1 = 1.0 - x.1;
                    }
                }
                if a.is_empty() || b.is_empty() {
                    continue;
                }
                let near = |from: &[(usize, f64)], t: f64| from.iter().min_by(|x, y| (x.1 - t).abs().total_cmp(&(y.1 - t).abs())).expect("points").0;
                for &(i, t) in &a {
                    c.seams.push((i, near(&b, t)));
                }
                for &(j, t) in &b {
                    let i = near(&a, t);
                    if !c.seams.contains(&(i, j)) {
                        c.seams.push((i, j));
                    }
                }
            }
            c.sewn.push((format!("{pa} {ea}"), c.seams.len() - before));
        }
        c
    }

    fn pull(&mut self, a: usize, b: usize, rest: f64, k: f64) {
        let (pa, pb) = (self.pos[a], self.pos[b]);
        let d = dist(pa, pb);
        if d < 1e-9 {
            return;
        }
        let s = k * 0.5 * (d - rest) / d;
        for i in 0..3 {
            let dd = (pb[i] - pa[i]) * s;
            self.pos[a][i] += dd;
            self.pos[b][i] -= dd;
        }
    }

    pub fn run(&mut self, sdf: &Sdf, p: &Params) -> Report {
        let g = -9810.0 * p.dt * p.dt;
        let n = self.pos.len();
        let mut last_move = 0.0;
        let mut contact = vec![false; n];
        for step in 0..p.steps {
            let ramp = ((step + 1) as f64 / p.ramp as f64).min(1.0);
            // pinned first, let go after: no gravity until the seams are shut
            let g = if step < p.ramp { 0.0 } else { g };
            let mut moved = 0.0;
            for i in 0..n {
                let v: [f64; 3] = std::array::from_fn(|k| (self.pos[i][k] - self.prev[i][k]) * p.damping);
                self.prev[i] = self.pos[i];
                self.pos[i] = [self.pos[i][0] + v[0], self.pos[i][1] + v[1] + g, self.pos[i][2] + v[2]];
                moved += (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            }
            last_move = moved / n as f64;
            for _ in 0..p.iterations {
                for k in 0..self.stretch.len() {
                    let (a, b, r) = self.stretch[k];
                    self.pull(a, b, r, p.stretch);
                }
                for k in 0..self.bend.len() {
                    let (a, b, r) = self.bend[k];
                    self.pull(a, b, r, p.bend);
                }
                for k in 0..self.seams.len() {
                    let (a, b) = self.seams[k];
                    self.pull(a, b, 0.0, p.seam * ramp);
                }
                for &(i, x) in &self.pins {
                    self.pos[i][0] = x;
                }
                for i in 0..n {
                    let (d, nrm) = sdf.sample(self.pos[i]);
                    contact[i] = d < p.thickness;
                    if contact[i] {
                        for k in 0..3 {
                            self.pos[i][k] += nrm[k] * (p.thickness - d);
                        }
                    }
                }
            }
            // friction: a particle on the body keeps only part of its slide
            for i in 0..n {
                if contact[i] {
                    for k in 0..3 {
                        self.prev[i][k] = self.pos[i][k] - (self.pos[i][k] - self.prev[i][k]) * (1.0 - p.friction);
                    }
                }
            }
        }
        let gaps: Vec<f64> = self.seams.iter().map(|&(a, b)| dist(self.pos[a], self.pos[b])).collect();
        let max_stretch = self.stretch.iter().map(|&(a, b, r)| dist(self.pos[a], self.pos[b]) / r.max(1e-9) - 1.0).fold(0.0, f64::max);
        let penetration = self.pos.iter().map(|q| -sdf.sample(*q).0).fold(0.0, f64::max);
        Report {
            particles: n,
            triangles: self.tri.len(),
            seam_pairs: self.seams.len(),
            seam_gap: (gaps.iter().sum::<f64>() / gaps.len().max(1) as f64, gaps.iter().copied().fold(0.0, f64::max)),
            max_stretch,
            contact: contact.iter().filter(|c| **c).count() as f64 / n as f64,
            penetration,
            settle: last_move,
        }
    }
}
