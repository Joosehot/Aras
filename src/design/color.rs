//! Colours: a closed, named garment palette (dyed-fabric shades, not screen
//! primaries), "light"/"dark" shades, and the contrast arithmetic the design
//! rules use to pick trims that read against the body.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Rgb(pub u8, pub u8, pub u8);

impl Rgb {
    pub const fn hex(v: u32) -> Rgb {
        Rgb((v >> 16) as u8, (v >> 8) as u8, v as u8)
    }
    pub fn css(self) -> String {
        format!("#{:02x}{:02x}{:02x}", self.0, self.1, self.2)
    }
    /// Straight mix toward `o`: `t = 0` is self, `1` is `o`.
    pub fn mix(self, o: Rgb, t: f64) -> Rgb {
        let m = |a: u8, b: u8| (a as f64 + (b as f64 - a as f64) * t).round().clamp(0.0, 255.0) as u8;
        Rgb(m(self.0, o.0), m(self.1, o.1), m(self.2, o.2))
    }
    pub fn lighter(self, t: f64) -> Rgb {
        self.mix(Rgb(255, 255, 255), t)
    }
    pub fn darker(self, t: f64) -> Rgb {
        self.mix(Rgb(0, 0, 0), t)
    }
    /// WCAG relative luminance, 0 (black) to 1 (white).
    pub fn luminance(self) -> f64 {
        let ch = |c: u8| {
            let c = c as f64 / 255.0;
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * ch(self.0) + 0.7152 * ch(self.1) + 0.0722 * ch(self.2)
    }
    /// WCAG contrast ratio, 1 to 21.
    pub fn contrast(self, o: Rgb) -> f64 {
        let (a, b) = (self.luminance(), o.luminance());
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }
}

/// A named colour as the sentence said it ("dark green").
#[derive(Clone, Debug, PartialEq)]
pub struct Color {
    pub name: String,
    pub rgb: Rgb,
}

impl Color {
    pub fn new(name: impl Into<String>, rgb: Rgb) -> Color {
        Color { name: name.into(), rgb }
    }
    pub fn named(name: &str) -> Option<Color> {
        COLORS.iter().find(|(n, _)| *n == name).map(|(n, v)| Color::new(*n, Rgb::hex(*v)))
    }
    /// "light"/"dark" + colour.
    pub fn shaded(&self, shade: f64) -> Color {
        if shade > 0.0 {
            Color::new(format!("light {}", self.name), self.rgb.lighter(shade))
        } else if shade < 0.0 {
            Color::new(format!("dark {}", self.name), self.rgb.darker(-shade))
        } else {
            self.clone()
        }
    }
    /// Title-case label for swatches: "Heather Grey".
    pub fn title(&self) -> String {
        self.name
            .split(' ')
            .map(|w| {
                let mut c = w.chars();
                c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// Garment-dye shades. Names are also lexicon phrases.
pub const COLORS: &[(&str, u32)] = &[
    ("black", 0x1f1f21),
    ("white", 0xf7f6f2),
    ("off white", 0xf0ece1),
    ("cream", 0xefe5cf),
    ("ivory", 0xf3edda),
    ("ecru", 0xe6d9bd),
    ("grey", 0x9a9b9d),
    ("gray", 0x9a9b9d),
    ("heather grey", 0xb5b3ae),
    ("heather gray", 0xb5b3ae),
    ("charcoal", 0x3e4044),
    ("navy", 0x1f2a44),
    ("blue", 0x2f5fa7),
    ("royal blue", 0x2548a8),
    ("cobalt", 0x1f4fbf),
    ("sky blue", 0x8ec5e8),
    ("baby blue", 0xbcd7ef),
    ("denim", 0x4a6a8f),
    ("teal", 0x1f6f73),
    ("turquoise", 0x3fb8b0),
    ("green", 0x3a7d44),
    ("forest green", 0x2b4a33),
    ("olive", 0x6b6b3a),
    ("sage", 0x9caf88),
    ("mint", 0xa8dcc0),
    ("khaki", 0xc3b091),
    ("beige", 0xd8c8a8),
    ("tan", 0xc19a6b),
    ("sand", 0xd6c29a),
    ("camel", 0xb8875a),
    ("brown", 0x6b4a34),
    ("chocolate", 0x4a2f23),
    ("rust", 0xa4502e),
    ("orange", 0xe0782f),
    ("burnt orange", 0xc1571f),
    ("mustard", 0xd4a93a),
    ("wheat", 0xd9b66f),
    ("yellow", 0xf2d24b),
    ("red", 0xc1272d),
    ("burgundy", 0x6d1f2e),
    ("maroon", 0x6a1b24),
    ("wine", 0x5e1a2a),
    ("pink", 0xf2a7b8),
    ("hot pink", 0xe0457b),
    ("blush", 0xe8c1c0),
    ("coral", 0xf07a62),
    ("peach", 0xf6b995),
    ("lavender", 0xb9a8d6),
    ("lilac", 0xc8a2c8),
    ("purple", 0x6a3d8f),
    ("plum", 0x5b2a4b),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_round_trips() {
        assert_eq!(Rgb::hex(0x1f2a44).css(), "#1f2a44");
    }

    #[test]
    fn contrast_is_symmetric_and_bounded() {
        let (b, w) = (Color::named("black").unwrap().rgb, Color::named("white").unwrap().rgb);
        assert!(b.contrast(w) > 15.0);
        assert_eq!(b.contrast(w), w.contrast(b));
        assert!((b.contrast(b) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn shades_move_luminance() {
        let g = Color::named("green").unwrap();
        assert!(g.shaded(0.35).rgb.luminance() > g.rgb.luminance());
        assert!(g.shaded(-0.35).rgb.luminance() < g.rgb.luminance());
        assert_eq!(g.shaded(-0.35).name, "dark green");
    }
}
