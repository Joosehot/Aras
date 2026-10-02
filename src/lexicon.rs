//! The lexicon: words and phrases -> tokens. This is Aras's music theory: a
//! closed, hand-written vocabulary of garments, parts, changes and amounts.
//! Anything outside it is an error (or ignored with `--lenient`), never a guess.

use crate::design::color::COLORS;
use crate::design::print::PrintKind;
use crate::model::{DressLength, Fit, Garment, Size, SleeveKind};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Part {
    Sleeves,
    Collar,
    Body,
    Hood,
    Pocket,
    Cuffs,
    Legs,
    Waist,
    Chest,
    Seat,
    Lining,
    Drawstring,
    Buttons,
    Stitching,
    Rib,
}

impl Part {
    pub fn name(self) -> &'static str {
        match self {
            Part::Sleeves => "sleeves",
            Part::Collar => "collar",
            Part::Body => "body",
            Part::Hood => "hood",
            Part::Pocket => "pocket",
            Part::Cuffs => "cuffs",
            Part::Legs => "legs",
            Part::Waist => "waist",
            Part::Chest => "chest",
            Part::Seat => "seat",
            Part::Lining => "lining",
            Part::Drawstring => "drawstrings",
            Part::Buttons => "buttons",
            Part::Stitching => "stitching",
            Part::Rib => "rib trims",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Verb {
    Make,
    Add,
    Remove,
    Lengthen,
    Shorten,
    Crop,
    Widen,
    Narrow,
    Lower,
    Raise,
}

impl Verb {
    /// Verbs that change a measurement (the rest add, remove or start).
    pub fn is_edit(self) -> bool {
        !matches!(self, Verb::Make | Verb::Add | Verb::Remove)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Modifier {
    Neatly,
    Simply,
    Cheaply,
}

/// Modifiers that shift the profile; each needs `[modifiers.<key>]`.
pub const MODIFIER_KEYS: &[&str] = &["neatly", "simply", "cheaply"];

impl Modifier {
    pub fn key(self) -> &'static str {
        match self {
            Modifier::Neatly => "neatly",
            Modifier::Simply => "simply",
            Modifier::Cheaply => "cheaply",
        }
    }
}

/// Words that narrow a rule's variants, optionally turning a feature on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PinDef {
    pub rule: &'static str,
    pub variants: &'static [&'static str],
    pub implies: Option<Part>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tok {
    Garment(Garment),
    Part(Part),
    Verb(Verb),
    /// A comparative: "longer" is Lengthen, "wider" is Widen, ...
    Cmp(Verb),
    Fit(Fit),
    Mod(Modifier),
    Num(f64),
    /// A unit, as millimetres per unit.
    Unit(f64),
    Size(Size),
    Sleeves(SleeveKind),
    DressLen(DressLength),
    Pin(PinDef),
    /// A palette colour, by name.
    Color(&'static str),
    /// "light" (+1) or "dark" (-1).
    Shade(f64),
    Print(PrintKind),
    /// "thin" (-1) or "thick" (+1) print.
    Scale(f64),
    /// How much of the garment a placed print covers, 0..1.
    Coverage(f64),
    Half,
    Article,
    With,
    Without,
    No,
    And,
    Comma,
    Then,
    Stop,
    Filler,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub tok: Tok,
    pub text: String,
}

const INCH: f64 = 25.4;

const fn pin(rule: &'static str, variants: &'static [&'static str], implies: Option<Part>) -> Tok {
    Tok::Pin(PinDef { rule, variants, implies })
}

/// Every phrase, longest first within the matcher. Kept as one table so
/// `--vocabulary` and "did you mean" see exactly what the parser sees.
pub const PHRASES: &[(&str, Tok)] = &[
    // garments
    ("t shirt", Tok::Garment(Garment::Tshirt)),
    ("tshirt", Tok::Garment(Garment::Tshirt)),
    ("tee shirt", Tok::Garment(Garment::Tshirt)),
    ("tee", Tok::Garment(Garment::Tshirt)),
    ("hoodie", Tok::Garment(Garment::Hoodie)),
    ("hoody", Tok::Garment(Garment::Hoodie)),
    ("hooded sweatshirt", Tok::Garment(Garment::Hoodie)),
    ("hooded jumper", Tok::Garment(Garment::Hoodie)),
    ("sweatshirt", Tok::Garment(Garment::Sweatshirt)),
    ("sweater", Tok::Garment(Garment::Sweatshirt)),
    ("jumper", Tok::Garment(Garment::Sweatshirt)),
    ("crewneck", Tok::Garment(Garment::Sweatshirt)),
    ("button up shirt", Tok::Garment(Garment::Shirt)),
    ("button down shirt", Tok::Garment(Garment::Shirt)),
    ("button up", Tok::Garment(Garment::Shirt)),
    ("button down", Tok::Garment(Garment::Shirt)),
    ("dress shirt", Tok::Garment(Garment::Shirt)),
    ("shirt", Tok::Garment(Garment::Shirt)),
    ("pants", Tok::Garment(Garment::Pants)),
    ("trousers", Tok::Garment(Garment::Pants)),
    ("slacks", Tok::Garment(Garment::Pants)),
    ("chinos", Tok::Garment(Garment::Pants)),
    ("sundress", Tok::Garment(Garment::Dress)),
    ("dresses", Tok::Garment(Garment::Dress)),
    ("dress", Tok::Garment(Garment::Dress)),
    ("frock", Tok::Garment(Garment::Dress)),
    ("sleeveless", Tok::Sleeves(SleeveKind::None)),
    ("mini", Tok::DressLen(DressLength::Mini)),
    ("above the knee", Tok::DressLen(DressLength::Mini)),
    ("knee length", Tok::DressLen(DressLength::Knee)),
    ("knee", Tok::DressLen(DressLength::Knee)),
    ("midi", Tok::DressLen(DressLength::Midi)),
    ("calf length", Tok::DressLen(DressLength::Midi)),
    ("maxi", Tok::DressLen(DressLength::Maxi)),
    ("floor length", Tok::DressLen(DressLength::Maxi)),
    ("ankle length", Tok::DressLen(DressLength::Maxi)),
    // dress silhouettes, closures and finishes
    ("a line", pin("skirt", &["a_line"], None)),
    ("aline", pin("skirt", &["a_line"], None)),
    ("flared", pin("skirt", &["a_line"], None)),
    ("shift", pin("skirt", &["shift"], None)),
    ("sheath", pin("skirt", &["shift"], None)),
    ("fit and flare", pin("skirt", &["gathered"], None)),
    ("skater", pin("skirt", &["gathered"], None)),
    ("gathered skirt", pin("skirt", &["gathered"], None)),
    ("gathered", pin("skirt", &["gathered"], None)),
    ("back zip", pin("closure", &["back_zip"], None)),
    ("invisible zip", pin("closure", &["back_zip"], None)),
    ("zip", pin("closure", &["back_zip"], None)),
    ("pull on", pin("closure", &["pull_on"], None)),
    ("no zip", pin("closure", &["pull_on"], None)),
    ("neck facing", pin("neck_finish", &["facing"], None)),
    ("faced neckline", pin("neck_finish", &["facing"], None)),
    ("bias binding", pin("neck_finish", &["binding"], None)),
    ("armhole facings", pin("armhole_finish", &["facing"], None)),
    ("armhole facing", pin("armhole_finish", &["facing"], None)),
    ("armhole binding", pin("armhole_finish", &["binding"], None)),
    ("bound armholes", pin("armhole_finish", &["binding"], None)),
    // fits
    ("skinny", Tok::Fit(Fit::Fitted)),
    ("tight", Tok::Fit(Fit::Fitted)),
    ("slim", Tok::Fit(Fit::Fitted)),
    ("fitted", Tok::Fit(Fit::Fitted)),
    ("regular", Tok::Fit(Fit::Regular)),
    ("normal", Tok::Fit(Fit::Regular)),
    ("classic", Tok::Fit(Fit::Regular)),
    ("straight leg", Tok::Fit(Fit::Regular)),
    ("loose", Tok::Fit(Fit::Loose)),
    ("relaxed", Tok::Fit(Fit::Loose)),
    ("baggy", Tok::Fit(Fit::Loose)),
    ("wide leg", Tok::Fit(Fit::Loose)),
    ("oversized", Tok::Fit(Fit::Oversized)),
    ("boxy", Tok::Fit(Fit::Oversized)),
    // sleeve kinds
    ("long", Tok::Sleeves(SleeveKind::Long)),
    ("short", Tok::Sleeves(SleeveKind::Short)),
    // pins (checked before the plain parts they contain)
    ("rib cuffs", pin("sleeve_finish", &["rib_cuff"], None)),
    ("ribbed cuffs", pin("sleeve_finish", &["rib_cuff"], None)),
    ("barrel cuffs", pin("sleeve_finish", &["barrel_cuff"], None)),
    ("hemmed sleeves", pin("sleeve_finish", &["hemmed", "hemmed_wide"], None)),
    ("lined hood", pin("hood_lining", &["lined"], Some(Part::Hood))),
    ("double hood", pin("hood_lining", &["lined"], Some(Part::Hood))),
    ("unlined hood", pin("hood_lining", &["single"], Some(Part::Hood))),
    ("three piece hood", pin("hood", &["three_piece"], Some(Part::Hood))),
    ("3 piece hood", pin("hood", &["three_piece"], Some(Part::Hood))),
    ("two piece hood", pin("hood", &["two_piece"], Some(Part::Hood))),
    ("2 piece hood", pin("hood", &["two_piece"], Some(Part::Hood))),
    ("collar stand", pin("collar", &["with_stand"], None)),
    ("camp collar", pin("collar", &["one_piece"], None)),
    ("convertible collar", pin("collar", &["one_piece"], None)),
    ("one piece collar", pin("collar", &["one_piece"], None)),
    ("shirttail hem", pin("body_hem", &["shirttail"], None)),
    ("shirttail", pin("body_hem", &["shirttail"], None)),
    ("shirt tail", pin("body_hem", &["shirttail"], None)),
    ("curved hem", pin("body_hem", &["shirttail"], None)),
    ("straight hem", pin("body_hem", &["straight", "hemmed"], None)),
    ("rib hem", pin("body_hem", &["rib_band"], None)),
    ("ribbed hem", pin("body_hem", &["rib_band"], None)),
    ("hem band", pin("body_hem", &["rib_band"], None)),
    ("rib neckband", pin("neck_finish", &["rib_band"], None)),
    ("ribbed neckband", pin("neck_finish", &["rib_band"], None)),
    ("self fabric neckband", pin("neck_finish", &["self_band"], None)),
    ("neckband", pin("neck_finish", &["rib_band", "self_band"], None)),
    ("neck binding", pin("neck_finish", &["binding"], None)),
    ("binding", pin("neck_finish", &["binding"], None)),
    ("elastic waistband", pin("waist", &["elastic"], None)),
    ("elastic waist", pin("waist", &["elastic"], None)),
    ("drawstring waist", pin("waist", &["elastic"], None)),
    ("zip fly", pin("waist", &["zip_fly"], None)),
    ("zipper", pin("waist", &["zip_fly"], None)),
    ("fly", pin("waist", &["zip_fly"], None)),
    // parts
    ("sleeves", Tok::Part(Part::Sleeves)),
    ("sleeve", Tok::Part(Part::Sleeves)),
    ("sleeved", Tok::Part(Part::Sleeves)),
    ("arms", Tok::Part(Part::Sleeves)),
    ("neck hole", Tok::Part(Part::Collar)),
    ("neckhole", Tok::Part(Part::Collar)),
    ("neckline", Tok::Part(Part::Collar)),
    ("neck", Tok::Part(Part::Collar)),
    ("collar", Tok::Part(Part::Collar)),
    ("body", Tok::Part(Part::Body)),
    ("torso", Tok::Part(Part::Body)),
    ("hemline", Tok::Part(Part::Body)),
    ("hem", Tok::Part(Part::Body)),
    ("hood", Tok::Part(Part::Hood)),
    ("kangaroo pocket", Tok::Part(Part::Pocket)),
    ("front pocket", Tok::Part(Part::Pocket)),
    ("pockets", Tok::Part(Part::Pocket)),
    ("pocket", Tok::Part(Part::Pocket)),
    ("cuffs", Tok::Part(Part::Cuffs)),
    ("cuff", Tok::Part(Part::Cuffs)),
    ("pant legs", Tok::Part(Part::Legs)),
    ("trouser legs", Tok::Part(Part::Legs)),
    ("legs", Tok::Part(Part::Legs)),
    ("leg", Tok::Part(Part::Legs)),
    ("inseam", Tok::Part(Part::Legs)),
    ("waistline", Tok::Part(Part::Waist)),
    ("waistband", Tok::Part(Part::Waist)),
    ("waist", Tok::Part(Part::Waist)),
    ("rise", Tok::Part(Part::Waist)),
    ("chest", Tok::Part(Part::Chest)),
    ("bust", Tok::Part(Part::Chest)),
    ("hips", Tok::Part(Part::Seat)),
    ("hip", Tok::Part(Part::Seat)),
    ("seat", Tok::Part(Part::Seat)),
    // verbs
    ("give me", Tok::Verb(Verb::Make)),
    ("i want", Tok::Verb(Verb::Make)),
    ("i need", Tok::Verb(Verb::Make)),
    ("make", Tok::Verb(Verb::Make)),
    ("create", Tok::Verb(Verb::Make)),
    ("draft", Tok::Verb(Verb::Make)),
    ("design", Tok::Verb(Verb::Make)),
    ("sew", Tok::Verb(Verb::Make)),
    ("generate", Tok::Verb(Verb::Make)),
    ("add", Tok::Verb(Verb::Add)),
    ("put", Tok::Verb(Verb::Add)),
    ("include", Tok::Verb(Verb::Add)),
    ("attach", Tok::Verb(Verb::Add)),
    ("remove", Tok::Verb(Verb::Remove)),
    ("delete", Tok::Verb(Verb::Remove)),
    ("take off", Tok::Verb(Verb::Remove)),
    ("get rid of", Tok::Verb(Verb::Remove)),
    ("lengthen", Tok::Verb(Verb::Lengthen)),
    ("extend", Tok::Verb(Verb::Lengthen)),
    ("shorten", Tok::Verb(Verb::Shorten)),
    ("crop", Tok::Verb(Verb::Crop)),
    ("widen", Tok::Verb(Verb::Widen)),
    ("broaden", Tok::Verb(Verb::Widen)),
    ("enlarge", Tok::Verb(Verb::Widen)),
    ("loosen", Tok::Verb(Verb::Widen)),
    ("narrow", Tok::Verb(Verb::Narrow)),
    ("taper", Tok::Verb(Verb::Narrow)),
    ("tighten", Tok::Verb(Verb::Narrow)),
    ("lower", Tok::Verb(Verb::Lower)),
    ("deepen", Tok::Verb(Verb::Lower)),
    ("raise", Tok::Verb(Verb::Raise)),
    // comparatives
    ("longer", Tok::Cmp(Verb::Lengthen)),
    ("shorter", Tok::Cmp(Verb::Shorten)),
    ("wider", Tok::Cmp(Verb::Widen)),
    ("looser", Tok::Cmp(Verb::Widen)),
    ("bigger", Tok::Cmp(Verb::Widen)),
    ("narrower", Tok::Cmp(Verb::Narrow)),
    ("tighter", Tok::Cmp(Verb::Narrow)),
    ("slimmer", Tok::Cmp(Verb::Narrow)),
    ("smaller", Tok::Cmp(Verb::Narrow)),
    ("deeper", Tok::Cmp(Verb::Lower)),
    ("higher", Tok::Cmp(Verb::Raise)),
    // modifiers
    ("neatly", Tok::Mod(Modifier::Neatly)),
    ("neat", Tok::Mod(Modifier::Neatly)),
    ("tailored", Tok::Mod(Modifier::Neatly)),
    ("professionally", Tok::Mod(Modifier::Neatly)),
    ("professional", Tok::Mod(Modifier::Neatly)),
    ("properly", Tok::Mod(Modifier::Neatly)),
    ("nicely", Tok::Mod(Modifier::Neatly)),
    ("high quality", Tok::Mod(Modifier::Neatly)),
    ("simply", Tok::Mod(Modifier::Simply)),
    ("simple", Tok::Mod(Modifier::Simply)),
    ("easy", Tok::Mod(Modifier::Simply)),
    ("easily", Tok::Mod(Modifier::Simply)),
    ("beginner", Tok::Mod(Modifier::Simply)),
    ("quickly", Tok::Mod(Modifier::Simply)),
    ("quick", Tok::Mod(Modifier::Simply)),
    ("cheaply", Tok::Mod(Modifier::Cheaply)),
    ("cheap", Tok::Mod(Modifier::Cheaply)),
    ("economically", Tok::Mod(Modifier::Cheaply)),
    ("economical", Tok::Mod(Modifier::Cheaply)),
    ("budget", Tok::Mod(Modifier::Cheaply)),
    ("saving fabric", Tok::Mod(Modifier::Cheaply)),
    ("fabric saving", Tok::Mod(Modifier::Cheaply)),
    // design
    ("light", Tok::Shade(1.0)),
    ("pale", Tok::Shade(1.0)),
    ("dark", Tok::Shade(-1.0)),
    ("deep", Tok::Shade(-1.0)),
    ("vertical stripes", Tok::Print(PrintKind::VerticalStripes)),
    ("vertically striped", Tok::Print(PrintKind::VerticalStripes)),
    ("pinstriped", Tok::Print(PrintKind::Pinstripes)),
    ("pinstripes", Tok::Print(PrintKind::Pinstripes)),
    ("pinstripe", Tok::Print(PrintKind::Pinstripes)),
    ("striped", Tok::Print(PrintKind::Stripes)),
    ("stripes", Tok::Print(PrintKind::Stripes)),
    ("stripe", Tok::Print(PrintKind::Stripes)),
    ("breton", Tok::Print(PrintKind::Stripes)),
    ("gingham", Tok::Print(PrintKind::Gingham)),
    ("checkered", Tok::Print(PrintKind::Gingham)),
    ("checked", Tok::Print(PrintKind::Gingham)),
    ("checks", Tok::Print(PrintKind::Gingham)),
    ("check", Tok::Print(PrintKind::Gingham)),
    ("plaid", Tok::Print(PrintKind::Plaid)),
    ("tartan", Tok::Print(PrintKind::Plaid)),
    ("flannel", Tok::Print(PrintKind::Plaid)),
    ("polka dots", Tok::Print(PrintKind::Dots)),
    ("polka dot", Tok::Print(PrintKind::Dots)),
    ("polka dotted", Tok::Print(PrintKind::Dots)),
    ("dotted", Tok::Print(PrintKind::Dots)),
    ("spotted", Tok::Print(PrintKind::Dots)),
    ("dots", Tok::Print(PrintKind::Dots)),
    ("from the hem into the sleeves", Tok::Print(PrintKind::Sweep)),
    ("from the hem to the sleeves", Tok::Print(PrintKind::Sweep)),
    ("from the hem into the arms", Tok::Print(PrintKind::Sweep)),
    ("sweeping", Tok::Print(PrintKind::Sweep)),
    ("covering almost the whole shirt", Tok::Coverage(0.85)),
    ("covering almost the whole t shirt", Tok::Coverage(0.85)),
    ("covering the whole shirt", Tok::Coverage(0.95)),
    ("covering the whole t shirt", Tok::Coverage(0.95)),
    ("almost the whole shirt", Tok::Coverage(0.85)),
    ("almost the whole t shirt", Tok::Coverage(0.85)),
    ("the whole shirt", Tok::Coverage(0.95)),
    ("the whole t shirt", Tok::Coverage(0.95)),
    ("almost all over", Tok::Coverage(0.85)),
    ("swoosh", Tok::Print(PrintKind::Sweep)),
    ("bands", Tok::Print(PrintKind::Stripes)),
    ("flowers", Tok::Print(PrintKind::Florals)),
    ("flower", Tok::Print(PrintKind::Florals)),
    ("floral", Tok::Print(PrintKind::Florals)),
    ("florals", Tok::Print(PrintKind::Florals)),
    ("flowered", Tok::Print(PrintKind::Florals)),
    ("flowery", Tok::Print(PrintKind::Florals)),
    ("daisies", Tok::Print(PrintKind::Florals)),
    ("tiny", Tok::Scale(-1.0)),
    ("ditsy", Tok::Scale(-1.0)),
    ("little", Tok::Scale(-1.0)),
    ("camouflage", Tok::Print(PrintKind::Camo)),
    ("camo", Tok::Print(PrintKind::Camo)),
    ("thin", Tok::Scale(-1.0)),
    ("fine", Tok::Scale(-1.0)),
    ("thick", Tok::Scale(1.0)),
    ("bold", Tok::Scale(1.0)),
    ("chunky", Tok::Scale(1.0)),
    ("hood lining", Tok::Part(Part::Lining)),
    ("lining", Tok::Part(Part::Lining)),
    ("drawstrings", Tok::Part(Part::Drawstring)),
    ("drawstring", Tok::Part(Part::Drawstring)),
    ("drawcords", Tok::Part(Part::Drawstring)),
    ("strings", Tok::Part(Part::Drawstring)),
    ("buttons", Tok::Part(Part::Buttons)),
    ("button", Tok::Part(Part::Buttons)),
    ("topstitching", Tok::Part(Part::Stitching)),
    ("stitching", Tok::Part(Part::Stitching)),
    ("thread", Tok::Part(Part::Stitching)),
    ("ribbing", Tok::Part(Part::Rib)),
    ("rib", Tok::Part(Part::Rib)),
    ("trims", Tok::Part(Part::Rib)),
    ("trim", Tok::Part(Part::Rib)),
    ("colour", Tok::Filler),
    ("color", Tok::Filler),
    ("coloured", Tok::Filler),
    ("colored", Tok::Filler),
    ("print", Tok::Filler),
    ("printed", Tok::Filler),
    ("pattern", Tok::Filler),
    ("all over", Tok::Coverage(0.95)),
    // units
    ("inches", Tok::Unit(INCH)),
    ("inch", Tok::Unit(INCH)),
    ("centimeters", Tok::Unit(10.0)),
    ("centimetres", Tok::Unit(10.0)),
    ("centimeter", Tok::Unit(10.0)),
    ("centimetre", Tok::Unit(10.0)),
    ("cm", Tok::Unit(10.0)),
    ("millimeters", Tok::Unit(1.0)),
    ("millimetres", Tok::Unit(1.0)),
    ("mm", Tok::Unit(1.0)),
    // number words
    ("one", Tok::Num(1.0)),
    ("two", Tok::Num(2.0)),
    ("three", Tok::Num(3.0)),
    ("four", Tok::Num(4.0)),
    ("five", Tok::Num(5.0)),
    ("six", Tok::Num(6.0)),
    ("seven", Tok::Num(7.0)),
    ("eight", Tok::Num(8.0)),
    ("nine", Tok::Num(9.0)),
    ("ten", Tok::Num(10.0)),
    ("eleven", Tok::Num(11.0)),
    ("twelve", Tok::Num(12.0)),
    ("and a half", Tok::Half),
    ("a half", Tok::Half),
    ("half", Tok::Half),
    // sizes
    ("size xl", Tok::Size(Size::XL)),
    ("size l", Tok::Size(Size::L)),
    ("size m", Tok::Size(Size::M)),
    ("size s", Tok::Size(Size::S)),
    ("extra large", Tok::Size(Size::XL)),
    ("x large", Tok::Size(Size::XL)),
    ("xl", Tok::Size(Size::XL)),
    ("large", Tok::Size(Size::L)),
    ("medium", Tok::Size(Size::M)),
    ("small", Tok::Size(Size::S)),
    // connectives
    ("and then", Tok::Then),
    ("after that", Tok::Then),
    ("then", Tok::Then),
    ("and", Tok::And),
    ("also", Tok::And),
    ("plus", Tok::And),
    ("that has", Tok::With),
    ("having", Tok::With),
    ("with", Tok::With),
    ("without", Tok::Without),
    ("no", Tok::No),
    ("a", Tok::Article),
    ("an", Tok::Article),
    // filler
    ("a bit", Tok::Filler),
    ("a little", Tok::Filler),
    ("the", Tok::Filler),
    ("please", Tok::Filler),
    ("me", Tok::Filler),
    ("it", Tok::Filler),
    ("its", Tok::Filler),
    ("them", Tok::Filler),
    ("to", Tok::Filler),
    ("of", Tok::Filler),
    ("by", Tok::Filler),
    ("for", Tok::Filler),
    ("in", Tok::Filler),
    ("on", Tok::Filler),
    ("my", Tok::Filler),
    ("some", Tok::Filler),
    ("pair of", Tok::Filler),
    ("that", Tok::Filler),
    ("this", Tok::Filler),
    ("i", Tok::Filler),
    ("would", Tok::Filler),
    ("like", Tok::Filler),
    ("can", Tok::Filler),
    ("you", Tok::Filler),
    ("could", Tok::Filler),
    ("both", Tok::Filler),
    ("slightly", Tok::Filler),
    ("more", Tok::Filler),
    ("fit", Tok::Filler),
    ("front", Tok::Filler),
    ("is", Tok::Filler),
    ("be", Tok::Filler),
];

/// Lowercase, split punctuation into its own words, turn `2"` into `2 inches`
/// and `2cm` into `2 cm`.
fn normalize(sentence: &str) -> Vec<String> {
    let mut s = String::new();
    let chars: Vec<char> = sentence.chars().collect();
    for (i, &c) in chars.iter().enumerate() {
        let prev_digit = i > 0 && chars[i - 1].is_ascii_digit();
        match c {
            '"' | '”' if prev_digit => s.push_str(" inches "),
            ',' | '.' | ';' | '!' | '?' | ':' => {
                let decimal = c == '.' && prev_digit && chars.get(i + 1).is_some_and(|n| n.is_ascii_digit());
                if decimal {
                    s.push('.');
                } else {
                    s.push(' ');
                    s.push(c);
                    s.push(' ');
                }
            }
            '-' | '_' | '"' | '\'' | '“' | '”' | '’' => s.push(' '),
            c if c.is_ascii_alphabetic() && prev_digit => {
                s.push(' ');
                s.push(c.to_ascii_lowercase());
            }
            c => s.extend(c.to_lowercase()),
        }
    }
    s.split_whitespace().map(str::to_string).collect()
}

fn number(word: &str) -> Option<f64> {
    if let Some((a, b)) = word.split_once('/') {
        let (a, b): (f64, f64) = (a.parse().ok()?, b.parse().ok()?);
        return (b != 0.0).then(|| a / b);
    }
    if word.chars().all(|c| c.is_ascii_digit() || c == '.') {
        return word.parse().ok();
    }
    None
}

pub fn tokenize(sentence: &str) -> Vec<Token> {
    let words = normalize(sentence);
    let mut out = Vec::new();
    let mut i = 0;
    'outer: while i < words.len() {
        // Longest phrase first (up to six words).
        for n in (1..=6).rev() {
            if i + n > words.len() {
                continue;
            }
            let cand = words[i..i + n].join(" ");
            if let Some((_, tok)) = PHRASES.iter().find(|(p, _)| *p == cand) {
                out.push(Token { tok: *tok, text: cand });
                i += n;
                continue 'outer;
            }
            if let Some((name, _)) = COLORS.iter().find(|(c, _)| *c == cand) {
                out.push(Token { tok: Tok::Color(name), text: cand });
                i += n;
                continue 'outer;
            }
        }
        let w = &words[i];
        let tok = match w.as_str() {
            "," => Tok::Comma,
            "." | ";" | "!" | "?" | ":" => Tok::Stop,
            _ => number(w).map_or(Tok::Unknown, Tok::Num),
        };
        out.push(Token { tok, text: w.clone() });
        i += 1;
    }
    out
}

/// Every known word and phrase, sorted.
pub fn vocabulary() -> Vec<&'static str> {
    let mut v: Vec<&str> = PHRASES.iter().map(|(p, _)| *p).chain(COLORS.iter().map(|(c, _)| *c)).collect();
    v.sort_unstable();
    v.dedup();
    v
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let sub = prev[j] + usize::from(ca != *cb);
            cur.push(sub.min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

/// The closest known single word, if it is close enough to be a typo.
pub fn suggest(word: &str) -> Option<&'static str> {
    PHRASES
        .iter()
        .map(|(p, _)| *p)
        .chain(COLORS.iter().map(|(c, _)| *c))
        .filter(|p| !p.contains(' '))
        .map(|p| (edit_distance(word, p), p))
        .filter(|(d, p)| *d <= (p.len() / 3).max(1))
        .min()
        .map(|(_, p)| p)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(s: &str) -> Vec<Tok> {
        tokenize(s).into_iter().map(|t| t.tok).collect()
    }

    #[test]
    fn phrases_win_over_words() {
        assert_eq!(toks("T-shirt"), vec![Tok::Garment(Garment::Tshirt)]);
        assert_eq!(toks("button-up shirt"), vec![Tok::Garment(Garment::Shirt)]);
    }

    #[test]
    fn amounts_split_from_units() {
        assert_eq!(toks("2cm"), vec![Tok::Num(2.0), Tok::Unit(10.0)]);
        assert_eq!(toks("1.5\""), vec![Tok::Num(1.5), Tok::Unit(INCH)]);
        assert_eq!(toks("two and a half inches"), vec![Tok::Num(2.0), Tok::Half, Tok::Unit(INCH)]);
        assert_eq!(toks("1/2 inch"), vec![Tok::Num(0.5), Tok::Unit(INCH)]);
    }

    #[test]
    fn sentence_ends_are_separators() {
        assert_eq!(toks("hood. then"), vec![Tok::Part(Part::Hood), Tok::Stop, Tok::Then]);
    }

    #[test]
    fn colours_and_prints() {
        assert_eq!(toks("dark heather grey"), vec![Tok::Shade(-1.0), Tok::Color("heather grey")]);
        assert_eq!(toks("thin navy stripes"), vec![Tok::Scale(-1.0), Tok::Color("navy"), Tok::Print(PrintKind::Stripes)]);
        assert_eq!(toks("polka dot"), vec![Tok::Print(PrintKind::Dots)]);
    }

    #[test]
    fn typos_get_a_suggestion() {
        assert_eq!(suggest("sleves"), Some("sleeves"));
        assert_eq!(suggest("hodie"), Some("hoodie"));
        assert_eq!(suggest("zzzzzz"), None);
    }
}
