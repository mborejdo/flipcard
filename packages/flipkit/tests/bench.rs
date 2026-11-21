//! Speed: the phone example, a tall film of 30 s or more at 1080x1920 and 30 fps, GIF and MP4,
//! at under 10 s of work per 30 s of film. `cargo test --release -p flipkit -- --ignored bench`; peak memory with
//! `/usr/bin/time -l` (macOS) or `-v` (Linux) around it.

use std::path::Path;
use std::time::Instant;

use flipkit::{Config, Options, Outputs, Settings};

#[test]
#[ignore = "a benchmark: cargo test --release -p flipkit -- --ignored bench"]
fn bench_a_30_s_tall_film() {
    let config = Config::default();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/phone/script.md");
    let loaded = config.deck(&path).unwrap();
    let options = Options::resolve(Settings::default(), &loaded.source, &config).unwrap();
    let both = Outputs {
        gif: true,
        mp4: true,
    };
    let start = Instant::now();
    let rendered = flipkit::render(&loaded, &options, both).unwrap();
    let spent = start.elapsed().as_secs_f64();
    let length = f64::from(u32::try_from(rendered.frames).unwrap()) / f64::from(options.fps);
    let per_30 = spent * 30.0 / length;
    eprintln!("a {length:.1} s tall film, GIF and MP4, in {spent:.1} s: {per_30:.1} s per 30 s");
    assert!(
        length >= 30.0 && per_30 < 10.0,
        "{per_30:.1} s per 30 s of film"
    );
}
