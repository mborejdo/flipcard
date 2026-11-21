//! The films' bytes: SHA-256 of every file of the wide example and of a small tall render of the
//! phone example, against `golden.sha256`. `FLIPCARD_BLESS=1` rewrites that file.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use flipkit::{Config, Options, Outputs, Settings};
use sha2::{Digest, Sha256};

fn here() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Every file `flipcard --gif --mp4 --docx --pdf --cards` writes for `deck`, by its path under
/// `name`.
fn films(name: &str, deck: &str, flags: Settings) -> Result<Vec<(String, Vec<u8>)>> {
    let path = here().join("../..").join(deck);
    let config = Config::load(path.parent().context("the deck's folder")?)?;
    let loaded = config.deck(&path)?;
    let options = Options::resolve(flags, &loaded.source, &config)?;
    let both = Outputs {
        gif: true,
        mp4: true,
    };
    let rendered = flipkit::render(&loaded, &options, both)?;
    let stem = path.file_stem().context("the deck's name")?.display();
    let gif = rendered.gif.clone().context("the GIF")?;
    let mp4 = rendered.mp4.clone().context("the MP4")?;
    let note = "The film plays in Acrobat when the page opens; other viewers show the still.";
    let mut files = vec![
        (format!("{stem}.docx"), rendered.docx(&loaded.deck, &gif)?),
        (format!("{stem}.gif"), gif),
        (
            format!("{stem}.pdf"),
            rendered.pdf(&loaded.deck, &mp4, note)?,
        ),
        (format!("{stem}.mp4"), mp4),
    ];
    for (n, (slide, png)) in rendered.cards()?.into_iter().enumerate() {
        files.push((format!("cards/card{slide:02}-{n:03}.png"), png));
    }
    Ok(files
        .into_iter()
        .map(|(file, bytes)| (format!("{name}/{file}"), bytes))
        .collect())
}

#[test]
fn the_films_are_the_golden_bytes() {
    let mut files = films("film", "examples/film/slides.md", Settings::default()).unwrap();
    let small = Settings {
        width: Some(216),
        fps: Some(10),
        gif_width: Some(72),
        gif_fps: Some(5),
        ..Settings::default()
    };
    files.extend(films("phone", "examples/phone/script.md", small).unwrap());
    let mut actual = String::new();
    for (name, bytes) in &files {
        for byte in Sha256::digest(bytes) {
            let _ = write!(actual, "{byte:02x}");
        }
        let _ = writeln!(actual, "  {name}");
    }
    let golden = here().join("tests/golden.sha256");
    if std::env::var_os("FLIPCARD_BLESS").is_some() {
        std::fs::write(&golden, &actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&golden).unwrap();
    assert_eq!(
        actual, expected,
        "the films changed; FLIPCARD_BLESS=1 cargo test -p flipkit --test golden rewrites the hashes"
    );
}
