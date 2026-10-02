# Adding words to Aras

Aras understands a closed vocabulary. A word it does not know is an error
with a "did you mean" hint, never a guess. Teaching it a word means telling it
exactly what the word stands for. This page shows where each kind of word
lives and how to check that the new one works.

Everything below is plain Rust or TOML. After any change, run:

```
cargo test
cargo run --release -- --vocabulary      # the word should be in the list
cargo run --release -- --explain "make a dress with your new word"
```

## How words are read

`src/lexicon.rs` turns a sentence into tokens before anything else.

1. **Normalise.** The sentence is lowercased. Hyphens, quotes and
   apostrophes become spaces, so `A-line`, `a line` and `a-line` are the same
   phrase. `2cm` splits into `2 cm`, and `2"` becomes `2 inches`.
2. **Match.** At each position Aras tries the longest phrase first, up to
   six words, against the `PHRASES` table and the colour names. `button down
   shirt` therefore wins over `shirt`, and `straight hem` wins over a
   separate `straight`.
3. **Anything else** is a number or `Unknown`, and an unknown word is an
   error (unless `--lenient`).

So adding a word is almost always **one line in a table**. What changes is
which table, and what the word's token is.

## Which table, by kind of word

| The word is... | Add it to | Example line |
|---|---|---|
| another name for a garment | `PHRASES` in `src/lexicon.rs` | `("sundress", Tok::Garment(Garment::Dress)),` |
| another name for a part | `PHRASES` | `("neckline", Tok::Part(Part::Collar)),` |
| a fit | `PHRASES` | `("relaxed", Tok::Fit(Fit::Loose)),` |
| a change verb | `PHRASES` | `("extend", Tok::Verb(Verb::Lengthen)),` |
| a comparative | `PHRASES` | `("longer", Tok::Cmp(Verb::Lengthen)),` |
| a construction choice | `PHRASES`, as a pin | `("camp collar", pin("collar", &["one_piece"], None)),` |
| a "how" word | `PHRASES` + `[modifiers]` in `rules.toml` | `("tailored", Tok::Mod(Modifier::Neatly)),` |
| a print | `PHRASES` | `("tartan", Tok::Print(PrintKind::Plaid)),` |
| a print size | `PHRASES` | `("ditsy", Tok::Scale(-1.0)),` |
| a colour | `COLORS` in `src/design/color.rs` | `("baby blue", 0xbcd7ef),` |
| a fabric | `PHRASES` + `[fabrics.<name>]` in `rules.toml` | `("velour", Tok::Fabric("velvet")),` |
| a dress length | `PHRASES` | `("calf length", Tok::DressLen(DressLength::Midi)),` |
| filler (ignored) | `PHRASES` | `("please", Tok::Filler),` |

### Synonyms (the common case)

If the meaning already exists, add one line that points the new word at it.
The rest of the engine never sees the difference. For example, to make
`gown` mean a dress:

```rust
("gown", Tok::Garment(Garment::Dress)),
```

### Pins: words that choose a construction

A pin narrows one rule to some of its variants. `pin(rule, variants, implies)`
takes three things:

- the rule's name (a file in `src/rules/`);
- the variants the word allows, which must be names that the rule and
  `[rules.<rule>.variants]` in `rules.toml` both have;
- optionally a part the word turns on (`Some(Part::Hood)` for "lined hood").

```rust
("bound armholes", pin("armhole_finish", &["binding"], None)),
```

If a pin's variant is not one the garment allows, the sentence fails with
"doesn't fit a ...", and the hint lists the options.

### Colours

Colours live in `COLORS` in `src/design/color.rs` as `(name, 0xRRGGBB)`.
Use dyed-fabric shades, not screen primaries. Two names may share a hex:
`"light blue"` and `"baby blue"` are the same pale blue. `light`/`dark`
before any colour already make shades, so add a colour only when that shade
would be wrong.

### Fabrics

A fabric word needs two things:

1. The fabric's numbers in `rules.toml`, read by `config.rs`. Every field is
   required except `nap`:

   ```toml
   [fabrics.velvet]
   stretch = 1.0        # how far a finished edge can be pulled (1.0 = not at all)
   seam = 15            # seam allowance, mm
   hem = 40             # hem allowance, mm
   cap_ease_min = 5     # how much longer a sleeve cap may be than the armhole
   cap_ease_max = 20
   width = 1400         # fabric width, mm (for the cutting layout)
   nap = true           # optional: pile with a direction, cut every piece one way
   ```

2. The word in `PHRASES`: `("velvet", Tok::Fabric("velvet")),`. The string
   must be the `[fabrics.<name>]` key.

The construction checks use these numbers. A fabric that does not stretch
cannot be pulled over the head through a narrow neckline, and the search
then picks a zip.

### "How" words (modifiers)

`neatly`, `simply` and `cheaply` shift the profile, which is the tradeoff
between finish, simplicity and economy. A new word for an existing modifier
is one `PHRASES` line. A new kind of modifier needs four things:

- a `Modifier` variant;
- its key in `MODIFIER_KEYS`;
- a `[modifiers.<key>]` table in `rules.toml`;
- the phrase.

## Words that mean something new

When no existing token fits, the word needs code as well as a table entry.
Follow how the dress words were added:

- **A new garment:** a `Garment` variant in `src/model.rs`, a
  `[garments.<key>]` table and a `design_defaults` colour in `rules.toml`,
  and the drafting (`src/draft.rs`, `src/blocks/`).
- **A new decision:** one file in `src/rules/` implementing `Rule`, an entry
  in `registry()` in `src/rules/mod.rs`, and `[rules.<name>]` with its
  variants in `rules.toml`. `config.rs` refuses to load if a rule's variant
  has no scores, so a typo fails at start-up rather than mid-draft.
- **A new token kind:** a `Tok` variant in `src/lexicon.rs`, and a case
  in `State::clause` in `src/parser.rs` that stores what it means in the
  `Spec`.

## Check it

- Add the word to a test in `src/lexicon.rs` (see `dress_words`) so a
  later change cannot quietly break it.
- If the word changes an output, add a sentence to `examples/` and bless the
  goldens: `ARAS_BLESS=1 cargo test`, then look at the diff before committing.
- `--explain` shows which rule each word reached. A word that reached
  nothing is a bug in the table, not in the sentence.

Watch for collisions. `small` is a size, so in "small yellow flowers" the
parser turns it into a print scale because a colour or a print follows it.
`long` means long sleeves, so a long dress is `maxi` or `floor length`.
Before adding a word, search `PHRASES` for it.
