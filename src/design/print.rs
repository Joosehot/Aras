//! Prints: stripes, pinstripes, gingham, plaid, dots and camo, drawn as SVG
//! `<pattern>` tiles in millimetres. Every print is anchored per piece
//! (see `anchor` in `design/mod.rs`) so it matches across seams, and every
//! tile is computed from fixed numbers, so the same print is byte-identical.

use super::color::{Color, Rgb};
use crate::config::Config;
use crate::geom::Pt;
use crate::svg::f;
use std::fmt::Write as _;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum PrintKind {
    Stripes,
    VerticalStripes,
    Pinstripes,
    Gingham,
    Plaid,
    Dots,
    Camo,
    /// Small five-petal flowers scattered on the ground colour.
    Florals,
    /// Placement bands from the hem into the sleeves (see `sweep.rs`).
    Sweep,
}

impl PrintKind {
    pub fn name(self) -> &'static str {
        match self {
            PrintKind::Stripes => "stripes",
            PrintKind::VerticalStripes => "vertical stripes",
            PrintKind::Pinstripes => "pinstripes",
            PrintKind::Gingham => "gingham",
            PrintKind::Plaid => "plaid",
            PrintKind::Dots => "polka dots",
            PrintKind::Camo => "camo",
            PrintKind::Florals => "florals",
            PrintKind::Sweep => "sweep stripes",
        }
    }
    /// "Striped T-shirt", "Gingham Shirt".
    pub fn adjective(self) -> &'static str {
        match self {
            PrintKind::Stripes => "Striped",
            PrintKind::VerticalStripes => "Vertical Stripe",
            PrintKind::Pinstripes => "Pinstriped",
            PrintKind::Gingham => "Gingham",
            PrintKind::Plaid => "Plaid",
            PrintKind::Dots => "Polka Dot",
            PrintKind::Camo => "Camo",
            PrintKind::Florals => "Floral",
            PrintKind::Sweep => "Sweep",
        }
    }
    fn repeat_key(self) -> &'static str {
        match self {
            PrintKind::Stripes | PrintKind::VerticalStripes => "stripe_repeat",
            PrintKind::Pinstripes => "pinstripe_repeat",
            PrintKind::Gingham => "gingham_repeat",
            PrintKind::Plaid => "plaid_repeat",
            PrintKind::Dots => "dot_repeat",
            PrintKind::Camo => "camo_repeat",
            PrintKind::Florals => "floral_repeat",
            PrintKind::Sweep => "sweep_band",
        }
    }
    /// Prints with a horizontal bar must be matched at side seams and
    /// underarms; prints with a vertical bar are centred on centre front.
    pub fn horizontal(self) -> bool {
        matches!(self, PrintKind::Stripes | PrintKind::Gingham | PrintKind::Plaid)
    }
}

/// A resolved print: kind, at least two colours, repeat in mm.
#[derive(Clone, Debug, PartialEq)]
pub struct Print {
    pub kind: PrintKind,
    pub colors: Vec<Color>,
    pub repeat: f64,
}

/// Fills in the colours the sentence didn't give and the repeat size.
pub fn resolve(kind: PrintKind, given: &[Color], scale: f64, base: &Color, cfg: &Config) -> Print {
    let first = given.first().cloned().unwrap_or_else(|| base.clone());
    let named = |n: &str| Color::named(n).expect("palette colour");
    // Second colour: the classic partner for a light or dark ground.
    let partner = |c: &Color| if c.rgb.luminance() < 0.4 { named("off white") } else { named("navy") };
    let mut colors: Vec<Color> = given.to_vec();
    if colors.is_empty() {
        colors.push(first.clone());
    }
    match kind {
        PrintKind::Camo if colors.len() == 1 => {
            let c = colors[0].clone();
            colors.push(Color::new(format!("dark {}", c.name), c.rgb.darker(0.35)));
            colors.push(Color::new(format!("light {}", c.name), c.rgb.lighter(0.3)));
            colors.push(named("black"));
        }
        // Camo in the sentence's own colours: fill up with a blend of the
        // first two and a deeper shade of the ground, never foreign colours.
        PrintKind::Camo if colors.len() < 4 => {
            let (a, b) = (colors[0].clone(), colors[1].clone());
            colors.push(Color::new(format!("{} {}", a.name, b.name), a.rgb.mix(b.rgb, 0.5)));
            if colors.len() < 4 {
                colors.push(Color::new(format!("deep {}", a.name), a.rgb.darker(0.4)));
            }
        }
        PrintKind::Plaid if colors.len() == 1 => {
            let c = colors[0].clone();
            colors.push(Color::new(format!("dark {}", c.name), c.rgb.darker(0.45)));
            colors.push(partner(&c));
        }
        PrintKind::Plaid if colors.len() == 2 => {
            let c = colors[0].clone();
            colors.push(Color::new(format!("dark {}", c.name), c.rgb.darker(0.45)));
        }
        // Florals: ground, petals, and a centre a shade deeper than the petals.
        PrintKind::Florals => {
            // "a light blue dress with yellow flowers": one flower colour is
            // the petals; the ground is the garment's colour
            if given.len() == 1 && base.rgb != given[0].rgb {
                colors = vec![base.clone(), given[0].clone()];
            }
            if colors.len() == 1 {
                colors.push(if colors[0].rgb.luminance() < 0.4 { named("off white") } else { named("white") });
            }
            if colors.len() == 2 {
                let p = colors[1].clone();
                // a white eye, unless the petals are white themselves
                let centre = if p.rgb.luminance() > 0.8 { Color::new(format!("deep {}", p.name), p.rgb.darker(0.4)) } else { named("white") };
                colors.push(centre);
            }
        }
        _ if colors.len() == 1 && kind != PrintKind::Sweep => {
            let c = colors[0].clone();
            colors.push(partner(&c));
        }
        _ => {}
    }
    Print { kind, colors, repeat: cfg.design[kind.repeat_key()] * scale }
}

/// Deterministic generator for camo blobs.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f64) / ((1u64 << 31) as f64)
    }
}

fn rect(s: &mut String, x: f64, y: f64, w: f64, h: f64, c: Rgb, opacity: f64) {
    let op = if opacity < 1.0 { format!(" fill-opacity=\"{}\"", f(opacity)) } else { String::new() };
    let _ = write!(s, "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"{op}/>", f(x), f(y), f(w), f(h), c.css());
}

/// The `<pattern>` element for `print`, its origin at `anchor`, rotated by
/// `angle` degrees (sleeves drawn at an angle keep their stripes square to
/// the sleeve).
pub fn pattern_def(id: &str, print: &Print, anchor: Pt, angle: f64) -> String {
    let r = print.repeat;
    let c: Vec<Rgb> = print.colors.iter().map(|c| c.rgb).collect();
    let mut s = String::new();
    // (offset x, offset y): where the tile's origin sits relative to the anchor.
    let (ox, oy) = match print.kind {
        // Stripes take any number of colours, one equal band each.
        PrintKind::Stripes | PrintKind::Sweep => {
            let n = c.len() as f64;
            rect(&mut s, 0.0, 0.0, r, r, c[0], 1.0);
            for (i, col) in c.iter().enumerate().skip(1) {
                rect(&mut s, 0.0, r * i as f64 / n, r, r / n, *col, 1.0);
            }
            (0.0, 0.0)
        }
        PrintKind::VerticalStripes => {
            let n = c.len() as f64;
            rect(&mut s, 0.0, 0.0, r, r, c[0], 1.0);
            for (i, col) in c.iter().enumerate().skip(1) {
                rect(&mut s, r * i as f64 / n, 0.0, r / n, r, *col, 1.0);
            }
            (-r / (2.0 * n), 0.0)
        }
        PrintKind::Pinstripes => {
            rect(&mut s, 0.0, 0.0, r, r, c[0], 1.0);
            rect(&mut s, 0.0, 0.0, r * 0.1, r, c[1], 1.0);
            (-r * 0.05, 0.0)
        }
        PrintKind::Gingham => {
            rect(&mut s, 0.0, 0.0, r, r, c[1], 1.0);
            rect(&mut s, 0.0, 0.0, r / 2.0, r, c[0], 0.5);
            rect(&mut s, 0.0, 0.0, r, r / 2.0, c[0], 0.5);
            (-r / 4.0, -r / 4.0)
        }
        PrintKind::Plaid => {
            rect(&mut s, 0.0, 0.0, r, r, c[0], 1.0);
            rect(&mut s, r * 0.1, 0.0, r * 0.3, r, c[2], 0.5);
            rect(&mut s, 0.0, r * 0.1, r, r * 0.3, c[2], 0.5);
            for x in [0.6, 0.72] {
                rect(&mut s, r * x, 0.0, r * 0.035, r, c[1], 0.85);
                rect(&mut s, 0.0, r * x, r, r * 0.035, c[1], 0.85);
            }
            (-r * 0.25, -r * 0.25)
        }
        PrintKind::Dots => {
            rect(&mut s, 0.0, 0.0, r, r, c[0], 1.0);
            for (x, y) in [(0.25, 0.25), (0.75, 0.75)] {
                let _ = write!(s, "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\"/>", f(r * x), f(r * y), f(r * 0.16), c[1].css());
            }
            (-r / 4.0, -r / 4.0)
        }
        PrintKind::Camo => {
            rect(&mut s, 0.0, 0.0, r, r, c[0], 1.0);
            let mut rng = Lcg(7);
            for i in 0..18 {
                let col = c[1 + i % (c.len() - 1)];
                let (cx, cy) = (rng.next() * r, rng.next() * r);
                let (rx, ry) = (r * (0.08 + rng.next() * 0.12), r * (0.05 + rng.next() * 0.09));
                let rot = rng.next() * 180.0;
                // Draw the wrapped copies so the tile repeats seamlessly.
                for dx in [-r, 0.0, r] {
                    for dy in [-r, 0.0, r] {
                        let _ = write!(
                            s,
                            "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" transform=\"rotate({} {} {})\" fill=\"{}\"/>",
                            f(cx + dx),
                            f(cy + dy),
                            f(rx),
                            f(ry),
                            f(rot),
                            f(cx + dx),
                            f(cy + dy),
                            col.css()
                        );
                    }
                }
            }
            (0.0, 0.0)
        }
        PrintKind::Florals => {
            rect(&mut s, 0.0, 0.0, r, r, c[0], 1.0);
            let mut rng = Lcg(11);
            // six flowers per tile, placed apart so they never touch
            // flowers are small against the tile: the tile sets how sparse they are
            let size = r * 0.045;
            // Even but irregular, like a ditsy print: each flower goes to the
            // farthest of 20 random spots (Mitchell's best candidate), measured
            // on the repeating tile so the tile edges don't show.
            let dist = |a: (f64, f64), b: (f64, f64)| {
                let dx = (a.0 - b.0).abs().min(r - (a.0 - b.0).abs());
                let dy = (a.1 - b.1).abs().min(r - (a.1 - b.1).abs());
                (dx * dx + dy * dy).sqrt()
            };
            let mut placed: Vec<(f64, f64)> = vec![(rng.next() * r, rng.next() * r)];
            while placed.len() < 24 {
                let best = (0..20)
                    .map(|_| (rng.next() * r, rng.next() * r))
                    .map(|p| (placed.iter().map(|q| dist(p, *q)).fold(f64::INFINITY, f64::min), p))
                    .fold((-1.0, (0.0, 0.0)), |a, b| if b.0 > a.0 { b } else { a });
                placed.push(best.1);
            }
            for (cx, cy) in placed {
                let turn = rng.next() * 72.0;
                let size = size * (0.85 + rng.next() * 0.3);
                // the wrapped copies, so the tile repeats seamlessly
                for dx in [-r, 0.0, r] {
                    for dy in [-r, 0.0, r] {
                        let (x, y) = (cx + dx, cy + dy);
                        // only the copies that reach into the tile
                        let reach = size * 1.2;
                        if x < -reach || x > r + reach || y < -reach || y > r + reach {
                            continue;
                        }
                        for k in 0..5 {
                            let a = (turn + 72.0 * k as f64).to_radians();
                            let _ = write!(s, "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\"/>", f(x + size * 0.62 * a.cos()), f(y + size * 0.62 * a.sin()), f(size * 0.5), c[1].css());
                        }
                        let _ = write!(s, "<circle cx=\"{}\" cy=\"{}\" r=\"{}\" fill=\"{}\"/>", f(x), f(y), f(size * 0.34), c[2].css());
                    }
                }
            }
            (0.0, 0.0)
        }
    };
    format!(
        "<pattern id=\"{id}\" patternUnits=\"userSpaceOnUse\" width=\"{w}\" height=\"{w}\" patternTransform=\"translate({} {}) rotate({}) translate({} {})\">{s}</pattern>",
        f(anchor.x),
        f(anchor.y),
        f(angle),
        f(ox),
        f(oy),
        w = f(r)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::pt;

    fn base() -> Color {
        Color::named("navy").unwrap()
    }

    #[test]
    fn one_colour_gets_a_partner() {
        let cfg = Config::builtin();
        let p = resolve(PrintKind::Stripes, &[base()], 1.0, &base(), &cfg);
        assert_eq!(p.colors.len(), 2);
        assert_eq!(p.colors[1].name, "off white");
    }

    #[test]
    fn camo_has_four_colours_and_is_deterministic() {
        let cfg = Config::builtin();
        let p = resolve(PrintKind::Camo, &[], 1.0, &Color::named("olive").unwrap(), &cfg);
        assert_eq!(p.colors.len(), 4);
        assert_eq!(pattern_def("a", &p, pt(0.0, 0.0), 0.0), pattern_def("a", &p, pt(0.0, 0.0), 0.0));
    }

    #[test]
    fn scale_changes_the_repeat() {
        let cfg = Config::builtin();
        let thin = resolve(PrintKind::Stripes, &[base()], 0.5, &base(), &cfg);
        let thick = resolve(PrintKind::Stripes, &[base()], 2.0, &base(), &cfg);
        assert!(thick.repeat == 4.0 * thin.repeat);
    }
}
