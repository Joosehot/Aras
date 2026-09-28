use anyhow::{bail, Context, Result};
use aras::config::Config;
use aras::model::{Garment, Size};
use aras::{compile, explain, lexicon, Options, SheetKind};
use clap::Parser;
use std::io::Read;
use std::path::PathBuf;

/// Aras: describe the garment in plain words, get a deterministic sewing pattern.
///
///   aras "make a t-shirt with long sleeves and a hood" -o tee.svg
#[derive(Parser)]
#[command(version)]
struct Cli {
    /// The sentence. Omit to read it from --file or stdin.
    sentence: Option<String>,
    /// Read the sentence from a file.
    #[arg(short, long)]
    file: Option<PathBuf>,
    /// Write the SVG here instead of stdout.
    #[arg(short, long)]
    out: Option<PathBuf>,
    /// Printable pattern pieces (halves on the fold, test square) instead of the product sheet.
    #[arg(long, conflicts_with = "marker")]
    pieces: bool,
    /// Cutting layout on fabric at 1:1 instead of the product sheet.
    #[arg(long)]
    marker: bool,
    /// Size S, M, L or XL (overrides the sentence).
    #[arg(long)]
    size: Option<String>,
    /// Garment to change when the sentence names none: tshirt, hoodie, sweatshirt, shirt, pants.
    #[arg(long)]
    base: Option<String>,
    /// Show the spec, every decision, the finalists and every check (stderr).
    #[arg(long)]
    explain: bool,
    /// Ignore unknown words instead of failing.
    #[arg(long)]
    lenient: bool,
    /// Use this rules.toml instead of the built-in one.
    #[arg(long)]
    rules: Option<PathBuf>,
    /// List every word and phrase Aras knows.
    #[arg(long)]
    vocabulary: bool,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    if cli.vocabulary {
        for w in lexicon::vocabulary() {
            println!("{w}");
        }
        return Ok(());
    }
    let cfg = match &cli.rules {
        Some(p) => Config::load(p)?,
        None => Config::builtin(),
    };
    let size = match &cli.size {
        Some(s) => Some(Size::parse(s).with_context(|| format!("unknown size {s:?} (S, M, L or XL)"))?),
        None => None,
    };
    let base = match &cli.base {
        Some(b) => Some(Garment::ALL.into_iter().find(|g| g.key() == b).with_context(|| format!("unknown base {b:?}"))?),
        None => None,
    };
    let sentence = match (&cli.sentence, &cli.file) {
        (Some(s), _) => s.clone(),
        (None, Some(f)) => std::fs::read_to_string(f).with_context(|| format!("reading {}", f.display()))?,
        (None, None) => {
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s)?;
            s
        }
    };
    let opts = Options { lenient: cli.lenient, base, size, sheet: if cli.pieces {
        SheetKind::Pattern
    } else if cli.marker {
        SheetKind::Marker
    } else {
        SheetKind::Product
    } };
    let compiled = match compile(&sentence, &cfg, &opts) {
        Ok(c) => c,
        Err(diags) => {
            for d in &diags {
                eprintln!("{d}");
            }
            bail!("no pattern drafted ({} problem{})", diags.len(), if diags.len() == 1 { "" } else { "s" });
        }
    };
    if cli.explain {
        eprint!("{}", explain::explain(&compiled.spec, &compiled.outcome, &compiled.look, &compiled.marker));
    }
    match &cli.out {
        Some(p) => std::fs::write(p, &compiled.svg).with_context(|| format!("writing {}", p.display()))?,
        None => print!("{}", compiled.svg),
    }
    Ok(())
}
