//! `--explain`: what the parser understood, every decision and its scores,
//! the finalists, and every construction check.

use crate::design::Look;
use crate::layout::Marker;
use crate::model::Spec;
use crate::parser::fmt_mm;
use crate::search::Outcome;
use std::fmt::Write as _;

pub fn explain(spec: &Spec, o: &Outcome, look: &Look, marker: &Marker) -> String {
    let mut s = String::new();
    let p = &o.pattern;
    let _ = writeln!(s, "sentence : {}", spec.sentence);
    let _ = writeln!(s, "garment  : {} <- \"{}\"   size {}   fabric {}", spec.g().name(), spec.garment.words, spec.size.key(), p.fabric);
    if let Some(f) = &spec.fit {
        let _ = writeln!(s, "fit      : {} <- \"{}\"", f.value.key(spec.g()), f.words);
    }
    for e in &spec.edits {
        let amount = if e.given { fmt_mm(e.mm) } else { format!("{} (default from [edits])", fmt_mm(e.mm)) };
        let _ = writeln!(s, "edit     : {} {} <- \"{}\"", e.kind.key(), amount, e.words);
    }
    let pr = &o.profile;
    let _ = writeln!(s, "profile  : finish {:.2}  simplicity {:.2}  economy {:.2}", pr.finish, pr.simplicity, pr.economy);
    let win = &o.finalists[o.best];
    let _ = writeln!(s, "\ndecisions:");
    for slot in &o.slots {
        let _ = writeln!(s, "  {} <- \"{}\"", slot.rule, slot.words);
        for (v, score) in &slot.options {
            let star = if win.choices[slot.rule] == *v { "*" } else { " " };
            let _ = writeln!(s, "    {star} {v:<16} {score:>6.3}");
        }
    }
    let _ = writeln!(s, "\nfinalists{}:", if o.exhaustive { " (beam had no valid pattern; tried every combination)" } else { "" });
    let mut order: Vec<usize> = (0..o.finalists.len()).collect();
    order.sort_by(|a, b| o.finalists[*b].total.partial_cmp(&o.finalists[*a].total).unwrap_or(std::cmp::Ordering::Equal));
    for i in order.into_iter().take(8) {
        let f = &o.finalists[i];
        let choices: Vec<String> = f.choices.iter().map(|(k, v)| format!("{k}={v}")).collect();
        let mark = if i == o.best { ">" } else { " " };
        match (&f.rejected, f.fabric) {
            (Some(why), _) => {
                let _ = writeln!(s, "  {mark} rejected  {}\n      because {why}", choices.join(" "));
            }
            (None, Some((area, cost))) => {
                let _ = writeln!(s, "  {mark} {:>7.3}  {}", f.total, choices.join(" "));
                let judges: Vec<String> = f.judges.iter().map(|(n, v, note)| format!("{n} {v:+.3} ({note})")).collect();
                let _ = writeln!(s, "      local {:+.3}  fabric {area:.2} m² {cost:+.3}  {}", f.local, judges.join("  "));
            }
            _ => {}
        }
    }
    let _ = writeln!(s, "\nchecks:");
    for c in &p.checks {
        let _ = writeln!(s, "  {} {:<44} {}", if c.ok { "ok  " } else { "FAIL" }, c.name, c.detail);
    }
    for n in &p.notes {
        let _ = writeln!(s, "note: {n}");
    }
    let _ = writeln!(s, "\npieces:");
    for pc in &p.pieces {
        let _ = writeln!(s, "  {:<18} {:<14} {:<26} <- \"{}\"", pc.name, pc.cut.describe(), pc.rule, pc.words);
    }
    let _ = writeln!(s, "
colourway: {}", look.name);
    for z in &look.zones {
        let _ = writeln!(s, "  {:<12} {:<34} ({})", z.zone.name(), z.fill.label(), z.why);
    }
    for n in &look.notes {
        let _ = writeln!(s, "note: {n}");
    }
    for sh in &marker.sheets {
        let _ = writeln!(s, "layout: {} {} {:.0} cm wide, {:.2} m long", sh.fabric, sh.label, sh.width / 10.0, sh.length / 1000.0);
    }
    s
}
