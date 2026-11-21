# flipkit

A presiterm deck as a flip-card film. Every slide is a 16:9 card drawn by Typst in the deck's
tokens; every key press is a build-in; `filmstrip` writes the GIF, the MP4, the Word file and the
PDF.

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

A deck made in memory: `config.parse(source, name, base)`. `check` draws only the fully built
cards and returns the ones that overflow. License: MIT OR Apache-2.0.
