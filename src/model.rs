//! What the parser produces: a garment specification. Everything here says
//! *what* was asked for; how to build it is decided by the rules and the search.

use crate::config::{Config, FabricConfig, Measurements};
use crate::lexicon::Modifier;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Garment {
    Tshirt,
    Hoodie,
    Sweatshirt,
    Shirt,
    Pants,
}

impl Garment {
    pub const ALL: [Garment; 5] = [Garment::Tshirt, Garment::Hoodie, Garment::Sweatshirt, Garment::Shirt, Garment::Pants];

    pub fn key(self) -> &'static str {
        match self {
            Garment::Tshirt => "tshirt",
            Garment::Hoodie => "hoodie",
            Garment::Sweatshirt => "sweatshirt",
            Garment::Shirt => "shirt",
            Garment::Pants => "pants",
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Garment::Tshirt => "t-shirt",
            Garment::Hoodie => "hoodie",
            Garment::Sweatshirt => "sweatshirt",
            Garment::Shirt => "button shirt",
            Garment::Pants => "pants",
        }
    }
    pub fn is_top(self) -> bool {
        self != Garment::Pants
    }
    pub fn class(self) -> &'static str {
        if self.is_top() {
            "top"
        } else {
            "pants"
        }
    }
    /// Knit tops pull over the head; the shirt opens down the front.
    pub fn pulls_over_head(self) -> bool {
        matches!(self, Garment::Tshirt | Garment::Hoodie | Garment::Sweatshirt)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Fit {
    Fitted,
    Regular,
    Loose,
    Oversized,
}

impl Fit {
    pub const ALL: [Fit; 4] = [Fit::Fitted, Fit::Regular, Fit::Loose, Fit::Oversized];

    /// Fit names differ by garment class: fitted pants are "tight", loose
    /// ones "baggy".
    pub fn key(self, g: Garment) -> &'static str {
        match (g.is_top(), self) {
            (true, Fit::Fitted) => "fitted",
            (true, Fit::Regular) => "regular",
            (true, Fit::Loose) => "loose",
            (true, Fit::Oversized) => "oversized",
            (false, Fit::Fitted) => "tight",
            (false, Fit::Regular) => "regular",
            (false, _) => "baggy",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Size {
    S,
    M,
    L,
    XL,
}

impl Size {
    pub const ALL: [Size; 4] = [Size::S, Size::M, Size::L, Size::XL];
    pub fn key(self) -> &'static str {
        match self {
            Size::S => "S",
            Size::M => "M",
            Size::L => "L",
            Size::XL => "XL",
        }
    }
    pub fn parse(s: &str) -> Option<Size> {
        Size::ALL.into_iter().find(|z| z.key().eq_ignore_ascii_case(s))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SleeveKind {
    Short,
    Long,
}

/// Something the sentence changed by an amount.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EditKind {
    SleeveLength,
    SleeveWidth,
    NeckWidth,
    NeckDepth,
    BodyLength,
    ChestWidth,
    SeatWidth,
    LegLength,
    LegWidth,
    Rise,
}

impl EditKind {
    /// Keys of `[edits]` (the defaults), including "crop".
    pub const KEYS: &'static [&'static str] = &[
        "sleeve_length", "sleeve_width", "neck_width", "neck_depth", "body_length", "crop",
        "chest_width", "seat_width", "leg_length", "leg_width", "rise",
    ];

    /// Also the name of the rule that carries it out.
    pub fn key(self) -> &'static str {
        match self {
            EditKind::SleeveLength => "sleeve_length",
            EditKind::SleeveWidth => "sleeve_width",
            EditKind::NeckWidth => "neck_width",
            EditKind::NeckDepth => "neck_depth",
            EditKind::BodyLength => "body_length",
            EditKind::ChestWidth => "chest_width",
            EditKind::SeatWidth => "seat_width",
            EditKind::LegLength => "leg_length",
            EditKind::LegWidth => "leg_width",
            EditKind::Rise => "rise",
        }
    }
    pub fn for_tops(self) -> bool {
        !matches!(self, EditKind::SeatWidth | EditKind::LegLength | EditKind::LegWidth | EditKind::Rise)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Edit {
    pub kind: EditKind,
    /// Signed change, mm. Positive lengthens, widens, lowers a neckline, raises a waist.
    pub mm: f64,
    /// The sentence gave the amount (otherwise it's the `[edits]` default).
    pub given: bool,
    pub words: String,
}

/// A value plus the words that asked for it.
#[derive(Clone, Debug, PartialEq)]
pub struct Worded<T> {
    pub value: T,
    pub words: String,
}

/// Words that narrow a rule to some of its variants ("rib cuffs", "a lined hood").
#[derive(Clone, Debug, PartialEq)]
pub struct Pin {
    pub rule: &'static str,
    pub variants: &'static [&'static str],
    pub words: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    pub sentence: String,
    pub garment: Worded<Garment>,
    pub size: Size,
    pub fit: Option<Worded<Fit>>,
    pub sleeves: Option<Worded<SleeveKind>>,
    pub hood: Option<Worded<bool>>,
    pub pocket: Option<Worded<bool>>,
    pub pins: Vec<Pin>,
    pub edits: Vec<Edit>,
    pub mods: Vec<Modifier>,
    pub design: crate::design::DesignSpec,
}

impl Spec {
    pub fn g(&self) -> Garment {
        self.garment.value
    }
    pub fn fit(&self) -> Fit {
        self.fit.as_ref().map_or(Fit::Regular, |f| f.value)
    }
    pub fn measurements<'a>(&self, cfg: &'a Config) -> &'a Measurements {
        &cfg.sizes[self.size.key()]
    }
    pub fn fabric_name<'a>(&self, cfg: &'a Config) -> &'a str {
        let g = self.g();
        let fit = &cfg.fits[g.class()][self.fit().key(g)];
        fit.fabric.as_deref().unwrap_or(&cfg.garments[g.key()].fabric)
    }
    pub fn fabric<'a>(&self, cfg: &'a Config) -> &'a FabricConfig {
        cfg.fabric(self.fabric_name(cfg))
    }
    pub fn sleeves(&self, cfg: &Config) -> SleeveKind {
        match &self.sleeves {
            Some(s) => s.value,
            None => match cfg.garments[self.g().key()].sleeves.as_deref() {
                Some("short") => SleeveKind::Short,
                _ => SleeveKind::Long,
            },
        }
    }
    pub fn has_hood(&self, cfg: &Config) -> bool {
        self.g().is_top() && self.hood.as_ref().map_or(cfg.garments[self.g().key()].hood, |h| h.value)
    }
    pub fn has_pocket(&self, cfg: &Config) -> bool {
        self.g().is_top() && self.pocket.as_ref().map_or(cfg.garments[self.g().key()].pocket, |p| p.value)
    }
    /// Words for a feature: the sentence's, or the garment name when it's a default.
    pub fn feature_words(&self, w: &Option<Worded<bool>>) -> String {
        w.as_ref().map_or_else(|| self.garment.words.clone(), |w| w.words.clone())
    }
    pub fn edit(&self, kind: EditKind) -> Option<Edit> {
        let mut found: Option<Edit> = None;
        for e in self.edits.iter().filter(|e| e.kind == kind) {
            found = Some(match found {
                None => e.clone(),
                Some(f) => Edit { mm: f.mm + e.mm, given: f.given || e.given, words: format!("{}; {}", f.words, e.words), kind },
            });
        }
        found
    }
    pub fn pin(&self, rule: &str) -> Option<&Pin> {
        self.pins.iter().rev().find(|p| p.rule == rule)
    }
}
