//! The sweep: a stack of bands that rises straight up from the hem, turns
//! a quarter circle toward the armhole and runs into the sleeve, where the
//! sleeve continues it as lengthwise stripes in the body colour plus the
//! band colours.
//!
//! It is a placement print, drawn on the front and back pieces in their own
//! coordinates, so it lands in the same place in every size and meets the
//! sleeve at the armhole. On a fold piece it is mirrored: the two stacks
//! rise on either side of centre front and turn out to both sleeves.

use super::color::Color;
use crate::config::Config;
use crate::geom::{self, pt, Pt};
use crate::pattern::Piece;

#[derive(Clone, Debug, PartialEq)]
pub struct Sweep {
    pub colors: Vec<Color>,
    /// Width of one band, mm.
    pub band: f64,
    /// Share of the half body the bands cover ("almost the whole shirt").
    /// `None` is a single stack of the named colours.
    pub coverage: Option<f64>,
    /// The body colour, used as the spacer between repeats.
    pub ground: Color,
}

impl Sweep {
    /// Colours of the bands, outermost (nearest centre front) first. With
    /// coverage the named colours repeat, separated by a ground-colour band,
    /// until the stack spans that share of the half body.
    fn stack(&self, half_width: f64) -> Vec<Color> {
        match self.coverage {
            None => self.colors.clone(),
            Some(c) => {
                let mut seq = self.colors.clone();
                seq.push(self.ground.clone());
                let count = ((c * half_width / self.band).floor() as usize).max(self.colors.len());
                (0..count).map(|k| seq[k % seq.len()].clone()).collect()
            }
        }
    }
}

/// Band centre-lines for a top's front or back (one side of centre), in
/// the piece's own coordinates, with the colour of each. Empty for any
/// other piece.
pub fn bands(p: &Piece, s: &Sweep, cfg: &Config) -> Vec<(Vec<Pt>, Color)> {
    if !(p.name == "front" || p.name == "back") || !p.has_edge("armhole") {
        return Vec::new();
    }
    let hem = &p.edge("hem").pts;
    let side_x = hem[0].x;
    let colors = s.stack(side_x);
    let n = colors.len() as f64;
    let w = s.band;
    let stack = n * w;
    let hem_y = hem.iter().map(|q| q.y).fold(f64::MIN, f64::max);
    let arm = &p.edge("armhole").pts;
    let under = *arm.last().expect("armhole");
    let arm_x = arm.iter().map(|q| q.x).fold(f64::MIN, f64::max);

    // Vertical run at x0, a quarter circle of radius r, then horizontal at
    // yc: the whole stack enters the armhole just above the underarm.
    // A single stack starts a share of the hem out from centre front; a
    // covering stack ends just inside the side seam.
    let x0 = match s.coverage {
        None => stack / 2.0 + cfg.design["sweep_start"] * side_x,
        Some(_) => side_x - cfg.design["sweep_clear"] - stack / 2.0,
    };
    let yc = under.y - stack / 2.0 - cfg.design["sweep_clear"];
    // The innermost band must still turn on a real radius.
    let r = ((under.x - x0) * cfg.design["sweep_radius"]).min(hem_y - yc - stack).max(stack / 2.0 + w);
    let steps = 32;
    let mut line = vec![pt(x0, hem_y + stack), pt(x0, yc + r)];
    for i in 1..steps {
        let a = std::f64::consts::FRAC_PI_2 * i as f64 / steps as f64;
        line.push(pt(x0 + r - r * a.cos(), yc + r - r * a.sin()));
    }
    line.push(pt(x0 + r, yc));
    line.push(pt(arm_x + stack, yc));
    let line = geom::chain(&[line]);

    colors
        .iter()
        .enumerate()
        // Ground-colour spacers are the body showing through: not drawn.
        .filter(|(_, c)| s.coverage.is_none() || **c != s.ground)
        .map(|(i, c)| {
            // Outermost band on the outside of the turn.
            let off = (i as f64 - (n - 1.0) / 2.0) * w;
            (shift(&line, off), c.clone())
        })
        .collect()
}

/// A polyline moved sideways by `d` (positive: to the right of travel on screen).
pub fn shift(pts: &[Pt], d: f64) -> Vec<Pt> {
    let n = pts.len();
    (0..n)
        .map(|i| {
            let a = pts[i.saturating_sub(1)];
            let b = pts[(i + 1).min(n - 1)];
            let t = (b - a).unit();
            pts[i] + pt(-t.y, t.x) * d
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::test_util::ok;

    fn sweep() -> Sweep {
        let c = |n: &str| Color::named(n).unwrap();
        Sweep { colors: vec![c("wheat"), c("navy"), c("forest green")], band: 24.0, coverage: None, ground: c("white") }
    }

    #[test]
    fn coverage_repeats_the_bands_across_the_body() {
        let cfg = Config::builtin();
        let p = ok("make a t-shirt with long sleeves", &[]);
        let front = p.piece("front");
        let one = bands(front, &sweep(), &cfg).len();
        let s = Sweep { coverage: Some(0.85), ..sweep() };
        let many = bands(front, &s, &cfg);
        assert!(many.len() >= 2 * one, "{} bands", many.len());
        // The innermost band still starts off centre front.
        let min_x = many.iter().map(|(l, _)| l[0].x).fold(f64::MAX, f64::min);
        assert!(min_x > 0.0);
    }

    #[test]
    fn bands_rise_from_the_hem_and_turn_into_the_armhole() {
        let cfg = Config::builtin();
        let p = ok("make a t-shirt with long sleeves", &[]);
        let front = p.piece("front");
        let b = bands(front, &sweep(), &cfg);
        assert_eq!(b.len(), 3);
        let hem_y = front.edge("hem").pts.last().unwrap().y;
        let under = *front.edge("armhole").pts.last().unwrap();
        for (line, _) in &b {
            // Starts below the hem going straight up, ends past the armhole going sideways.
            assert!(line[0].y > hem_y && (line[0].x - line[1].x).abs() < 1.0);
            let (a, z) = (line[line.len() - 2], line[line.len() - 1]);
            assert!(z.x > under.x && (a.y - z.y).abs() < 1.0);
            assert!(z.y < under.y);
        }
    }

    #[test]
    fn only_on_body_pieces() {
        let cfg = Config::builtin();
        let p = ok("make a t-shirt", &[]);
        assert!(bands(p.piece("sleeve"), &sweep(), &cfg).is_empty());
    }
}
