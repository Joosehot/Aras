//! The top block: back and front halves drafted from the body measurements,
//! and a one-piece sleeve whose cap is solved to fit the armhole.
//!
//! Frame: x runs from centre front/back outward, y down from the neck point
//! level. The half body is cut on the fold (or with a button stand for shirts).

use crate::draft::Draft;
use crate::geom::{self, cubic, pt, Pt};
use crate::pattern::{Cut, Edge, EdgeKind, Mark, Piece};

/// Control-point factor that makes a cubic Bézier a quarter ellipse.
pub const KAPPA: f64 = 0.5523;

pub fn draft(d: &mut Draft) -> Result<(), String> {
    let step = d.cfg.draft("armhole_step");
    let tries = d.cfg.draft("armhole_tries") as usize;
    let start = d.top.armhole_drop;
    for _ in 0..=tries {
        let back = half(d, false)?;
        let front = half(d, true)?;
        match sleeve(d, &back, &front) {
            Ok(sleeve) => {
                if d.top.armhole_drop > start {
                    d.notes.push(format!(
                        "armhole deepened {:.1} cm so the {:.1} cm sleeve fits it",
                        (d.top.armhole_drop - start) / 10.0,
                        d.top.bicep / 10.0
                    ));
                }
                finish(d, back, front, sleeve);
                return Ok(());
            }
            Err(SleeveErr::TooWide) => d.top.armhole_drop += step,
            Err(SleeveErr::Other(e)) => return Err(e),
        }
    }
    Err(format!("a {:.1} cm sleeve doesn't fit the armhole even after deepening it", d.top.bicep / 10.0))
}

fn finish(d: &mut Draft, back: Piece, front: Piece, sleeve: Piece) {
    let (ab, af) = (back.len("armhole"), front.len("armhole"));
    let cap = sleeve.len("cap_back") + sleeve.len("cap_front");
    let (lo, hi) = (d.fabric.cap_ease_min, d.fabric.cap_ease_max);
    d.seam("shoulder: front / back", front.len("shoulder"), back.len("shoulder"), 0.0, 0.0);
    d.seam("side seam: front / back", front.len("side"), back.len("side"), 0.0, 0.0);
    d.seam("sleeve cap / armhole (cap ease)", ab + af, cap, lo, hi);
    d.seam("underarm seam: front / back", sleeve.len("underarm_front"), sleeve.len("underarm_back"), 0.0, 0.0);
    let min = d.cfg.check("min_shoulder");
    let sl = d.top.shoulder_len;
    d.check("shoulder length", sl >= min, format!("{:.1} cm (at least {:.1} cm)", sl / 10.0, min / 10.0));
    let (spec, words) = (d.spec.g().name(), d.spec.garment.words.clone());
    for p in [back, front, sleeve] {
        d.add(p.by(&format!("block/{spec}"), &words));
    }
}

/// Shoulder-to-underarm armhole as two Béziers, and the across-back/front point.
fn armhole(sp: Pt, hps: Pt, b: Pt, c: Pt) -> (Vec<Pt>, Vec<Pt>) {
    let dir = (sp - hps).unit();
    let n = pt(-dir.y, dir.x);
    let k1 = sp.dist(b) / 3.0;
    let upper = cubic(sp, sp + n * k1, b - pt(0.0, k1), b);
    let k2 = (c.y - b.y) * 0.5;
    let lower = cubic(b, b + pt(0.0, k2), c - pt((c.x - b.x) * 0.6, 0.0), c);
    (upper, lower)
}

fn half(d: &Draft, front: bool) -> Result<Piece, String> {
    let (t, m, cfg) = (&d.top, d.m, d.cfg);
    let q = (m.chest + t.chest_ease) / 4.0;
    if t.shoulder_len <= t.shoulder_drop {
        return Err(format!("the shoulder seam ({:.1} cm) is shorter than its slope", t.shoulder_len / 10.0));
    }
    let hps = pt(t.nw, 0.0);
    let sp = pt(t.nw + (t.shoulder_len.powi(2) - t.shoulder_drop.powi(2)).sqrt(), t.shoulder_drop);
    let chest_y = t.back_neck_depth + m.scye_depth + t.armhole_drop;
    let bw = m.back_width / 2.0 + t.chest_ease * cfg.draft("back_width_ease");
    let across = if front { bw - cfg.draft("front_width_less") } else { bw };
    let across = across.min(sp.x - 5.0).min(q - 15.0);
    let b = pt(across, sp.y + (chest_y - sp.y) * cfg.draft("width_point"));
    let c = pt(q, chest_y);
    let side_len = t.length - t.shirttail;
    if side_len < chest_y + 50.0 {
        return Err(format!("the body ({:.1} cm) is too short to reach below the armhole", t.length / 10.0));
    }
    let depth = if front { t.front_neck_depth } else { t.back_neck_depth };
    if depth <= 0.0 {
        return Err("the neckline is raised above the neck point".into());
    }
    let ext = if front { t.button_ext } else { 0.0 };
    let center = if ext > 0.0 { -ext } else { 0.0 };

    // Front: a quarter ellipse. Back: shallow, so it rises to the neck point
    // at an angle instead of turning sharply there.
    let curve = if front {
        cubic(pt(0.0, depth), pt(KAPPA * t.nw, depth), pt(t.nw, KAPPA * depth), hps)
    } else {
        cubic(pt(0.0, depth), pt(0.5 * t.nw, depth), pt(0.8 * t.nw, 0.8 * depth), hps)
    };
    let neck = if ext > 0.0 { geom::chain(&[vec![pt(-ext, depth)], curve]) } else { curve };
    let (upper, lower) = armhole(sp, hps, b, c);
    let notch_at = geom::length(&upper);
    let h = pt(q + t.hem_flare, side_len);
    let hem = if t.shirttail > 0.0 {
        let curve = cubic(h, pt(h.x, h.y + t.shirttail * 0.6), pt(h.x * 0.55, t.length), pt(0.0, t.length));
        geom::chain(&[curve, vec![pt(center, t.length)]])
    } else {
        vec![h, pt(center, t.length)]
    };
    let mut edges = vec![
        Edge::new("neck", EdgeKind::Seam, neck),
        Edge::new("shoulder", EdgeKind::Seam, vec![hps, sp]),
        Edge::new("armhole", EdgeKind::Seam, geom::chain(&[upper, lower])),
        Edge::new("side", EdgeKind::Seam, vec![c, h]),
        Edge::new("hem", EdgeKind::Hem, hem),
    ];
    let name = if front { "front" } else { "back" };
    let cut = if ext > 0.0 {
        edges.push(Edge::new("cf", EdgeKind::Hem, vec![pt(-ext, t.length), pt(-ext, depth)]).sa(cfg.part("placket_facing")));
        Cut::PAIR
    } else {
        edges.push(Edge::new(if front { "cf" } else { "cb" }, EdgeKind::Fold, vec![pt(0.0, t.length), pt(0.0, depth)]));
        Cut::FOLD
    };
    let mut p = Piece::new(name, cut, edges).anchored(pt(0.0, chest_y)).mark(Mark::Notch { edge: "armhole", at: notch_at, count: if front { 1 } else { 2 } });
    if ext > 0.0 {
        p = p.mark(Mark::Line { pts: vec![pt(0.0, depth), pt(0.0, t.length)], dashed: true });
        let (top, bottom) = (depth + 15.0, t.length - 100.0);
        let n = ((bottom - top) / 90.0).floor().max(0.0) as usize + 1;
        for i in 0..n {
            let y = top + (bottom - top) * i as f64 / (n.max(2) - 1) as f64;
            p = p.mark(Mark::Line { pts: vec![pt(0.0, y), pt(0.0, y + 15.0)], dashed: false });
        }
    }
    Ok(p)
}

enum SleeveErr {
    /// Even the flattest cap is longer than the armhole.
    TooWide,
    Other(String),
}

/// Back and front halves of the cap, top to underarm, for cap height `h`.
fn cap(hw: f64, h: f64) -> (Vec<Pt>, Vec<Pt>) {
    let back = cubic(pt(0.0, 0.0), pt(-0.55 * hw, 0.0), pt(-0.45 * hw, h), pt(-hw, h));
    let front = cubic(pt(0.0, 0.0), pt(0.45 * hw, 0.0), pt(0.6 * hw, h), pt(hw, h));
    (back, front)
}

fn notch_at(p: &Piece) -> f64 {
    p.marks
        .iter()
        .find_map(|m| match m {
            Mark::Notch { edge: "armhole", at, .. } => Some(*at),
            _ => None,
        })
        .expect("armhole notch")
}

fn sleeve(d: &Draft, back: &Piece, front: &Piece) -> Result<Piece, SleeveErr> {
    let t = &d.top;
    let hw = t.bicep / 2.0;
    let (ab, af) = (back.len("armhole"), front.len("armhole"));
    let target = ab + af + (d.fabric.cap_ease_min + d.fabric.cap_ease_max) / 2.0;
    let f = |h: f64| {
        let (b, fr) = cap(hw, h);
        geom::length(&b) + geom::length(&fr) - target
    };
    let (lo, hi) = (d.cfg.draft("cap_min"), d.cfg.draft("cap_max"));
    if f(lo) > 0.0 {
        return Err(SleeveErr::TooWide);
    }
    let h = geom::solve(lo, hi, f).ok_or_else(|| {
        SleeveErr::Other(format!("a {:.1} cm sleeve is too narrow for the armhole: widen the sleeves", t.bicep / 10.0))
    })?;
    if t.sleeve_len < h + 20.0 {
        return Err(SleeveErr::Other(format!("the sleeve ({:.1} cm) is shorter than its cap", t.sleeve_len / 10.0)));
    }
    let (ub, uf) = (pt(-hw, h), pt(hw, h));
    let mut hb = pt(-t.sleeve_hem / 2.0, t.sleeve_len);
    let mut hf = pt(t.sleeve_hem / 2.0, t.sleeve_len);
    let e = t.sleeve_extend;
    if e.abs() > d.cfg.check("max_extend") {
        return Err(SleeveErr::Other(format!("{:.1} cm is more than a sleeve can be extended", e / 10.0)));
    }
    if e != 0.0 {
        let k = e / (t.sleeve_len - h);
        hb = hb + (hb - ub) * k;
        hf = hf + (hf - uf) * k;
    }
    if hf.x < 10.0 {
        return Err(SleeveErr::Other("continuing the sleeve's taper closes it before the hem".into()));
    }
    let (cb, cf) = cap(hw, h);
    let mut cap_back = cb.clone();
    cap_back.reverse();
    let cap_front_len = geom::length(&cf);
    let p = Piece::new(
        "sleeve",
        Cut::PAIR,
        vec![
            Edge::new("cap_back", EdgeKind::Seam, cap_back),
            Edge::new("cap_front", EdgeKind::Seam, cf),
            Edge::new("underarm_front", EdgeKind::Seam, vec![uf, hf]),
            Edge::new("hem", EdgeKind::Hem, vec![hf, hb]),
            Edge::new("underarm_back", EdgeKind::Seam, vec![hb, ub]),
        ],
    )
    .mark(Mark::Notch { edge: "cap_back", at: ab - notch_at(back), count: 2 })
    .mark(Mark::Notch { edge: "cap_front", at: cap_front_len - (af - notch_at(front)), count: 1 })
    .mark(Mark::Notch { edge: "cap_front", at: 0.0, count: 1 })
    .anchored(pt(0.0, h));
    Ok(p)
}
