# flipkit

A presiterm deck as a flip-card film: every slide a 16:9 Typst card in the deck's tokens, every
key press a build-in; `filmstrip` writes GIF, MP4, Word file and PDF.

```rust,no_run
use std::path::Path;

use flipkit::{Config, Options, Settings};

let file = Path::new("talk/slides.md");
let config = Config::load(Path::new("talk"))?;
let loaded = config.deck(file)?;
let options = Options::resolve(Settings::default(), &loaded.source, &config)?;
let rendered = flipkit::render(&loaded, &options)?;
std::fs::write("talk/slides.gif", rendered.gif()?)?;
# Ok::<(), anyhow::Error>(())
```

A deck from memory: `config.parse(source, name, base)`. `check` draws only the fully built cards
and returns the ones that overflow. License: MIT OR Apache-2.0.
