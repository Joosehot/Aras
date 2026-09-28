//! SVG output at 1:1 in millimetres, so it prints to scale. Every piece shows
//! its cutting line (solid), stitching line (dashed), grainline, notches,
//! internal marks and a label, and carries a comment naming the rule that
//! drafted it and the words that asked for it.

use crate::config::Config;
use crate::geom::{self, pt, Pt};
use crate::design::{anchor, Look, Painter};
use crate::layout::{Marker, Placement, Sheet};
use crate::pattern::{EdgeKind, Mark, Pattern, Piece};
use crate::search::Outcome;
use std::fmt::Write as _;

pub fn f(v: f64) -> String {
    let s = format!("{v:.1}");
    if s == "-0.0" {
        "0.0".into()
    } else {
        s
    }
}

pub fn path(pts: &[Pt], close: bool) -> String {
    let mut s = String::new();
    for (i, p) in pts.iter().enumerate() {
        let _ = write!(s, "{}{} {}", if i == 0 { "M" } else { " L" }, f(p.x), f(p.y));
    }
    if close {
        s.push_str(" Z");
    }
    s
}

pub fn esc(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace("--", "- -")
}

const STYLE: &str = "<style>
  .fabric { fill: #f1f4f8; stroke: #8a9bb0; stroke-width: 0.8; }
  .rib { fill: #f6f1f8; }
  .cut { fill: #fffdf7; stroke: #1d1d1d; stroke-width: 0.7; }
  .stitch { fill: none; stroke: #7c7c7c; stroke-width: 0.4; stroke-dasharray: 4 2; }
  .mark { fill: none; stroke: #1d1d1d; stroke-width: 0.5; }
  .dash { fill: none; stroke: #1d1d1d; stroke-width: 0.4; stroke-dasharray: 2 2; }
  .centre { fill: none; stroke: #b04a3a; stroke-width: 0.4; stroke-dasharray: 8 2 2 2; }
  .fold { fill: none; stroke: #b04a3a; stroke-width: 1.2; }
  .grain { fill: none; stroke: #2b5d8a; stroke-width: 0.6; }
  text { font-family: Helvetica, Arial, sans-serif; fill: #1d1d1d; }
  .h1 { font-size: 11px; font-weight: bold; }
  .small { font-size: 4.5px; fill: #555; }
  .label { font-size: 9px; font-weight: bold; text-anchor: middle; }
  .sub { font-size: 5px; text-anchor: middle; fill: #333; }
  .lbg { fill: #ffffff; fill-opacity: 0.85; stroke: none; }
</style>
";

/// Draws one placed piece, filled with `paint` (a colour or print) if given.
pub fn piece(out: &mut String, pat: &Pattern, pl: &Placement, show_fold: bool, paint: Option<&str>) {
    let pc: &Piece = &pat.pieces[pl.piece];
    let (seam, hem) = pat.allowances(pc);
    let (stitch, cut) = pc.outlines(seam, hem, pl.unfold);
    let place = |p: Pt| pl.place(p);
    let _ = writeln!(out, "<!-- {}  [{}]  \"{}\" -->", esc(&pc.name), esc(&pc.rule), esc(&pc.words));
    let _ = writeln!(out, "<g>");
    let fill = paint.map(|p| format!(" style=\"fill:{p}\"")).unwrap_or_default();
    let _ = writeln!(out, "<path class=\"cut\"{fill} d=\"{}\"/>", path(&cut.iter().map(|p| place(*p)).collect::<Vec<_>>(), true));
    let _ = writeln!(out, "<path class=\"stitch\" d=\"{}\"/>", path(&stitch.iter().map(|p| place(*p)).collect::<Vec<_>>(), true));

    // Marks, repeated on the mirrored half of an unfolded piece.
    let halves: &[bool] = if pl.unfold { &[false, true] } else { &[false] };
    let (outline, _) = pc.outline();
    let sign = if geom::signed_area(&outline) >= 0.0 { 1.0 } else { -1.0 };
    for &mirror in halves {
        let local = |p: Pt| if mirror { p.mirror_x() } else { p };
        for m in &pc.marks {
            match m {
                Mark::Notch { edge, at, count } => {
                    let e = pc.edge(edge);
                    let depth = e.sa.unwrap_or(if e.kind == EdgeKind::Hem { hem } else { seam }).max(4.0);
                    let (p, dir) = geom::at_length(&e.pts, *at);
                    let n = pt(dir.y, -dir.x) * sign;
                    for k in 0..*count {
                        let off = dir * ((k as f64 - (*count as f64 - 1.0) / 2.0) * 3.0);
                        let a = place(local(p + off));
                        let b = place(local(p + off + n * depth));
                        let _ = writeln!(out, "<path class=\"mark\" d=\"{}\"/>", path(&[a, b], false));
                    }
                }
                Mark::Line { pts, dashed } => {
                    let w: Vec<Pt> = pts.iter().map(|p| place(local(*p))).collect();
                    let _ = writeln!(out, "<path class=\"{}\" d=\"{}\"/>", if *dashed { "dash" } else { "mark" }, path(&w, false));
                }
            }
        }
    }

    // Fold edge: on the pattern sheet it is a thick line to place on the
    // fold; unfolded on the fabric it becomes the centre line.
    if pc.cut.fold {
        let fold = &pc.edges[pc.edges.len() - 1];
        let w: Vec<Pt> = fold.pts.iter().map(|p| place(*p)).collect();
        let class = if show_fold { "fold" } else { "centre" };
        let _ = writeln!(out, "<path class=\"{class}\" d=\"{}\"/>", path(&w, false));
        if show_fold {
            let mid = w[0].lerp(w[w.len() - 1], 0.5);
            let _ = writeln!(
                out,
                "<text class=\"small\" transform=\"translate({} {}) rotate(-90)\" text-anchor=\"middle\">PLACE ON FOLD</text>",
                f(mid.x + 5.0),
                f(mid.y)
            );
        }
    }

    // Grainline, clear of any centre line.
    let r = pl.rect;
    let gx = if pc.cut.fold && pl.unfold { r.min.x + r.width() * 0.25 } else { r.min.x + r.width() * 0.5 };
    let (gy0, gy1) = (r.min.y + r.height() * 0.15, r.max.y - r.height() * 0.15);
    if gy1 - gy0 > 12.0 {
        let _ = writeln!(
            out,
            "<path class=\"grain\" d=\"M{x} {a} L{x} {b} M{l} {a2} L{x} {a} L{rr} {a2} M{l} {b2} L{x} {b} L{rr} {b2}\"/>",
            x = f(gx),
            a = f(gy0),
            b = f(gy1),
            a2 = f(gy0 + 4.0),
            b2 = f(gy1 - 4.0),
            l = f(gx - 2.0),
            rr = f(gx + 2.0)
        );
    }

    // Label.
    let c = pt(r.min.x + r.width() / 2.0, r.min.y + r.height() / 2.0);
    let fabric = if pc.rib { "rib" } else { pat.fabric.as_str() };
    let cut_text = if pl.unfold { format!("{} · {fabric}", pc.cut.describe().replace(" on fold", " (unfolded)")) } else { format!("{} · {fabric}", pc.cut.describe()) };
    let lw = (pc.name.len() as f64 * 6.2).max(cut_text.len() as f64 * 2.9).max(40.0);
    let _ = writeln!(out, "<rect class=\"lbg\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"25\" rx=\"2\"/>", f(c.x - lw / 2.0 - 3.0), f(c.y - 9.0), f(lw + 6.0));
    let _ = writeln!(out, "<text class=\"label\" x=\"{}\" y=\"{}\">{}</text>", f(c.x), f(c.y), esc(&pc.name.to_uppercase()));
    let _ = writeln!(out, "<text class=\"sub\" x=\"{}\" y=\"{}\">{}</text>", f(c.x), f(c.y + 7.0), esc(&cut_text));
    let _ = writeln!(out, "<text class=\"sub\" x=\"{}\" y=\"{}\">size {} · {}</text>", f(c.x), f(c.y + 13.0), esc(&pat.size), esc(&pc.rule));
    let _ = writeln!(out, "</g>");
}

fn header(out: &mut String, pat: &Pattern, spec_sentence: &str, lines: &[String], margin: f64) -> f64 {
    let mut y = margin + 10.0;
    let _ = writeln!(out, "<text class=\"h1\" x=\"{}\" y=\"{}\">Aras · {} · size {}</text>", f(margin), f(y), esc(&pat.title), esc(&pat.size));
    y += 8.0;
    let _ = writeln!(out, "<text class=\"small\" x=\"{}\" y=\"{}\">\"{}\"</text>", f(margin), f(y), esc(spec_sentence));
    for l in lines {
        y += 6.0;
        let _ = writeln!(out, "<text class=\"small\" x=\"{}\" y=\"{}\">{}</text>", f(margin), f(y), esc(l));
    }
    y + 10.0
}

pub fn open(out: &mut String, w: f64, h: f64) {
    open_bare(out, w, h);
    out.push_str(STYLE);
}

/// The SVG header without the pattern stylesheet.
pub fn open_bare(out: &mut String, w: f64, h: f64) {
    let _ = writeln!(out, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>");
    let _ = writeln!(
        out,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}mm\" height=\"{h}mm\" viewBox=\"0 0 {w} {h}\">",
        w = f(w),
        h = f(h)
    );
}

fn summary(pat: &Pattern, o: &Outcome) -> Vec<String> {
    let win = &o.finalists[o.best];
    let mut lines = Vec::new();
    for s in &o.slots {
        lines.push(format!("{} = {}   <- \"{}\"", s.rule, win.choices[s.rule], s.words));
    }
    lines.push(format!(
        "seam allowance {:.0} mm, hem {:.0} mm included · every seam checked: {} checks passed",
        pat.seam,
        pat.hem,
        pat.checks.len()
    ));
    for n in &pat.notes {
        lines.push(format!("note: {n}"));
    }
    lines
}

/// One fabric sheet with its pieces at `shift`, filled from `look` if given.
pub fn fabric_sheet(out: &mut String, pat: &Pattern, s: &Sheet, shift: Pt, look: Option<&Look>, painter: &mut Painter) {
    let class = if s.rib { "fabric rib" } else { "fabric" };
    let _ = writeln!(out, "<rect class=\"{class}\" x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\"/>", f(shift.x), f(shift.y), f(s.width), f(s.length));
    for p in &s.placed {
        let moved = Placement { shift: p.shift + shift, rect: geom::Rect { min: p.rect.min + shift, max: p.rect.max + shift }, ..p.clone() };
        let paint = look.map(|l| painter.paint(&l.pieces[p.piece], moved.place(anchor(&pat.pieces[p.piece])), 0.0));
        piece(out, pat, &moved, false, paint.as_deref());
    }
}

/// The cutting layout on fabric, one sheet per fabric and colourway.
pub fn marker(pat: &Pattern, o: &Outcome, m: &Marker, look: Option<&Look>, sentence: &str, cfg: &Config) -> String {
    let margin = cfg.layout("margin");
    let lines = summary(pat, o);
    let head = margin + 10.0 + 8.0 + 6.0 * lines.len() as f64 + 10.0;
    let width = m.sheets.iter().map(|s| s.width).fold(0.0, f64::max) + 2.0 * margin;
    let height = head + m.sheets.iter().map(|s| s.length + 30.0).sum::<f64>() + margin;
    let mut body = String::new();
    let mut painter = Painter::new("m");
    let _ = writeln!(body, "<!-- Aras: \"{}\" -->", esc(sentence));
    let _ = writeln!(body, "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"#ffffff\"/>", f(width), f(height));
    let mut y = header(&mut body, pat, sentence, &lines, margin);
    for s in &m.sheets {
        let colour = if s.label.is_empty() { String::new() } else { format!(" · {}", s.label) };
        let _ = writeln!(
            body,
            "<text class=\"small\" x=\"{}\" y=\"{}\">{}{} · {:.0} cm wide · {:.2} m long · single layer, right side up, selvedges left and right</text>",
            f(margin),
            f(y + 8.0),
            esc(&s.fabric),
            esc(&colour),
            s.width / 10.0,
            s.length / 1000.0
        );
        y += 14.0;
        fabric_sheet(&mut body, pat, s, pt(margin, y), look, &mut painter);
        y += s.length + 16.0;
    }
    let mut out = String::new();
    open(&mut out, width, height);
    if !painter.defs.is_empty() {
        let _ = write!(out, "<defs>\n{}</defs>\n", painter.defs);
    }
    out.push_str(&body);
    out.push_str("</svg>\n");
    out
}

/// The printable pattern: halves on the fold, with a 10 cm test square.
pub fn pattern_sheet(pat: &Pattern, o: &Outcome, sheet: &Sheet, sentence: &str, cfg: &Config) -> String {
    let margin = cfg.layout("margin");
    let lines = summary(pat, o);
    let head = margin + 10.0 + 8.0 + 6.0 * lines.len() as f64 + 10.0;
    let square = 100.0;
    let width = sheet.width + 2.0 * margin;
    let height = head + square + 20.0 + sheet.length + margin;
    let mut out = String::new();
    open(&mut out, width, height);
    let _ = writeln!(out, "<!-- Aras: \"{}\" -->", esc(sentence));
    let _ = writeln!(out, "<rect x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" fill=\"#ffffff\"/>", f(width), f(height));
    let y = header(&mut out, pat, sentence, &lines, margin);
    let _ = writeln!(out, "<rect class=\"mark\" x=\"{}\" y=\"{}\" width=\"{sq}\" height=\"{sq}\"/>", f(margin), f(y), sq = f(square));
    let _ = writeln!(out, "<text class=\"sub\" x=\"{}\" y=\"{}\">10 cm test square</text>", f(margin + square / 2.0), f(y + square / 2.0));
    let shift = pt(margin, y + square + 20.0);
    for p in &sheet.placed {
        let moved = Placement { shift: p.shift + shift, rect: geom::Rect { min: p.rect.min + shift, max: p.rect.max + shift }, ..p.clone() };
        piece(&mut out, pat, &moved, true, None);
    }
    out.push_str("</svg>\n");
    out
}
