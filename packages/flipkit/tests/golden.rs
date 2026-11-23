//! The films' bytes: SHA-256 of every file of the wide example, of a small tall render of the
//! phone example, of its films padded square, of typed marks, of a heading that rises over typed text and of the letter presets, sharp and blurred, against `golden.sha256`. `FLIPCARD_BLESS=1` rewrites that file.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use flipkit::{Config, Options, Outputs, Settings};
use sha2::{Digest, Sha256};

fn here() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// Every file `flipcard --gif --mp4 --cards` writes for `deck`, by its path under
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
    let mut files = vec![
        (
            format!("{stem}.gif"),
            rendered.gif.clone().context("the GIF")?,
        ),
        (
            format!("{stem}.mp4"),
            rendered.mp4.clone().context("the MP4")?,
        ),
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
    let padded = Settings {
        pad: Some(flipkit::Pad::Square),
        ..small
    };
    files.extend(
        films("padded", "examples/phone/script.md", padded)
            .unwrap()
            .into_iter()
            .filter(|(name, _)| !name.contains("/cards/")),
    );
    // typed words with marks, the caret after the last: nothing moves as they finish
    let typed = Settings {
        width: Some(216),
        fps: Some(15),
        gif_width: Some(108),
        gif_fps: Some(15),
        ..Settings::default()
    };
    files.extend(
        films("type", "packages/flipkit/tests/type/marks.md", typed)
            .unwrap()
            .into_iter()
            .filter(|(name, _)| !name.contains("/cards/")),
    );
    // the heading rises with its bar, then the text is typed
    files.extend(
        films("heading", "packages/flipkit/tests/type/heading.md", typed)
            .unwrap()
            .into_iter()
            .filter(|(name, _)| !name.contains("/cards/")),
    );
    // the letter presets: typed, a wave, a drop, a decode
    files.extend(
        films("letters", "examples/letters/script.md", typed)
            .unwrap()
            .into_iter()
            .filter(|(name, _)| !name.contains("/cards/")),
    );
    // the same with motion blur: four sub-frames a moving frame
    let blurred = Settings {
        blur: Some(4),
        ..typed
    };
    files.extend(
        films("blur", "examples/letters/script.md", blurred)
            .unwrap()
            .into_iter()
            .filter(|(name, _)| !name.contains("/cards/")),
    );
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
