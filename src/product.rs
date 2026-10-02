//! The product sheet (a one-page tech pack): name and style code, front and
//! back flats in the colourway, swatches with the reason for every colour,
//! the construction decisions and checks, the bill of materials, and the
//! cutting layout per fabric and colour. Millimetres, A3 width.

use crate::config::Config;
use crate::design::{self, sketch, Look, Painter};
use crate::layout::Marker;
use crate::model::{Garment, Spec};
use crate::search::Outcome;
use crate::svg::{esc, f, fabric_sheet, open_bare};
use crate::geom::pt;
use std::fmt::Write as _;

const PAGE_W: f64 = 420.0;
const M: f64 = 15.0;

const STYLE: &str = "<style>
  text { font-family: Helvetica, Arial, sans-serif; fill: #1d1d1d; }
  .title { font-size: 11px; font-weight: bold; letter-spacing: 0.2px; }
  .sub { font-size: 3.6px; fill: #555; }
  .quote { font-size: 3.4px; fill: #555; font-style: italic; }
  .h { font-size: 3.4px; font-weight: bold; letter-spacing: 0.6px; fill: #1d1d1d; }
  .b { font-size: 3.2px; }
  .bb { font-size: 3.2px; font-weight: bold; }
  .s { font-size: 2.6px; fill: #6b6b6b; }
  .ok { font-size: 3.2px; fill: #2e7d4f; font-weight: bold; }
  .warn { font-size: 3.0px; fill: #a15c00; }
  .cap { font-size: 3.2px; font-weight: bold; text-anchor: middle; letter-spacing: 0.8px; fill: #555; }
  .rule { stroke: #d6d6d6; stroke-width: 0.3; }
  .sw { stroke: #1d1d1d; stroke-width: 0.8; }
  .fl { stroke: #232323; stroke-width: 1.8; stroke-linejoin: round; }
  .fs { fill: none; stroke-width: 1.1; stroke-dasharray: 6 4; }
  .rb { fill: none; stroke-width: 0.9; }
  .ds { fill: none; stroke-width: 6; stroke-linecap: round; }
  .fabric { fill: #f1f4f8; stroke: #8a9bb0; stroke-width: 2; }
  .rib { fill: #f6f1f8; }
  .cut { fill: #fffdf7; stroke: #1d1d1d; stroke-width: 1.4; }
  .stitch { display: none; }
  .mark, .dash, .centre, .fold, .grain, .label, .lbg { display: none; }
  .sheet .sub { display: none; }
</style>
";

/// FNV-1a: a short, stable style code from the sentence.
fn code(s: &str) -> String {
    let mut h: u32 = 0x811c9dc5;
    for b in s.trim().bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(0x01000193);
    }
    format!("{:04X}", h & 0xffff)
}

fn garment_code(g: Garment) -> &'static str {
    match g {
        Garment::Tshirt => "TS",
        Garment::Hoodie => "HD",
        Garment::Sweatshirt => "SW",
        Garment::Shirt => "SH",
        Garment::Pants => "PT",
        Garment::Dress => "DR",
    }
}

fn text(out: &mut String, class: &str, x: f64, y: f64, s: &str) {
    let _ = writeln!(out, "<text class=\"{class}\" x=\"{}\" y=\"{}\">{}</text>", f(x), f(y), esc(s));
}

pub fn sheet(spec: &Spec, o: &Outcome, look: &Look, marker: &Marker, cfg: &Config) -> String {
    let pat = &o.pattern;
    let mut body = String::new();
    let mut paint = Painter::new("p");

    // ---- header ----
    let style = format!("ARS-{}-{}", garment_code(spec.g()), code(&spec.sentence));
    let fit = spec.fit().key(spec.g());
    text(&mut body, "title", M, M + 9.0, &look.name);
    text(&mut body, "sub", M, M + 16.0, &format!("{style}   ·   size {}   ·   {fit} fit   ·   {}", pat.size, pat.fabric));
    text(&mut body, "quote", M, M + 22.0, &format!("\u{201c}{}\u{201d}", spec.sentence));
    let _ = writeln!(body, "<path class=\"rule\" d=\"M{} {} L{} {}\"/>", f(M), f(M + 27.0), f(PAGE_W - M), f(M + 27.0));

    // ---- flats ----
    let (fx, fy, cw, ch) = (M, M + 34.0, 128.0, 180.0);
    let front = sketch::draw(o, look, spec, cfg, &mut paint, false);
    let back = sketch::draw(o, look, spec, cfg, &mut paint, true);
    let k = [&front, &back].iter().map(|d| (cw / d.bbox.width()).min(ch / d.bbox.height())).fold(f64::INFINITY, f64::min);
    for (i, (d, cap)) in [(&front, "FRONT"), (&back, "BACK")].into_iter().enumerate() {
        let x0 = fx + i as f64 * (cw + 8.0);
        let tx = x0 + (cw - d.bbox.width() * k) / 2.0 - d.bbox.min.x * k;
        let ty = fy + (ch - d.bbox.height() * k) - d.bbox.min.y * k;
        let _ = writeln!(body, "<g transform=\"translate({} {}) scale({k:.4})\">\n{}</g>", f(tx), f(ty), d.svg);
        let _ = writeln!(body, "<text class=\"cap\" x=\"{}\" y=\"{}\">{cap}</text>", f(x0 + cw / 2.0), f(fy + ch + 7.0));
    }

    // ---- right column: colourway, construction ----
    let rx = M + 2.0 * cw + 8.0 + 14.0;
    let mut y = fy + 2.0;
    text(&mut body, "h", rx, y, "COLOURWAY");
    y += 5.0;
    let sk = 0.4;
    for z in &look.zones {
        let fill = paint.paint(&z.fill, pt(0.0, 0.0), 0.0);
        let _ = writeln!(
            body,
            "<g transform=\"translate({} {}) scale({sk})\"><rect class=\"sw\" x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" rx=\"2\" fill=\"{fill}\"/></g>",
            f(rx),
            f(y),
            f(14.0 / sk),
            f(9.0 / sk)
        );
        text(&mut body, "bb", rx + 17.0, y + 3.2, &z.zone.name().to_uppercase());
        text(&mut body, "b", rx + 17.0, y + 6.6, &z.fill.label());
        text(&mut body, "s", rx + 17.0, y + 9.6, &z.why);
        y += 13.0;
    }
    for n in &look.notes {
        text(&mut body, "warn", rx, y, &format!("note: {n}"));
        y += 5.0;
    }
    y += 4.0;
    text(&mut body, "h", rx, y, "CONSTRUCTION");
    y += 5.5;
    let win = &o.finalists[o.best];
    for s in &o.slots {
        text(&mut body, "b", rx, y, &format!("{}: {}", s.rule.replace('_', " "), win.choices[s.rule].replace('_', " ")));
        text(&mut body, "s", rx, y + 3.2, &format!("\u{2190} \u{201c}{}\u{201d}", s.words));
        y += 7.4;
    }
    let passed = pat.checks.iter().filter(|c| c.ok).count();
    text(&mut body, "ok", rx, y + 1.0, &format!("\u{2713} {passed}/{} construction checks passed", pat.checks.len()));
    y += 5.5;
    for n in &pat.notes {
        text(&mut body, "s", rx, y, &format!("note: {n}"));
        y += 3.6;
    }
    let right_end = y;

    // ---- bill of materials ----
    let mut by = fy + ch + 16.0;
    text(&mut body, "h", M, by, "BILL OF MATERIALS");
    by += 5.5;
    for s in &marker.sheets {
        let what = format!("{} {}, {:.0} cm wide", s.fabric, if s.label.is_empty() { String::new() } else { format!("\u{00b7} {}", s.label) }, s.width / 10.0);
        text(&mut body, "bb", M, by, &format!("{:.2} m", s.length / 1000.0));
        text(&mut body, "b", M + 22.0, by, &what);
        by += 4.8;
    }
    for (q, item) in design::notions(spec, o, look, cfg) {
        text(&mut body, "bb", M, by, &q);
        text(&mut body, "b", M + 22.0, by, &item);
        by += 4.8;
    }
    text(&mut body, "s", M, by + 1.0, &format!("seam allowance {:.0} mm and hems {:.0} mm included in every piece", pat.seam, pat.hem));
    by += 6.0;

    // ---- cutting layout thumbnails ----
    let mut ly = by.max(right_end) + 8.0;
    let _ = writeln!(body, "<path class=\"rule\" d=\"M{} {} L{} {}\"/>", f(M), f(ly - 5.0), f(PAGE_W - M), f(ly - 5.0));
    text(&mut body, "h", M, ly, "CUTTING LAYOUT");
    ly += 4.0;
    let ks = 0.11;
    let mut lx = M;
    let mut row_h: f64 = 0.0;
    for s in &marker.sheets {
        let (w, h) = (s.width * ks, s.length * ks);
        if lx + w > PAGE_W - M && lx > M {
            lx = M;
            ly += row_h + 12.0;
            row_h = 0.0;
        }
        let _ = writeln!(body, "<g class=\"sheet\" transform=\"translate({} {}) scale({ks})\">", f(lx), f(ly + 4.0));
        fabric_sheet(&mut body, pat, s, pt(0.0, 0.0), Some(look), &mut paint, cfg);
        let _ = writeln!(body, "</g>");
        let label = if s.label.is_empty() { s.fabric.clone() } else { format!("{} \u{00b7} {}", s.fabric, s.label) };
        text(&mut body, "s", lx, ly + 2.6, &format!("{label} \u{00b7} {:.2} m", s.length / 1000.0));
        lx += w + 10.0;
        row_h = row_h.max(h + 6.0);
    }
    let height = ly + row_h + 14.0;
    text(&mut body, "s", M, height - 6.0, "Aras \u{00b7} drafted from body measurements by hand-written rules \u{00b7} same sentence, same rules.toml, same sheet");

    let mut out = String::new();
    open_bare(&mut out, PAGE_W, height);
    out.push_str(STYLE);
    let _ = writeln!(out, "<defs>\n{}</defs>", paint.defs);
    let _ = writeln!(out, "<!-- Aras: \"{}\" -->", esc(&spec.sentence));
    let _ = writeln!(out, "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"#ffffff\"/>", f(PAGE_W), f(height));
    out.push_str(&body);
    out.push_str("</svg>\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn style_code_is_stable() {
        assert_eq!(code("make a hoodie"), code("  make a hoodie\n"));
        assert_ne!(code("make a hoodie"), code("make a t-shirt"));
    }
}
