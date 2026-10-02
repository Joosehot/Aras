# Aras

Describe a garment in plain words and get a deterministic sewing pattern and product sheet back. It feels like prompting an AI but behaves like a compiler.

```
$ aras "make an oversized black hoodie with white sleeves, a grey hood and rib cuffs" -o hoodie.svg
```

The output is a one-page product sheet (a tech pack) with:

- the product name and style code (`Black Oversized Hoodie`, `ARS-HD-B205`)
- front and back flats in the colourway
- a swatch for every colour, with the reason it has that colour
- the construction decisions and their checks
- a bill of materials: fabric per colour, rib, drawstring length, aglets, eyelets and thread
- the cutting layout for each fabric

`--marker` gives the cutting layout at 1:1 with each piece printed in its colourway, and `--pieces` gives a printable paper pattern.

The same sentence always produces byte-identical output, and no model runs in the generation path.

## Gallery

Every garment type, each from the one sentence under it. Every image is the default product sheet; the SVGs next to them in [`gallery/`](gallery) are 1:1 in millimetres. Regenerate them with `bash gallery/render.sh`.

| | |
|---|---|
| ![Breton t-shirt](gallery/01_breton_tee.png) `make a navy and white striped t-shirt with long sleeves` | ![Boxy sage t-shirt](gallery/02_boxy_sage_tee.png) `make a boxy sage t-shirt with a cream collar` |
| ![Colour-blocked hoodie](gallery/03_colorblock_hoodie.png) `make an oversized black hoodie with white sleeves, a grey hood and rib cuffs` | ![Forest green hoodie](gallery/04_forest_hoodie.png) `neatly make a forest green hoodie with a cream hood lining and cream drawstrings` |
| ![Heather grey sweatshirt](gallery/05_heather_sweatshirt.png) `make a heather grey sweatshirt with navy rib trims` | ![Gingham shirt](gallery/06_gingham_shirt.png) `make a red gingham shirt` |
| ![Pinstripe shirt](gallery/07_pinstripe_shirt.png) `simply make a short sleeve shirt in thin sky blue pinstripes` | ![Plaid shirt](gallery/08_plaid_shirt.png) `make a loose forest green plaid shirt` |
| ![Tight black pants](gallery/09_tight_black_pants.png) `make tight black pants` | ![Pinstripe pants](gallery/10_charcoal_pinstripe_pants.png) `make dark grey pinstriped pants` |
| ![Camo pants](gallery/11_blue_camo_pants.png) `make baggy camo pants in dark blue and cream` | ![Olive baggy pants](gallery/12_olive_baggy_pants.png) `make baggy olive pants with an elastic waist` |
| ![Sweep t-shirt](gallery/13_joose_sweep_tee.png) `make a white t-shirt with long sleeves and wheat, dark blue and forest green stripes from the hem into the sleeves, covering almost the whole shirt` (design: Joose Hotari) | |

The same engine also writes the 1:1 cutting layout (`--marker`, left: the colour-blocked hoodie, one fabric per colour) and the printable paper pattern (`--pieces`, right: the gingham shirt, halves on the fold with notches, grainlines and a 10 cm test square):

| | |
|---|---|
| ![Cutting layout](gallery/cutting_layout.png) | ![Paper pattern](gallery/paper_pattern.png) |

## Why

AI image generators are bad at clothing because they guess pixels. Sewing already has fixed, well-documented rules:

- matching seams are the same length
- the sleeve cap fits the armhole
- seam allowances and grading between sizes
- a neckline must go over the head
- stripes line up at the side seams

Aras applies those rules the way FeelRight applies music theory and Senne applies Rust idioms.

## Architecture (same as FeelRight and Senne)

| FeelRight | Senne | Aras | Role |
|---|---|---|---|
| `theory.rs` | `lexicon.rs` | `lexicon.rs` | closed vocabulary: garments, parts, changes, amounts, colours, prints |
| `parser.rs` | `parser.rs` | `parser.rs` | tokens → garment spec + design spec |
| `src/rules/` | `src/rules/` | `src/rules/` | one file per construction decision; variants + drafting |
| — | — | `src/blocks/` | torso + sleeve and trouser blocks drafted from body measurements |
| `search.rs` | `search.rs` | `search.rs` | beam over rule variants; finalists drafted, checked and laid out |
| `rules.toml` | `rules.toml` | `rules.toml` | every number: size chart, fabrics, fits, weights, print repeats |
| tension curve | modifiers | modifiers | "neatly", "simply", "cheaply" shift the finish/simplicity/economy profile |
| — | judges | judges | consistent finish, rib as a second fabric, fabric use |
| — | — | `src/design/` | colourway rules, prints, flats |
| `midi.rs` | `codegen.rs` | `svg.rs`, `product.rs` | product sheet, cutting layout, paper pattern |

**Construction.** Every garment is drafted from a size chart (S–XL) with rules like these:

- The sleeve cap height is *solved* so the cap is longer than the armhole by the fabric's cap ease (0–1 cm for knits, 1–3 cm for wovens). If the sleeve is too wide for the armhole, the armhole is deepened until it fits.
- The back crotch point of pants is solved so the back inseam is 5 mm shorter than the front.
- The back side-waist point is adjusted until both outseams are the same length.
- Waist suppression the side seam can't take becomes a front pleat.
- A hood's neck edge is solved to match the neckline exactly.
- Bands and cuffs are cut shorter than the edge they're sewn to and stretched on.

**Checks.** A pattern is only output if every check passes:

- matching seams are the same length (shoulder, side, underarm, inseam, outseam, collar, hood, waistband)
- the head goes through the neckline and its band
- the hand goes through the sleeve hem or cuff
- the foot goes through the leg hem
- the hips go through an elastic waist
- the shoulder seam stays long enough
- every outline is a simple polygon

**Search.** Each decision offers variants with finish/simplicity/economy scores:

- hood: two- or three-piece, lined or single
- cuffs: rib or hemmed
- shirt collar: one-piece or with a stand
- pants: zip fly or elastic waist
- sleeve length: continue the taper or keep the hem width

Judges score combinations. Rib cuffs are worth more when the hem is ribbed too, and the first rib part costs a second fabric. The finalists are drafted for real: candidates that fail a check are dropped, and the rest pay for the fabric their layout uses. If nothing in the beam passes, every combination is tried.

**Design.** The sentence names some colours, and the rules fill in the rest:

- Parts without their own colour take the body's.
- Rib is knitted solid, so under a print it takes the print's ground colour.
- A hood lining takes the hood's colour, or the print's second colour.
- Drawstrings contrast with the hood; buttons and thread are tonal.
- A single print colour gets its classic partner (off white on dark, navy on light).
- Prints are anchored per piece so they match across seams. Body stripes start at the underarm line, sleeves at their underarm, and legs at the crotch line, all centred on centre front.
- Colour-blocked garments get one fabric (and one cutting layout) per colour.

## Garments

| | fits | decisions |
|---|---|---|
| t-shirt | fitted, regular, loose, oversized | neck finish (rib band / self band / binding), sleeve finish, hood |
| hoodie | same | hood (2/3-piece, lined), cuffs, hem band, kangaroo pocket |
| sweatshirt | same | cuffs, hem band, neck finish |
| button shirt | same | collar (one-piece / stand), barrel cuffs or open hem, shirttail or straight hem |
| pants | tight (stretch twill), regular, baggy | zip fly or elastic waist, pleat when needed |

## Grammar, briefly

- A garment noun starts the pattern: "make a hoodie", "baggy pants", "a long sleeve shirt in size L".
- "with …" lists features until the next verb: "with long sleeves and a hood", "with rib cuffs, no pocket".
- Change verbs take a part and an amount: "lengthen the sleeves by 2 inches", "make the collar wider", "crop it by 3 cm". Amounts need units. Without an amount, the `[edits]` default applies and `--explain` says so.
- Colours go on the part after them ("white sleeves"), the part before them ("sleeves in red"), or the garment ("a navy hoodie"). "light"/"dark" shade them.
- Prints: striped, vertical stripes, pinstriped, gingham/check, plaid/tartan, polka dot, camo, with "thin" or "thick" to scale the repeat. "black and white striped" gives the print both colours.
- An unknown word is an error that suggests the closest known word (`--lenient` ignores it). The engine never guesses.

## Usage

```
aras "sentence" -o out.svg      # product sheet
aras "..." --marker -o cut.svg  # 1:1 cutting layout in colour
aras "..." --pieces -o pat.svg  # printable pattern, halves on the fold, 10 cm test square
aras "..." --size XL            # size override
aras --base hoodie "widen the collar and lengthen the sleeves by two inches"
aras --explain "..."            # spec, every decision and score, finalists, checks, colourway (stderr)
aras --rules my_rules.toml      # different numbers
aras --vocabulary               # every known word and phrase
```

## The proof

`cargo test` runs:

- **Unit tests:** every rule has one test where it applies and one where it doesn't, plus tests for the lexicon, parser, geometry, layout, colours, prints and judges.
- **`tests/golden.rs`:**
  - each sentence in `examples/*.txt` must produce byte-identical output and match `examples/out/*.svg`
  - every example must pass every construction check, have simple outlines and non-overlapping pieces, and render all three sheet kinds deterministically
  - every garment must draft in every size, and grading must grow with size
  - stripes must anchor on the side seams and underarms

Run `ARAS_BLESS=1 cargo test` to accept intended output changes.

## Non-goals for v0

- no sleeveless tops, raglan sleeves, zips on tops, pockets on pants or jeans yet
- layouts don't rotate pieces 180°, so legs don't nest head-to-toe
- a printed fabric's layout assumes each piece is printed with its own anchored print (print-on-demand cut-and-sew), not cut from pre-printed yardage
- the size chart is a standard-proportion unisex chart; replace `[sizes]` with your own measurements

## License

PolyForm Noncommercial 1.0.0, see [LICENSE](LICENSE). Free for personal, hobby, research, educational and other noncommercial use; commercial use needs a separate licence from the author. Patterns that Aras generates belong to whoever ran it.
