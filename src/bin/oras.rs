//! oras: the garment Aras drafts, simulated on a mannequin.
//!
//!   oras "make a black satin dress with a cowl neck" --out out/oras
//!
//! writes view.html (turn it in a browser; #front, #three, #side, #left,
//! #back, add -bare to hide the garment), garment.obj and report.txt.

use anyhow::{anyhow, Context, Result};
use aras::config::Config;
use aras::design::Zone;
use aras::model::Size;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, about = "Oras: the garment Aras drafts, simulated on a mannequin")]
struct Cli {
    sentence: String,
    #[arg(long, default_value = "out/oras")]
    out: PathBuf,
    /// Size S, M, L or XL (overrides the sentence)
    #[arg(long)]
    size: Option<String>,
    /// Rules file (default: the built-in rules.toml)
    #[arg(long)]
    rules: Option<PathBuf>,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = match &cli.rules {
        Some(p) => Config::load(p)?,
        None => Config::builtin(),
    };
    let size = cli.size.as_deref().map(|s| Size::parse(s).ok_or_else(|| anyhow!("unknown size {s}"))).transpose()?;
    let opts = aras::Options { size, ..Default::default() };
    let c = aras::compile(&cli.sentence, &cfg, &opts).map_err(|d| anyhow!(d.iter().map(|x| x.to_string()).collect::<Vec<_>>().join("\n")))?;
    let o = aras::oras::simulate(&c, &cfg).map_err(|e| anyhow!(e))?;
    std::fs::create_dir_all(&cli.out).with_context(|| cli.out.display().to_string())?;
    let color = c.look.get(Zone::Body).dominant().rgb.css();
    let fabric = c.spec.fabric_name(&cfg);
    let shine = match fabric {
        "satin" => 1.0,
        "crepe" | "shirting" => 0.4,
        "velvet" => 0.15,
        _ => 0.3,
    };
    let title = format!("Oras · {}", c.look.name);
    std::fs::write(cli.out.join("view.html"), aras::oras::view::html(&o, &title, &color, shine))?;
    std::fs::write(cli.out.join("garment.obj"), aras::oras::view::obj(&o))?;
    std::fs::write(cli.out.join("report.txt"), &o.text)?;
    println!("{}", o.text);
    println!("wrote {}", cli.out.display());
    Ok(())
}
