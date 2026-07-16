use anyhow::{Context, Result};
use std::path::PathBuf;

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let config = PathBuf::from(args.next().unwrap_or_else(|| "dotman.yaml".into()));
    let output = PathBuf::from(
        args.next()
            .unwrap_or_else(|| "site/public/demo-frames.json".into()),
    );
    let width = args
        .next()
        .map(|value| value.parse())
        .transpose()
        .context("invalid terminal width")?
        .unwrap_or(120);
    let height = args
        .next()
        .map(|value| value.parse())
        .transpose()
        .context("invalid terminal height")?
        .unwrap_or(40);

    let bundle = dotman::tui::web_demo::export(&config, width, height)
        .with_context(|| format!("failed to render web demo from {}", config.display()))?;
    let json = serde_json::to_vec(&bundle)?;
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&output, json)
        .with_context(|| format!("failed to write {}", output.display()))?;
    println!("wrote {}", output.display());
    Ok(())
}
