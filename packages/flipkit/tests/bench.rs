//! Speed at 1080x1920: one frame drawn (compile and raster), the H.264 encode, the phone example
//! as a whole film. `cargo test --release -p flipkit -- --ignored bench`.

use std::path::Path;
use std::time::{Duration, Instant};

use flipkit::{Config, Format, Options, Outputs, Settings};

#[test]
#[ignore = "a benchmark: cargo test --release -p flipkit -- --ignored bench"]
fn bench_a_tall_frame_the_encode_and_a_film() {
    let config = Config::default();
    let tall = Settings {
        format: Some(Format::Tall),
        ..Settings::default()
    };
    let options = Options::resolve(tall, "", &config).unwrap();
    let deck = |text: &str| config.parse(text, "bench", Path::new("."));
    flipkit::check(&deck("# Warm up\n"), &options).unwrap();
    // a new deck every run, so no run reuses another's layout
    let runs = 8;
    let start = Instant::now();
    for i in 0..runs {
        let text = format!("# Frame {i}\n\n- point {i}\n- and one more\n");
        flipkit::check(&deck(&text), &options).unwrap();
    }
    let frame = start.elapsed() / runs;

    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/phone/script.md");
    let loaded = config.deck(&path).unwrap();
    let options = Options::resolve(Settings::default(), &loaded.source, &config).unwrap();
    let start = Instant::now();
    let both = Outputs {
        gif: true,
        mp4: true,
    };
    let rendered = flipkit::render(&loaded, &options, both).unwrap();
    let film = start.elapsed();

    // every frame new: blends from one still to the next
    let mut writer = flipkit::filmstrip::video::Writer::new(1080, 1920, 30).unwrap();
    let (mut spent, mut frames) = (Duration::ZERO, 0_u32);
    for pair in rendered.stills.windows(2) {
        for k in 0..15_u8 {
            let blend = pair[0].blend(&pair[1], f32::from(k) / 15.0);
            let start = Instant::now();
            writer.push(&blend).unwrap();
            spent += start.elapsed();
            frames += 1;
        }
    }
    let start = Instant::now();
    writer.finish().unwrap();
    spent += start.elapsed();

    eprintln!(
        "1080x1920: a frame in {} ms, H.264 at {:.0} fps; the phone example ({:.1} s, GIF and MP4) in {:.1} s",
        frame.as_millis(),
        f64::from(frames) / spent.as_secs_f64(),
        f64::from(u32::try_from(rendered.frames).unwrap()) / f64::from(options.fps),
        film.as_secs_f64()
    );
}
