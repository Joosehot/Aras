//! The body as a signed distance field on a grid: for each cell the
//! distance to the nearest point of the body mesh, negative inside. Inside
//! is decided by rays, not by the nearest triangle's normal (which flips
//! wrongly near edges and corners): a point is inside when rays from it
//! cross the closed surface an odd number of times, two rays of three
//! agreeing. Built once; the cloth reads it with
//! trilinear interpolation every step.

use super::mannequin::V3;

fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}
fn add(a: V3, b: V3, s: f64) -> V3 {
    [a[0] + b[0] * s, a[1] + b[1] * s, a[2] + b[2] * s]
}

/// Closest point on triangle abc to p (Ericson, Real-Time Collision Detection 5.1.5).
fn closest(p: V3, a: V3, b: V3, c: V3) -> V3 {
    let (ab, ac, ap) = (sub(b, a), sub(c, a), sub(p, a));
    let (d1, d2) = (dot(ab, ap), dot(ac, ap));
    if d1 <= 0.0 && d2 <= 0.0 {
        return a;
    }
    let bp = sub(p, b);
    let (d3, d4) = (dot(ab, bp), dot(ac, bp));
    if d3 >= 0.0 && d4 <= d3 {
        return b;
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        return add(a, ab, d1 / (d1 - d3));
    }
    let cp = sub(p, c);
    let (d5, d6) = (dot(ab, cp), dot(ac, cp));
    if d6 >= 0.0 && d5 <= d6 {
        return c;
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        return add(a, ac, d2 / (d2 - d6));
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        return add(b, sub(c, b), (d4 - d3) / ((d4 - d3) + (d5 - d6)));
    }
    let den = 1.0 / (va + vb + vc);
    add(add(a, ab, vb * den), ac, vc * den)
}

struct Node {
    lo: V3,
    hi: V3,
    a: usize,
    b: usize,
    leaf: bool,
}

struct Bvh<'a> {
    v: &'a [V3],
    t: &'a [[usize; 3]],
    nodes: Vec<Node>,
    order: Vec<usize>,
}

impl<'a> Bvh<'a> {
    fn new(v: &'a [V3], t: &'a [[usize; 3]]) -> Bvh<'a> {
        let mut b = Bvh { v, t, nodes: Vec::new(), order: (0..t.len()).collect() };
        b.build(0, t.len());
        b
    }
    fn tbox(&self, i: usize) -> (V3, V3) {
        let [a, b, c] = self.t[i];
        let (p, q, r) = (self.v[a], self.v[b], self.v[c]);
        (std::array::from_fn(|k| p[k].min(q[k]).min(r[k])), std::array::from_fn(|k| p[k].max(q[k]).max(r[k])))
    }
    fn build(&mut self, s: usize, n: usize) -> usize {
        let (mut lo, mut hi) = ([f64::INFINITY; 3], [f64::NEG_INFINITY; 3]);
        for &i in &self.order[s..s + n] {
            let (a, b) = self.tbox(i);
            for k in 0..3 {
                lo[k] = lo[k].min(a[k]);
                hi[k] = hi[k].max(b[k]);
            }
        }
        let id = self.nodes.len();
        self.nodes.push(Node { lo, hi, a: s, b: n, leaf: true });
        if n <= 6 {
            return id;
        }
        let axis = (0..3).max_by(|&x, &y| (hi[x] - lo[x]).total_cmp(&(hi[y] - lo[y]))).expect("axis");
        let (v, t) = (self.v, self.t);
        let c = |i: usize| v[t[i][0]][axis] + v[t[i][1]][axis] + v[t[i][2]][axis];
        self.order[s..s + n].sort_by(|&x, &y| c(x).total_cmp(&c(y)));
        let l = self.build(s, n / 2);
        let r = self.build(s + n / 2, n - n / 2);
        self.nodes[id] = Node { lo, hi, a: l, b: r, leaf: false };
        id
    }
    /// Signed distance from p to the mesh (negative behind the nearest face).
    fn signed(&self, p: V3) -> f64 {
        let mut best = (f64::INFINITY, 0usize, [0.0; 3]);
        let mut stack = vec![0usize];
        while let Some(i) = stack.pop() {
            let n = &self.nodes[i];
            let d2: f64 = (0..3).map(|k| (n.lo[k] - p[k]).max(0.0).max(p[k] - n.hi[k]).powi(2)).sum();
            if d2 >= best.0 {
                continue;
            }
            if n.leaf {
                for &ti in &self.order[n.a..n.a + n.b] {
                    let [a, b, c] = self.t[ti];
                    let q = closest(p, self.v[a], self.v[b], self.v[c]);
                    let d = sub(p, q);
                    let dd = dot(d, d);
                    if dd < best.0 {
                        best = (dd, ti, q);
                    }
                }
            } else {
                stack.push(n.a);
                stack.push(n.b);
            }
        }
        let d = best.0.sqrt();
        // three skew directions, so no ray runs along a ring of the mesh
        const DIRS: [V3; 3] = [[0.5773, 0.0123, 0.8165], [-0.7071, 0.0311, 0.7064], [0.0213, 0.9994, -0.0271]];
        let votes = DIRS.iter().filter(|dir| self.crossings(p, **dir) % 2 == 1).count();
        if votes >= 2 {
            -d
        } else {
            d
        }
    }
    /// How many triangles the ray from p along dir passes through.
    fn crossings(&self, p: V3, dir: V3) -> usize {
        let inv: V3 = std::array::from_fn(|k| 1.0 / dir[k]);
        let mut count = 0;
        let mut stack = vec![0usize];
        while let Some(i) = stack.pop() {
            let n = &self.nodes[i];
            // slab test: does the ray meet this box at all
            let (mut t0, mut t1) = (0.0f64, f64::INFINITY);
            for k in 0..3 {
                let (a, b) = ((n.lo[k] - p[k]) * inv[k], (n.hi[k] - p[k]) * inv[k]);
                t0 = t0.max(a.min(b));
                t1 = t1.min(a.max(b));
            }
            if t0 > t1 {
                continue;
            }
            if n.leaf {
                for &ti in &self.order[n.a..n.a + n.b] {
                    let [a, b, c] = self.t[ti];
                    // Moller-Trumbore
                    let (e1, e2) = (sub(self.v[b], self.v[a]), sub(self.v[c], self.v[a]));
                    let h = cross(dir, e2);
                    let det = dot(e1, h);
                    if det.abs() < 1e-12 {
                        continue;
                    }
                    let s = sub(p, self.v[a]);
                    let u = dot(s, h) / det;
                    if !(0.0..=1.0).contains(&u) {
                        continue;
                    }
                    let q = cross(s, e1);
                    let w = dot(dir, q) / det;
                    if w < 0.0 || u + w > 1.0 {
                        continue;
                    }
                    if dot(e2, q) / det > 0.0 {
                        count += 1;
                    }
                }
            } else {
                stack.push(n.a);
                stack.push(n.b);
            }
        }
        count
    }
}

pub struct Sdf {
    lo: V3,
    cell: f64,
    n: [usize; 3],
    d: Vec<f32>,
}

impl Sdf {
    /// The field over the box [lo, hi], `cell` mm apart.
    pub fn build(v: &[V3], t: &[[usize; 3]], lo: V3, hi: V3, cell: f64) -> Sdf {
        let bvh = Bvh::new(v, t);
        let n: [usize; 3] = std::array::from_fn(|k| ((hi[k] - lo[k]) / cell).ceil() as usize + 1);
        let total = n[0] * n[1] * n[2];
        let threads = std::thread::available_parallelism().map_or(4, |x| x.get());
        let chunk = total.div_ceil(threads);
        let mut d = vec![0f32; total];
        std::thread::scope(|s| {
            for (ci, part) in d.chunks_mut(chunk).enumerate() {
                let bvh = &bvh;
                s.spawn(move || {
                    for (j, out) in part.iter_mut().enumerate() {
                        let idx = ci * chunk + j;
                        let (x, rest) = (idx % n[0], idx / n[0]);
                        let (y, z) = (rest % n[1], rest / n[1]);
                        let p = [lo[0] + x as f64 * cell, lo[1] + y as f64 * cell, lo[2] + z as f64 * cell];
                        *out = bvh.signed(p) as f32;
                    }
                });
            }
        });
        Sdf { lo, cell, n, d }
    }
    fn at(&self, x: usize, y: usize, z: usize) -> f64 {
        self.d[x + self.n[0] * (y + self.n[1] * z)] as f64
    }
    /// Distance and outward gradient at p (far away when outside the grid).
    pub fn sample(&self, p: V3) -> (f64, V3) {
        let g: [f64; 3] = std::array::from_fn(|k| (p[k] - self.lo[k]) / self.cell);
        if (0..3).any(|k| g[k] < 0.0 || g[k] >= (self.n[k] - 1) as f64) {
            return (1e9, [0.0, 1.0, 0.0]);
        }
        let i: [usize; 3] = std::array::from_fn(|k| g[k] as usize);
        let f: [f64; 3] = std::array::from_fn(|k| g[k] - i[k] as f64);
        let c = |dx: usize, dy: usize, dz: usize| self.at(i[0] + dx, i[1] + dy, i[2] + dz);
        let lerp = |a: f64, b: f64, t: f64| a + (b - a) * t;
        let d = lerp(
            lerp(lerp(c(0, 0, 0), c(1, 0, 0), f[0]), lerp(c(0, 1, 0), c(1, 1, 0), f[0]), f[1]),
            lerp(lerp(c(0, 0, 1), c(1, 0, 1), f[0]), lerp(c(0, 1, 1), c(1, 1, 1), f[0]), f[1]),
            f[2],
        );
        // gradient by differences across the cell
        let gx = lerp(lerp(c(1, 0, 0) - c(0, 0, 0), c(1, 1, 0) - c(0, 1, 0), f[1]), lerp(c(1, 0, 1) - c(0, 0, 1), c(1, 1, 1) - c(0, 1, 1), f[1]), f[2]);
        let gy = lerp(lerp(c(0, 1, 0) - c(0, 0, 0), c(1, 1, 0) - c(1, 0, 0), f[0]), lerp(c(0, 1, 1) - c(0, 0, 1), c(1, 1, 1) - c(1, 0, 1), f[0]), f[2]);
        let gz = lerp(lerp(c(0, 0, 1) - c(0, 0, 0), c(1, 0, 1) - c(1, 0, 0), f[0]), lerp(c(0, 1, 1) - c(0, 1, 0), c(1, 1, 1) - c(1, 1, 0), f[0]), f[1]);
        let l = (gx * gx + gy * gy + gz * gz).sqrt().max(1e-9);
        (d, [gx / l, gy / l, gz / l])
    }
}
