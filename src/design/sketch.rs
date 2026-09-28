//! Technical flats: front and back views of the finished garment, built from
//! the drafted pieces, not drawn freehand. The body is the real front/back
//! outline unfolded; sleeves hang from the real shoulder point with their
//! real length, bicep and hem widths; bands, cuffs, the hood and the pocket
//! use their drafted sizes. Pants are drawn from the legs' girth at waist,
//! seat, knee and hem. Everything is filled with the resolved colourway.

use super::{Fill, Look, Painter, Zone};
use crate::config::Config;
use crate::geom::{self, cubic, pt, Pt, Rect};
use crate::model::{Garment, Spec};
use crate::pattern::{Mark, Pattern, Piece};
use crate::search::Outcome;
use crate::svg::{f, path};
use std::fmt::Write as _;

pub struct Drawing {
    pub svg: String,
    pub bbox: Rect,
}

struct Pen<'a> {
    out: String,
    pts: Vec<Pt>,
    paint: &'a mut Painter,
    thread: String,
}

impl Pen<'_> {
    fn shape(&mut self, pts: &[Pt], fill: &str) {
        self.pts.extend_from_slice(pts);
        let _ = writeln!(self.out, "<path class=\"fl\" fill=\"{fill}\" d=\"{}\"/>", path(pts, true));
    }
    fn line(&mut self, pts: &[Pt], class: &str) {
        let _ = writeln!(self.out, "<path class=\"{class}\" fill=\"none\" d=\"{}\"/>", path(pts, false));
    }
    fn stitch(&mut self, pts: &[Pt]) {
        let _ = writeln!(self.out, "<path class=\"fs\" stroke=\"{}\" d=\"{}\"/>", self.thread, path(pts, false));
    }
    fn fill(&mut self, fill: &Fill, anchor: Pt, angle: f64) -> String {
        self.paint.paint(fill, anchor, angle)
    }
    /// Rib: vertical wale lines across a band.
    fn rib(&mut self, x0: f64, x1: f64, y0: f64, y1: f64, color: &str) {
        let mut x = x0 + 4.0;
        let mut d = String::new();
        while x < x1 - 2.0 {
            let _ = write!(d, "M{} {} L{} {} ", f(x), f(y0 + 1.5), f(x), f(y1 - 1.5));
            x += 5.0;
        }
        let _ = writeln!(self.out, "<path class=\"rb\" stroke=\"{color}\" d=\"{}\"/>", d.trim_end());
    }
}

fn mirror(pts: &[Pt]) -> Vec<Pt> {
    pts.iter().map(|p| p.mirror_x()).collect()
}

/// Mirror a half outline that starts and ends on x = 0 into a whole one.
fn whole(half: &[Pt]) -> Vec<Pt> {
    let mut v = half.to_vec();
    v.extend(half.iter().rev().map(|p| p.mirror_x()));
    v
}

/// Shift a polyline sideways by `d` (positive: to the left of travel in y-down space).
fn shift(pts: &[Pt], d: f64) -> Vec<Pt> {
    let n = pts.len();
    (0..n)
        .map(|i| {
            let a = pts[i.saturating_sub(1)];
            let b = pts[(i + 1).min(n - 1)];
            let t = (b - a).unit();
            pts[i] + pt(t.y, -t.x) * d
        })
        .collect()
}

fn rotate(p: Pt, a: f64) -> Pt {
    let (s, c) = a.sin_cos();
    pt(p.x * c - p.y * s, p.x * s + p.y * c)
}

fn ribcolor(look: &Look) -> String {
    super::tonal(look.get(Zone::Rib).dominant(), 0.25).rgb.css()
}

/// A top's sleeve in the flat: hung from the shoulder point, rotated so its
/// underarm meets the body's underarm. `side` is +1 (wearer's left, drawn on
/// the right) or -1.
fn sleeve(pen: &mut Pen, pat: &Pattern, body: &Piece, look: &Look, side: f64, drop_deg: f64) {
    let s = pat.piece("sleeve");
    let sp = body.edge("shoulder").pts[1];
    let hem = &s.edge("hem").pts;
    let sl = hem[0].y;
    let hem_w = hem[0].x;
    let bicep = s.edge("underarm_front").pts[0];
    let under = geom::length(&s.edge("underarm_front").pts);
    // Local frame: cap top at the origin, sleeve axis +y, underside to -x.
    // The axis hangs `drop_deg` below horizontal; the armhole curve closes
    // the shape back to the shoulder so there is no gap at the seam.
    let u = pt(-bicep.x, sl - under);
    let angle = drop_deg.to_radians() - std::f64::consts::FRAC_PI_2;
    let place = |p: Pt| {
        let q = rotate(p, angle) + sp;
        pt(q.x * side, q.y)
    };
    let local = [pt(0.0, 0.0), pt(0.0, sl), pt(-hem_w, sl), u];
    let mut poly: Vec<Pt> = local.iter().map(|p| place(*p)).collect();
    let arm = &body.edge("armhole").pts;
    poly.extend(arm.iter().rev().map(|p| pt(p.x * side, p.y)));
    let deg = angle.to_degrees() * side;
    let paint = pen.fill(look.get(Zone::Sleeves), place(pt(0.0, bicep.y)), deg);
    pen.shape(&poly, &paint);
    let hem_line: Vec<Pt> = [pt(0.0, sl - 12.0), pt(-hem_w, sl - 12.0)].iter().map(|p| place(*p)).collect();
    // Cuffs sit past the hem; hemmed sleeves show the hem stitching.
    let cuff = pat.pieces.iter().find(|p| p.name == "cuff");
    match cuff {
        Some(cp) => {
            let (w, h) = (cp.len("top") / 2.0, cp.len("end_a") / 2.0);
            let mid = -hem_w / 2.0;
            let r = [pt(mid + w / 2.0, sl), pt(mid + w / 2.0, sl + h), pt(mid - w / 2.0, sl + h), pt(mid - w / 2.0, sl)];
            let rr: Vec<Pt> = r.iter().map(|p| place(*p)).collect();
            let fill = if cp.rib { look.get(Zone::Rib).clone() } else { look.get(Zone::Cuffs).clone() };
            let paint = pen.fill(&fill, rr[0], deg);
            pen.shape(&rr, &paint);
            if cp.rib {
                let rc = ribcolor(look);
                let mut d = String::new();
                let mut x = mid - w / 2.0 + 4.0;
                while x < mid + w / 2.0 - 2.0 {
                    let (a, b) = (place(pt(x, sl + 1.5)), place(pt(x, sl + h - 1.5)));
                    let _ = write!(d, "M{} {} L{} {} ", f(a.x), f(a.y), f(b.x), f(b.y));
                    x += 5.0;
                }
                let _ = writeln!(pen.out, "<path class=\"rb\" stroke=\"{rc}\" d=\"{}\"/>", d.trim_end());
            } else {
                let b = place(pt(mid + w / 2.0 - 10.0, sl + h / 2.0));
                let btn = look.get(Zone::Buttons).dominant().rgb.css();
                let _ = writeln!(pen.out, "<circle class=\"fl\" cx=\"{}\" cy=\"{}\" r=\"5\" fill=\"{btn}\"/>", f(b.x), f(b.y));
            }
        }
        None => pen.stitch(&hem_line),
    }
}

fn top(o: &Outcome, look: &Look, spec: &Spec, cfg: &Config, painter: &mut Painter, back: bool) -> Drawing {
    let pat = &o.pattern;
    let body = pat.piece(if back { "back" } else { "front" });
    let other = pat.piece(if back { "front" } else { "back" });
    let thread = look.get(Zone::Stitching).dominant().rgb.css();
    let mut pen = Pen { out: String::new(), pts: Vec::new(), paint: painter, thread };
    let neck_pts = body.edge("neck").pts.clone();
    let hps = neck_pts.last().copied().expect("neck");
    let nw = hps.x;
    let hood = pat.pieces.iter().find(|p| p.name == "hood side");
    let body_fill = look.get(Zone::Body).clone();
    let chest_y = body.edge("armhole").pts.last().expect("armhole").y;
    let length = body.edge("hem").pts.last().expect("hem").y;

    // Hood behind the body (front view: up, face opening; back view: lying down).
    if let Some(h) = hood {
        let height = h.len("face") + pat.pieces.iter().find(|p| p.name == "hood centre").map_or(0.0, |c| c.len("face") / 2.0);
        let depth = geom::bbox(&h.outline().0).width();
        if !back {
            let paint = pen.fill(look.get(Zone::Hood), pt(0.0, 0.0), 0.0);
            let w = (nw + 25.0).max(depth * 0.62);
            let half = geom::chain(&[cubic(pt(0.0, -height * 0.95), pt(-w * 0.8, -height * 0.95), pt(-w, -height * 0.35), pt(-nw - 15.0, 5.0))]);
            let mut outer = half.clone();
            outer.extend(mirror(&half).into_iter().rev());
            pen.shape(&outer, &paint);
        }
    }

    // Inside of the back neck seen through the front neckline.
    if !back && hood.is_none() && spec.g() != Garment::Shirt {
        let inner = super::tonal(body_fill.dominant(), 0.3).rgb.css();
        let bn = &other.edge("neck").pts;
        let mut poly = mirror(bn).into_iter().rev().collect::<Vec<_>>();
        poly.extend(bn.iter().copied());
        poly.extend(neck_pts.iter().rev().copied());
        poly.extend(mirror(&neck_pts));
        pen.shape(&poly, &inner);
    }

    // Sleeves, then the body over their armhole edge.
    let drop = if pat.piece("sleeve").edge("hem").pts[0].y > 400.0 { cfg.design["flat_long_sleeve"] } else { cfg.design["flat_short_sleeve"] };
    sleeve(&mut pen, pat, body, look, 1.0, drop);
    sleeve(&mut pen, pat, body, look, -1.0, drop);
    let anchor = pt(0.0, chest_y);
    let paint = pen.fill(&body_fill, anchor, 0.0);
    if body.cut.fold {
        let (stitch, _) = body.outlines(0.0, 0.0, true);
        pen.shape(&stitch, &paint);
    } else {
        // Shirt fronts: the left front overlaps the right at the placket.
        let (stitch, _) = body.outlines(0.0, 0.0, false);
        pen.shape(&mirror(&stitch), &paint);
        if !back {
            pen.shape(&stitch, &paint);
        }
    }

    // Hem band or hem stitching.
    if let Some(band) = pat.pieces.iter().find(|p| p.name == "hem band") {
        let (w, h) = (band.len("top") / 2.0, band.len("end_a") / 2.0);
        let rib = look.get(Zone::Rib).clone();
        let paint = pen.fill(&rib, pt(0.0, length), 0.0);
        pen.shape(&[pt(-w, length), pt(w, length), pt(w, length + h), pt(-w, length + h)], &paint);
        pen.rib(-w, w, length, length + h, &ribcolor(look));
    } else {
        let hem = &body.edge("hem").pts;
        let up = shift(hem, -14.0);
        let mut line: Vec<Pt> = up.iter().filter(|p| p.x >= 0.0).copied().collect();
        let left: Vec<Pt> = mirror(&line).into_iter().rev().collect();
        line.reverse();
        let mut all = left;
        all.extend(line.into_iter().rev());
        pen.stitch(&all);
    }

    // Kangaroo pocket.
    if !back {
        if let Some(pts) = body.marks.iter().find_map(|m| match m {
            Mark::Line { pts, dashed: true } if pts.len() == 5 => Some(pts.clone()),
            _ => None,
        }) {
            let poly = whole(&pts);
            let paint = pen.fill(look.get(Zone::Pocket), anchor, 0.0);
            pen.shape(&poly, &paint);
            let inner: Vec<Pt> = pts[1..4].iter().map(|p| *p + pt(-6.0, 6.0)).collect();
            pen.stitch(&inner);
            pen.stitch(&mirror(&inner));
        }
    }

    // Neck finish.
    let band = pat.pieces.iter().find(|p| p.name == "neckband" || p.name == "neck binding");
    if let Some(b) = band {
        let w = b.len("end_a") / 2.0;
        let fill = if b.rib { look.get(Zone::Rib).clone() } else { look.get(Zone::Collar).clone() };
        let paint = pen.fill(&fill, pt(0.0, 0.0), 0.0);
        let curve: Vec<Pt> = mirror(&neck_pts).into_iter().rev().chain(neck_pts.iter().copied()).collect();
        let inner = shift(&curve, w);
        let mut poly = curve.clone();
        poly.extend(inner.into_iter().rev());
        pen.shape(&poly, &paint);
        if b.rib {
            let mid = shift(&curve, w / 2.0);
            let _ = writeln!(pen.out, "<path class=\"rb\" stroke=\"{}\" d=\"{}\"/>", ribcolor(look), path(&mid, false));
        } else {
            pen.stitch(&shift(&curve, w + 3.0));
        }
    }

    // Shirt: placket, buttons, collar.
    if spec.g() == Garment::Shirt {
        let ext = pat.piece("front").edge("cf").pts[0].x.abs();
        let collar = look.get(Zone::Collar).clone();
        let cpaint = pen.fill(&collar, pt(0.0, 0.0), 0.0);
        let depth = neck_pts[0].y;
        if !back {
            for x in [-ext + 3.0, ext - 3.0] {
                pen.stitch(&[pt(x, depth + 4.0), pt(x, length - 6.0)]);
            }
            let btn = look.get(Zone::Buttons).dominant().rgb.css();
            for m in &body.marks {
                if let Mark::Line { pts, dashed: false } = m {
                    if pts.len() == 2 && (pts[0].x - pts[1].x).abs() < 1e-9 {
                        let y = (pts[0].y + pts[1].y) / 2.0;
                        let _ = writeln!(pen.out, "<circle class=\"fl\" cx=\"0\" cy=\"{}\" r=\"5.5\" fill=\"{btn}\"/>", f(y));
                    }
                }
            }
            // Back of the collar standing behind the neck, then the two points.
            pen.shape(&[pt(-nw - 4.0, -2.0), pt(nw + 4.0, -2.0), pt(nw + 2.0, -26.0), pt(-nw - 2.0, -26.0)], &cpaint);
            let wing = vec![pt(-nw - 6.0, -6.0), pt(-nw - 16.0, depth * 0.45), pt(-22.0, depth + 30.0), pt(-2.0, depth + 6.0), pt(-nw * 0.55, 6.0)];
            pen.shape(&wing, &cpaint);
            pen.shape(&mirror(&wing), &cpaint);
        } else {
            let top = cubic(pt(-nw - 6.0, 4.0), pt(-nw * 0.5, 10.0), pt(nw * 0.5, 10.0), pt(nw + 6.0, 4.0));
            let mut poly = vec![pt(-nw - 2.0, -24.0), pt(nw + 2.0, -24.0)];
            poly.extend(top.into_iter().rev());
            pen.shape(&poly, &cpaint);
        }
    }

    // Hood on top: front view face opening; back view hood lying down.
    if let Some(h) = hood {
        let height = h.len("face") + pat.pieces.iter().find(|p| p.name == "hood centre").map_or(0.0, |c| c.len("face") / 2.0);
        let depth = geom::bbox(&h.outline().0).width();
        let fnd = neck_pts[0].y;
        if !back {
            let lining = look.get(Zone::HoodLining).dominant().clone();
            let inside = super::tonal(&lining, 0.2).rgb.css();
            let half = geom::chain(&[
                cubic(pt(0.0, fnd), pt(-nw * 0.55, fnd * 0.6), pt(-nw * 0.95, -height * 0.2), pt(-nw * 0.88, -height * 0.55)),
                cubic(pt(-nw * 0.88, -height * 0.55), pt(-nw * 0.82, -height * 0.74), pt(-nw * 0.42, -height * 0.8), pt(0.0, -height * 0.8)),
            ]);
            let mut opening = half.clone();
            opening.extend(mirror(&half).into_iter().rev());
            pen.shape(&opening, &inside);
            pen.stitch(&shift(&half, -8.0));
            pen.stitch(&mirror(&shift(&half, -8.0)));
            // Drawstrings from the eyelets.
            let dc = look.get(Zone::Drawstring).dominant().rgb.css();
            for s in [-1.0, 1.0] {
                let a = pt(s * nw * 0.42, fnd * 0.5);
                let b = pt(s * nw * 0.36, fnd + 150.0);
                let _ = writeln!(pen.out, "<path class=\"ds\" stroke=\"{dc}\" d=\"{}\"/>", path(&[a, b], false));
                let _ = writeln!(
                    pen.out,
                    "<rect class=\"fl\" x=\"{}\" y=\"{}\" width=\"6\" height=\"16\" fill=\"#9a9b9d\"/>",
                    f(b.x - 3.0),
                    f(b.y - 2.0)
                );
                let _ = writeln!(pen.out, "<circle class=\"fl\" cx=\"{}\" cy=\"{}\" r=\"3.2\" fill=\"#c9c9c9\"/>", f(a.x), f(a.y));
            }
        } else {
            let top = -8.0;
            let half = cubic(pt(-nw - 12.0, top), pt(-nw - depth * 0.4, height * 0.3), pt(-nw * 0.7, height * 0.78), pt(0.0, height * 0.82));
            let mut shape = half.clone();
            shape.extend(mirror(&half).into_iter().rev());
            let paint = pen.fill(look.get(Zone::Hood), pt(0.0, 0.0), 0.0);
            pen.shape(&shape, &paint);
            if let Some(c) = pat.pieces.iter().find(|p| p.name == "hood centre") {
                let w = c.edge("face").pts[1].x * 0.8;
                for x in [-w, w] {
                    pen.stitch(&[pt(x, top + 4.0), pt(x * 0.4, height * 0.78)]);
                }
            } else {
                pen.stitch(&[pt(0.0, top + 2.0), pt(0.0, height * 0.8)]);
            }
            let lining = look.get(Zone::HoodLining).dominant().rgb.css();
            let rim = cubic(pt(-nw - 10.0, top), pt(-nw * 0.5, top - 10.0), pt(nw * 0.5, top - 10.0), pt(nw + 10.0, top));
            let mut r = rim.clone();
            r.extend(shift(&rim, -9.0).into_iter().rev());
            pen.shape(&r, &lining);
        }
    }
    let bbox = geom::bbox(&pen.pts);
    Drawing { svg: pen.out, bbox }
}

/// Horizontal extent of a closed outline at height `y`.
fn span(poly: &[Pt], y: f64) -> f64 {
    let n = poly.len();
    let mut xs = Vec::new();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y <= y && b.y > y) || (b.y <= y && a.y > y) {
            xs.push(a.x + (y - a.y) / (b.y - a.y) * (b.x - a.x));
        }
    }
    let lo = xs.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = xs.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if xs.is_empty() {
        0.0
    } else {
        hi - lo
    }
}

fn pants(o: &Outcome, look: &Look, spec: &Spec, cfg: &Config, painter: &mut Painter, back: bool) -> Drawing {
    let pat = &o.pattern;
    let m = spec.measurements(cfg);
    let (fr, bk) = (pat.piece("front"), pat.piece("back"));
    let (fo, bo) = (fr.outline().0, bk.outline().0);
    let thread = look.get(Zone::Stitching).dominant().rgb.css();
    let mut pen = Pen { out: String::new(), pts: Vec::new(), paint: painter, thread };
    let hem_y = fr.edge("hem").pts[0].y;
    let rise_y = fr.edge("crotch").pts.last().expect("crotch").y;
    let hip_y = m.hip_depth.min(rise_y - 20.0);
    let knee_y = rise_y + (hem_y - rise_y) * 0.5 - 50.0;
    let girth = |y: f64| span(&fo, y) + span(&bo, y);
    let dart = bk.marks.iter().any(|mk| matches!(mk, Mark::Line { pts, .. } if pts.len() == 3));
    let waist_half = (fr.len("waist") + bk.len("waist")) / 2.0 - if dart { 10.0 } else { 0.0 };
    let hip_half = girth(hip_y) / 2.0;
    let knee_half = girth(knee_y) / 4.0;
    let hem_half = (fr.len("hem") + bk.len("hem")) / 4.0;
    let crotch_x = 5.0;
    // Leg centre: under the hip, but far enough out that the legs part
    // below the crotch.
    let xc = ((crotch_x + hip_half) / 2.0).max(crotch_x + knee_half + 12.0).max(crotch_x + hem_half + 22.0);
    let waist_w = waist_half.min(hip_half);
    let leg = vec![
        pt(0.0, 0.0),
        pt(waist_w, 0.0),
        pt(hip_half.max(xc + knee_half * 0.9), hip_y),
        pt(xc + knee_half, knee_y),
        pt(xc + hem_half, hem_y),
        pt(xc - hem_half, hem_y),
        pt(xc - knee_half, knee_y),
        pt(crotch_x, rise_y),
        pt(0.0, rise_y - 30.0),
    ];
    let paint = pen.fill(look.get(Zone::Body), pt(0.0, rise_y), 0.0);
    pen.shape(&mirror(&leg), &paint);
    pen.shape(&leg, &paint);
    for s in [-1.0, 1.0] {
        pen.stitch(&[pt(s * (xc - hem_half + 3.0), hem_y - 30.0), pt(s * (xc + hem_half - 3.0), hem_y - 30.0)]);
    }
    let wb = pat.pieces.iter().find(|p| p.name == "waistband");
    let band_h = wb.map_or(35.0, |w| w.len("end_a") / 2.0);
    let band_paint = pen.fill(look.get(if wb.is_some() { Zone::Waistband } else { Zone::Body }), pt(0.0, 0.0), 0.0);
    pen.shape(&[pt(-waist_w, 0.0), pt(waist_w, 0.0), pt(waist_w, -band_h), pt(-waist_w, -band_h)], &band_paint);
    if wb.is_some() {
        let loops: &[f64] = if back { &[-0.8, -0.35, 0.0, 0.35, 0.8] } else { &[-0.85, -0.45, 0.45, 0.85] };
        for x in loops {
            let x = x * waist_w;
            pen.shape(&[pt(x - 5.0, -band_h - 3.0), pt(x + 5.0, -band_h - 3.0), pt(x + 5.0, 6.0), pt(x - 5.0, 6.0)], &band_paint);
        }
        if !back {
            let btn = look.get(Zone::Buttons).dominant().rgb.css();
            let _ = writeln!(pen.out, "<circle class=\"fl\" cx=\"-12\" cy=\"{}\" r=\"7\" fill=\"{btn}\"/>", f(-band_h / 2.0));
            let fly = fr.edge("fly").pts.clone();
            let depth = fly.last().expect("fly").y;
            let w = 32.0;
            let j = geom::chain(&[vec![pt(-w, 4.0), pt(-w, depth - 30.0)], cubic(pt(-w, depth - 30.0), pt(-w, depth - 5.0), pt(-10.0, depth), pt(0.0, depth))]);
            pen.stitch(&j);
            if o.pattern.notes.iter().any(|n| n.contains("pleat")) {
                for s in [-1.0, 1.0] {
                    pen.line(&[pt(s * xc * 0.9, 2.0), pt(s * xc * 0.9, 70.0)], "fl");
                }
            }
        } else {
            for s in [-1.0, 1.0] {
                let x = s * waist_w * 0.5;
                pen.line(&[pt(x - 9.0, 1.0), pt(x, 110.0), pt(x + 9.0, 1.0)], "fl");
            }
        }
    } else {
        // Elastic casing: gathers and a drawstring.
        let mut x = -waist_w + 8.0;
        let mut d = String::new();
        while x < waist_w - 4.0 {
            let _ = write!(d, "M{} {} L{} {} ", f(x), f(-band_h + 5.0), f(x + 2.0), f(-5.0));
            x += 11.0;
        }
        let _ = writeln!(pen.out, "<path class=\"rb\" stroke=\"{}\" d=\"{}\"/>", super::tonal(look.get(Zone::Body).dominant(), 0.3).rgb.css(), d.trim_end());
        if !back {
            let dc = look.get(Zone::Drawstring).dominant().rgb.css();
            for s in [-1.0, 1.0] {
                let _ = writeln!(pen.out, "<path class=\"ds\" stroke=\"{dc}\" d=\"M{} {} L{} {}\"/>", f(s * 6.0), f(-band_h / 2.0), f(s * 22.0), f(110.0));
            }
        }
    }
    if back {
        pen.line(&[pt(0.0, -band_h), pt(0.0, rise_y - 30.0)], "fl");
    }
    let bbox = geom::bbox(&pen.pts);
    Drawing { svg: pen.out, bbox }
}

pub fn draw(o: &Outcome, look: &Look, spec: &Spec, cfg: &Config, painter: &mut Painter, back: bool) -> Drawing {
    if spec.g().is_top() {
        top(o, look, spec, cfg, painter, back)
    } else {
        pants(o, look, spec, cfg, painter, back)
    }
}
