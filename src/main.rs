mod app;
mod blit;
mod clipboard;
mod cypress;
mod keys;
mod ui;

use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "mnml-test-cypress",
    version,
    about = "Cypress test results viewer for mnml — mochawesome JSON"
)]
struct Cli {
    /// Path to a `mochawesome.json` (or to a directory containing
    /// one — looks for `mochawesome.json` / `output.json` /
    /// `results/mochawesome.json`).
    path: PathBuf,
    /// Print resolved path + parsed stats and exit.
    #[arg(long)]
    check: bool,
    /// Blit-host mode — render into a UDS-served cell grid instead
    /// of the local terminal. Used by mnml / tmnl to host this
    /// binary as a pane (`:host.launch mnml-test-cypress
    /// path/to/mochawesome.json`).
    #[arg(long, value_name = "SOCKET")]
    blit: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let resolved = resolve_input(&cli.path)?;

    let report = cypress::load(&resolved)?;

    if cli.check {
        println!("source: {}", resolved.display());
        let s = &report.stats;
        println!(
            "  tests={} passes={} failures={} pending={} duration={}",
            s.tests, s.passes, s.failures, s.pending, cypress::fmt_duration(s.duration_ms)
        );
        println!("  specs: {}", report.specs.len());
        for spec in &report.specs {
            println!(
                "    - {} ({} tests)",
                if spec.full_file.is_empty() {
                    spec.file.clone()
                } else {
                    spec.full_file.clone()
                },
                spec.tests.len()
            );
        }
        return Ok(());
    }

    let mut app = app::App::new(resolved, report)?;

    if let Some(socket) = cli.blit {
        blit::run(&mut app, std::path::Path::new(&socket)).await
    } else {
        ui::run(&mut app).await
    }
}

/// Accept a JSON file path directly, or a directory — in which
/// case look for the conventional cypress / mochawesome filenames.
fn resolve_input(input: &std::path::Path) -> Result<PathBuf> {
    if input.is_file() {
        return Ok(input.to_path_buf());
    }
    if input.is_dir() {
        let candidates = [
            "mochawesome.json",
            "output.json",
            "results/mochawesome.json",
        ];
        for c in candidates {
            let p = input.join(c);
            if p.is_file() {
                return Ok(p);
            }
        }
        anyhow::bail!(
            "{} is a directory but no mochawesome.json / output.json / results/mochawesome.json was found inside",
            input.display()
        );
    }
    anyhow::bail!("{} does not exist", input.display())
}
