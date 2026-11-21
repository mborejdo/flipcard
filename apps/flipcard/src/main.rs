//! The `flipcard` binary: render a deck as a film, lint its cards, show its config.

// The binary's output is the product: what it wrote goes to stdout, the one-line failure to stderr.
#![allow(
    clippy::print_stdout,
    clippy::print_stderr,
    reason = "the binary prints what each command returns"
)]

mod cli;

use std::path::{Path, PathBuf};

use anyhow::{Result, bail, ensure};
use clap::Parser;
use flipkit::deckmark::deck_dir;
use flipkit::{Config, Options, output_path};

use crate::cli::{Cli, Command, FilmArgs};

fn run() -> Result<i32> {
    let cli = Cli::parse();
    match cli.command {
        Some(Command::Lint { file }) => {
            let config = Config::load(&deck_dir(&file))?;
            let loaded = config.deck(&file)?;
            let options = Options::resolve(flipkit::Settings::default(), &loaded.source, &config)?;
            let overflows = flipkit::check(&loaded, &options)?;
            for overflow in &overflows {
                println!("{overflow}");
            }
            println!(
                "{} of {} cards overflow",
                overflows.len(),
                loaded.deck.slides.len()
            );
            Ok(i32::from(!overflows.is_empty()))
        },
        Some(Command::Config { path }) => {
            let from = path.as_deref().unwrap_or_else(|| Path::new("."));
            println!("{}", Config::load(&deck_dir(from))?);
            Ok(0)
        },
        None => film(&cli.film),
    }
}

/// Render the deck and write the files asked for (the GIF when none is).
fn film(args: &FilmArgs) -> Result<i32> {
    let Some(file) = args.file.as_deref() else {
        bail!("no deck given: run `flipcard talk/slides.md`, or see `flipcard --help`")
    };
    let config = Config::load(&deck_dir(file))?;
    let loaded = config.deck(file)?;
    let options = Options::resolve(args.settings(), &loaded.source, &config)?;
    let rendered = flipkit::render(&loaded, &options)?;
    let out = args.out.as_deref();
    let write = |path: PathBuf, bytes: &[u8]| -> Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&path, bytes)
            .map_err(|e| anyhow::anyhow!("write {}: {e}", path.display()))?;
        println!("{}  {} KB", path.display(), bytes.len() / 1024);
        Ok(())
    };
    let any_other = args.mp4 || args.docx || args.pdf;
    let gif = (args.gif || args.docx || args.preview || !any_other)
        .then(|| rendered.gif())
        .transpose()?;
    if let Some(gif) = &gif {
        if args.gif || !any_other {
            write(output_path(file, out, "gif"), gif)?;
        }
        if args.docx {
            write(
                output_path(file, out, "docx"),
                &rendered.docx(&loaded.deck, gif)?,
            )?;
        }
    }
    if args.mp4 || args.pdf {
        let mp4 = rendered.mp4()?;
        if args.mp4 {
            write(output_path(file, out, "mp4"), &mp4)?;
        }
        if args.pdf {
            let note =
                "The film plays in Acrobat when the page opens; other viewers show the still.";
            write(
                output_path(file, out, "pdf"),
                &rendered.pdf(&loaded.deck, &mp4, note)?,
            )?;
        }
    }
    if let Some(dir) = &args.cards {
        for (n, (slide, png)) in rendered.cards()?.iter().enumerate() {
            write(dir.join(format!("card{slide:02}-{n:03}.png")), png)?;
        }
    }
    if let Some(gif) = gif.as_deref().filter(|_| args.preview) {
        println!("{}", preview(gif)?);
    }
    Ok(0)
}

/// The GIF as an OSC 1337 inline image, 18 rows high: WezTerm and iTerm2 draw it in place.
fn preview(gif: &[u8]) -> Result<String> {
    let program = std::env::var("TERM_PROGRAM").unwrap_or_default();
    ensure!(
        matches!(program.as_str(), "WezTerm" | "iTerm.app")
            || std::env::var_os("WEZTERM_PANE").is_some(),
        "--preview needs WezTerm or iTerm2 (OSC 1337 inline images)"
    );
    Ok(format!(
        "\x1b]1337;File=inline=1;height=18;preserveAspectRatio=1;size={}:{}\x07",
        gif.len(),
        base64(gif)
    ))
}

/// Standard base64 with padding (RFC 4648).
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (u32::from(chunk[0]) << 16)
            | (u32::from(*chunk.get(1).unwrap_or(&0)) << 8)
            | u32::from(*chunk.get(2).unwrap_or(&0));
        for (i, shift) in [18, 12, 6, 0].into_iter().enumerate() {
            out.push(if i <= chunk.len() {
                char::from(ALPHABET[(n >> shift) as usize & 63])
            } else {
                '='
            });
        }
    }
    out
}

fn main() {
    match run() {
        Ok(0) => {},
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("flipcard: {e:#}");
            std::process::exit(2);
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_matches_rfc4648_vectors() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
    }
}
