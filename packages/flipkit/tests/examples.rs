//! The example deck draws: every card fits, and its frontmatter sets the timing.

use std::path::Path;

use flipkit::{Config, Options, Settings};

#[test]
fn the_film_example_fits_its_cards() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples/film/slides.md");
    let config = Config::default();
    let loaded = config.deck(&path).expect("read the film example");
    let options = Options::resolve(Settings::default(), &loaded.source, &config).expect("options");
    let overflows = flipkit::check(&loaded, &options).expect("draw the cards");
    assert!(overflows.is_empty(), "{overflows:?}");
    assert_eq!(
        (options.hold, options.enter),
        (4.0, 1.2),
        "from the frontmatter"
    );
}
