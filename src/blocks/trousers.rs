//! The trouser block: front and back legs drafted from waist, seat, rise,
//! inside leg, knee and hem. The back crotch point is solved so the back
//! inseam is a little shorter than the front (it is stretched to fit), and
//! the back side-waist point is trued so both outseams are the same length.
//!
//! Frame: x = 0 is the side seam at seat level, centre front/back to the
//! right; y = 0 is the waist line, y down.

use crate::draft::Draft;
use crate::geom::{self, cubic, pt, Pt};
use crate::pattern::{Cut, Edge, EdgeKind, Mark, Piece};

struct Leg {
    knee_y: f64,
    hem_y: f64,
    crease: f64,
    knee_half: f64,
    hem_half: f64,
}

impl Leg {
    fn points(&self, extend: f64) -> (Pt, Pt, Pt, Pt) {
        let ki = pt(self.crease + self.knee_half, self.knee_y);
        let ko = pt(self.crease - self.knee_half, self.knee_y);
        let mut hi = pt(self.crease + self.hem_half, self.hem_y);
        let mut ho = pt(self.crease - self.hem_half, self.hem_y);
        if extend != 0.0 {
            let k = extend / (self.hem_y - self.knee_y);
            hi = hi + (hi - ki) * k;
            ho = ho + (ho - ko) * k;
        }
        (ki, ko, hi, ho)
    }
}

/// Crotch point -> knee -> hem, curving out of the crotch.
fn inseam(crotch: Pt, knee: Pt, hem: Pt) -> Vec<Pt> {
    let drop = knee.y - crotch.y;
    let dir = (hem - knee).unit();
    geom::chain(&[
        cubic(crotch, crotch + pt((knee.x - crotch.x) * 0.15, drop * 0.35), knee - dir * (drop * 0.35), knee),
        vec![hem],
    ])
}

/// Hem -> knee -> seat level -> side waist.
fn outseam(hem: Pt, knee: Pt, hip_y: f64, waist: Pt) -> Vec<Pt> {
    let span = knee.y - hip_y;
    let up = (knee - hem).unit();
    let hip = pt(0.0, hip_y);
    geom::chain(&[
        vec![hem],
        cubic(knee, knee + up * (span * 0.35), pt(0.0, hip_y + span * 0.35), hip),
        cubic(hip, pt(0.0, hip_y * 0.6), waist.lerp(hip, 0.3), waist),
    ])
}

pub fn draft(d: &mut Draft) -> Result<(), String> {
    let (p, m, cfg) = (d.pants.clone(), d.m, d.cfg);
    let seat = m.seat + p.seat_ease;
    let waist = m.waist + p.waist_ease;
    let hip_y = m.hip_depth;
    let rise_y = m.body_rise + p.rise_ease;
    if rise_y <= hip_y + 20.0 {
        return Err(format!("the rise ({:.1} cm) no longer reaches below the seat", rise_y / 10.0));
    }
    let knee_y = rise_y + p.inseam * 0.5 - 50.0;
    let hem_y = rise_y + p.inseam;
    let bal = cfg.draft("leg_balance");
    if p.leg_extend.abs() > cfg.check("max_extend") {
        return Err(format!("{:.1} cm is more than a leg can be extended", p.leg_extend / 10.0));
    }

    // ---- front ----
    let fh = seat / 4.0 - 10.0;
    let fx = seat * cfg.draft("front_fork");
    let front_leg = Leg { knee_y, hem_y, crease: (fh + fx) / 2.0, knee_half: (p.knee / 2.0 - bal) / 2.0, hem_half: (p.hem / 2.0 - bal) / 2.0 };
    let (ki, ko, hi, ho) = front_leg.points(p.leg_extend);
    if hi.x - ho.x < 40.0 {
        return Err("the leg is too narrow at the hem".into());
    }
    let cf_in = if p.elastic { 0.0 } else { cfg.draft("cf_in") };
    let cf_x = |y: f64| fh - cf_in * (1.0 - y / hip_y);
    let cfw = pt(cf_x(0.0), 0.0);
    let wf = waist / 4.0 - 10.0;
    let (sw, pleat) = if p.elastic {
        (pt(0.0, 0.0), 0.0)
    } else {
        let x0 = cfw.x - wf;
        let shape = cfg.draft("max_side_shape");
        (pt(x0.min(shape), 0.0), (x0 - shape).max(0.0))
    };
    let fc = pt(fh + fx, rise_y);
    let crotch_curve = cubic(pt(fh, hip_y), pt(fh, hip_y + (rise_y - hip_y) * 0.55), pt(fh + fx * 0.4, rise_y), fc);
    let waist_kind = |e: Edge| if p.elastic { Edge { kind: EdgeKind::Hem, sa: Some(cfg.part("elastic_width") * 2.0 + 10.0), ..e } } else { e };
    let mut edges = vec![waist_kind(Edge::new("waist", EdgeKind::Seam, vec![sw, cfw]))];
    let crotch_start = if p.elastic {
        cfw
    } else {
        let (fd, fw) = (cfg.part("fly_depth").min(hip_y - 10.0), cfg.part("fly_width"));
        let bottom = pt(cf_x(fd), fd);
        let turn = pt(cf_x(fd - 25.0) + fw, fd - 25.0);
        let fly = geom::chain(&[vec![cfw, pt(cfw.x + fw, 0.0)], cubic(turn, turn + pt(0.0, 15.0), bottom + pt(10.0, 0.0), bottom)]);
        edges.push(Edge::new("fly", EdgeKind::Seam, fly));
        bottom
    };
    edges.push(Edge::new("crotch", EdgeKind::Seam, geom::chain(&[vec![crotch_start], crotch_curve])));
    let front_in = inseam(fc, ki, hi);
    edges.push(Edge::new("inseam", EdgeKind::Seam, front_in.clone()));
    edges.push(Edge::new("hem", EdgeKind::Hem, vec![hi, ho]));
    let front_out = outseam(ho, ko, hip_y, sw);
    edges.push(Edge::new("outseam", EdgeKind::Seam, front_out.clone()));
    let mut front = Piece::new("front", Cut::PAIR, edges)
        .anchored(pt(front_leg.crease, rise_y))
        .mark(Mark::Notch { edge: "inseam", at: geom::length(&inseam(fc, ki, ki)), count: 1 })
        .mark(Mark::Notch { edge: "outseam", at: ho.dist(ko), count: 1 });
    if pleat > 0.0 {
        let c = front_leg.crease;
        for x in [c - pleat / 2.0, c + pleat / 2.0] {
            front = front.mark(Mark::Line { pts: vec![pt(x, 0.0), pt(x, 60.0)], dashed: false });
        }
        d.notes.push(format!("{:.1} cm of front waist goes into a pleat (the side seam takes at most {:.1} cm)", pleat / 10.0, cfg.draft("max_side_shape") / 10.0));
    }

    // ---- back ----
    let bh = seat / 4.0 + 10.0;
    let bx = seat * cfg.draft("back_fork");
    let back_leg = Leg { knee_y, hem_y, crease: (bh + bx) / 2.0 - 10.0, knee_half: (p.knee / 2.0 + bal) / 2.0, hem_half: (p.hem / 2.0 + bal) / 2.0 };
    let (bki, bko, bhi, bho) = back_leg.points(p.leg_extend);
    let (cb_in, cb_rise) = if p.elastic { (0.0, cfg.draft("cb_rise")) } else { (cfg.draft("cb_in"), cfg.draft("cb_rise")) };
    let cbw = pt(bh - cb_in, -cb_rise);
    let cb_hip = pt(bh, hip_y);
    let u = (cb_hip - cbw).unit();
    let back_crotch = |drop: f64| {
        let bc = pt(bh + bx, rise_y + drop);
        (bc, cubic(cb_hip, cb_hip + u * ((rise_y - hip_y) * 0.5), pt(bh + bx * 0.4, rise_y + drop), bc))
    };
    let want_in = geom::length(&front_in) - cfg.check("inseam_ease");
    let drop = geom::solve(-20.0, 60.0, |dr| want_in - geom::length(&inseam(back_crotch(dr).0, bki, bhi))).unwrap_or(0.0);
    let (bc, back_curve) = back_crotch(drop);
    let back_in = inseam(bc, bki, bhi);

    let dart = if p.elastic { 0.0 } else { cfg.draft("back_dart") };
    let wb = waist / 4.0 + 10.0 + dart;
    let side_waist = |y: f64| {
        if p.elastic {
            pt(0.0, y)
        } else {
            let dy = cbw.y - y;
            pt(cbw.x - (wb * wb - dy * dy).max(0.0).sqrt(), y)
        }
    };
    let front_out_len = geom::length(&front_out);
    let y = geom::solve(-60.0, 60.0, |y| front_out_len - geom::length(&outseam(bho, bko, hip_y, side_waist(y)))).unwrap_or(0.0);
    let bsw = side_waist(y);
    let back_out = outseam(bho, bko, hip_y, bsw);
    let mut back = Piece::new(
        "back",
        Cut::PAIR,
        vec![
            waist_kind(Edge::new("waist", EdgeKind::Seam, vec![bsw, cbw])),
            Edge::new("crotch", EdgeKind::Seam, geom::chain(&[vec![cbw], back_curve])),
            Edge::new("inseam", EdgeKind::Seam, back_in.clone()),
            Edge::new("hem", EdgeKind::Hem, vec![bhi, bho]),
            Edge::new("outseam", EdgeKind::Seam, back_out.clone()),
        ],
    )
    .anchored(pt(back_leg.crease, rise_y))
    .mark(Mark::Notch { edge: "inseam", at: geom::length(&inseam(bc, bki, bki)), count: 2 })
    .mark(Mark::Notch { edge: "outseam", at: bho.dist(bko), count: 2 });
    if dart > 0.0 {
        let mid = bsw.lerp(cbw, 0.5);
        let along = (cbw - bsw).unit();
        let down = pt(-along.y, along.x);
        let apex = mid + down * cfg.draft("back_dart_length");
        back = back.mark(Mark::Line { pts: vec![mid - along * (dart / 2.0), apex, mid + along * (dart / 2.0)], dashed: false });
    }
    d.pants.pleat = pleat;
    d.pants.dart = dart;

    // ---- checks ----
    d.seam("outseam: front / back", front_out_len, geom::length(&back_out), 0.0, 0.0);
    let ie = cfg.check("inseam_ease");
    d.seam("inseam: back / front (back is stretched)", geom::length(&back_in), geom::length(&front_in), ie, ie);
    let hem = (hi.x - ho.x) + (bhi.x - bho.x);
    d.clearance("foot goes through the hem", hem, d.fabric.stretch, m.heel_instep, "heel-to-instep");
    d.clearance("seat ease", seat, d.fabric.stretch, m.seat, "seat");
    let words = d.spec.garment.words.clone();
    d.add(front.by("block/pants", &words));
    d.add(back.by("block/pants", &words));
    Ok(())
}
