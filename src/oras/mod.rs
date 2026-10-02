//! Oras: the garment on a body. Aras drafts the pattern; Oras builds a
//! tailor's dress form from the size, turns the body pieces into cloth,
//! pins and sews them on and lets them fall with position-based dynamics.
//! Out come a 3D view, an OBJ and a report.
//!
//! v0 simulates the body panels (front, back, a gathered skirt); sleeves,
//! hoods, collars, bands and facings are left off and the report says so.

pub mod cloth;
pub mod mannequin;
pub mod panel;
pub mod sdf;
pub mod view;

use crate::config::Config;
use crate::Compiled;

pub struct Outcome {
    pub body: mannequin::Body,
    pub cloth: cloth::Cloth,
    pub report: cloth::Report,
    pub left_off: Vec<String>,
    pub text: String,
}

const SIMULATED: &[&str] = &["front", "back", "skirt front", "skirt back"];

pub fn simulate(c: &Compiled, cfg: &Config) -> Result<Outcome, String> {
    let o = |k: &str| cfg.oras.get(k).copied().ok_or_else(|| format!("rules.toml: missing oras.{k}"));
    let t0 = std::time::Instant::now();
    let m = c.spec.measurements(cfg);
    let body = mannequin::form(m);
    let pat = &c.outcome.pattern;
    let spacing = o("spacing")?;
    // a gathered skirt hangs from the bodice's waist seam
    let waist_y = pat.pieces.iter().find(|p| p.name == "front").and_then(|p| p.edges.iter().find(|e| e.name == "waist")).map_or(0.0, |e| e.pts[0].y);
    let panels: Vec<(panel::Panel, f64)> = pat
        .pieces
        .iter()
        .filter(|p| SIMULATED.contains(&p.name.as_str()))
        .map(|p| (panel::build(p, spacing), if p.name.starts_with("skirt") { waist_y } else { 0.0 }))
        .collect();
    let left_off: Vec<String> = pat.pieces.iter().filter(|p| !SIMULATED.contains(&p.name.as_str())).map(|p| p.name.clone()).collect();
    let params = cloth::Params {
        gap: o("gap")?,
        steps: o("steps")? as usize,
        iterations: o("iterations")? as usize,
        dt: o("dt")?,
        damping: o("damping")?,
        stretch: o("stretch")?,
        bend: o("bend")?,
        seam: o("seam")?,
        ramp: o("ramp")? as usize,
        thickness: o("thickness")?,
        friction: o("friction")?,
    };
    // how far the form reaches front and back at each height (10 mm bands),
    // for hanging the panels clear of it
    let mut bands: std::collections::BTreeMap<i64, f64> = std::collections::BTreeMap::new();
    for p in &body.v {
        let e = bands.entry((p[1] / 10.0).round() as i64).or_insert(0.0);
        *e = e.max((p[2] - body.z0).abs());
    }
    let depth_at = |y: f64| {
        let k = (y / 10.0).round() as i64;
        // the deepest within 150 mm above or below, so a panel hangs clear
        // of the bust and never starts inside the form
        bands.range(k - 15..=k + 15).map(|(_, d)| *d).fold(0.0, f64::max)
    };
    let mut cl = cloth::Cloth::new(&panels, &body, &params, &depth_at);
    let t_build = t0.elapsed().as_secs_f64();
    // the field around where the garment can be
    let cell = o("sdf_cell")?;
    let lo = [-450.0, (body.neck_y - 1600.0).max(0.0), body.z0 - 350.0];
    let hi = [450.0, body.neck_y + 250.0, body.z0 + 350.0];
    let field = sdf::Sdf::build(&body.v, &body.t, lo, hi, cell);
    let t_sdf = t0.elapsed().as_secs_f64() - t_build;
    let report = cl.run(&field, &params);
    let t_sim = t0.elapsed().as_secs_f64() - t_build - t_sdf;
    let (gw, gc, gs) = body.girths;
    let mut text = vec![
        format!("Oras: \"{}\" on a size {} mannequin", c.spec.sentence, c.spec.size.key()),
        format!("dress form: built from the size, {} triangles; measured on it: waist {:.1} cm (size {:.1}), chest {:.1} cm (size {:.1}), seat {:.1} cm (size {:.1})", body.t.len(), gw / 10.0, m.waist / 10.0, gc / 10.0, m.chest / 10.0, gs / 10.0, m.seat / 10.0),
        format!(
            "cloth: {} pieces, {} particles, {} triangles, {} seam stitches, {} steps of {:.1} ms",
            panels.len(),
            report.particles,
            report.triangles,
            report.seam_pairs,
            params.steps,
            params.dt * 1000.0
        ),
        format!(
            "settled: seams closed to {:.1} mm on average ({:.1} mm at most), cloth stretched {:.1}% at most, {:.0}% of it resting on the body, deepest into the body {:.1} mm, still moving {:.3} mm a step",
            report.seam_gap.0,
            report.seam_gap.1,
            report.max_stretch * 100.0,
            report.contact * 100.0,
            report.penetration,
            report.settle
        ),
        format!("time: mesh {:.1} s, body field {:.1} s, simulation {:.1} s", t_build, t_sdf, t_sim),
    ];
    text.push(format!("pinned to the form: {} points along the shoulder seams, held at the back shoulder's width (free to settle up, down, forward and back)", cl.pins.len()));
    text.push(format!("seams: {}", cl.sewn.iter().map(|(n, k)| format!("{n} {k}")).collect::<Vec<_>>().join(", ")));
    if !left_off.is_empty() {
        text.push(format!("not simulated in v0: {}", left_off.join(", ")));
    }
    Ok(Outcome { body, cloth: cl, report, left_off, text: text.join("\n") })
}
