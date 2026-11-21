# flipkit

A presiterm deck as a flip-card film: every slide a Typst card in the deck's tokens, wide (16:9),
tall (9:16) or square, every key press a build-in; the frames stream to `filmstrip`, which writes
GIF, MP4, Word file and PDF.

```rust,no_run
use std::path::Path;

use flipkit::{Config, Options, Outputs, Settings};

let file = Path::new("talk/slides.md");
let config = Config::load(Path::new("talk"))?;
let loaded = config.deck(file)?;
let options = Options::resolve(Settings::default(), &loaded.source, &config)?;
let outputs = Outputs { gif: true, mp4: false };
let rendered = flipkit::render(&loaded, &options, outputs)?;
std::fs::write("talk/slides.gif", rendered.gif.unwrap_or_default())?;
# Ok::<(), anyhow::Error>(())
```

A deck from memory: `config.parse(source, name, base)`. `check` draws only the fully built cards
and returns the ones that overflow. License: MIT OR Apache-2.0.
