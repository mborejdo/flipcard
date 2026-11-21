//! The script around the deck: `:::{scene} <preset>` per slide with `:pace:` and `:beat:`, `==`
//! in a heading kept apart from `__`, and a card's text as numbered words, punctuation glued to
//! the word before, `//` a pause.

use anyhow::{Context, Result};
use colonmark::{Inline, Kind};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::motion::{Cue, Preset};

/// What one slide's `:::{scene}` and heading say.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Scene {
    /// `:::{scene} <preset>`.
    pub preset: Option<Preset>,
    /// `:pace:`, words a minute.
    pub pace: Option<f64>,
    /// `:beat:`, seconds.
    pub beat: Option<f64>,
    /// The heading with its marks.
    pub heading: Vec<Run>,
}

/// Every slide's scene, in the deck's order (cut as `deckmark` cuts the slides).
pub(crate) fn read(source: &str) -> Result<Vec<Scene>> {
    let doc = colonmark::parse(source);
    let tags: Vec<&str> = doc
        .frontmatter
        .get("tags")
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
        .collect();
    let blocks = colonmark::only::apply(doc.blocks.clone(), &tags).unwrap_or(doc.blocks);
    let sections = colonmark::sections(blocks, 2, doc.frontmatter.body_line());
    sections
        .into_iter()
        .map(|section| {
            let mut scene = Scene {
                heading: runs(
                    section.heading.as_deref().unwrap_or_default(),
                    &Run::default(),
                ),
                ..Scene::default()
            };
            for block in &section.blocks {
                let Kind::Directive(d) = &block.kind else {
                    continue;
                };
                if !d.name.eq_ignore_ascii_case("scene") {
                    continue;
                }
                let at = |key: &str| format!("line {}: scene {key}", block.line);
                let number = |key: &str| {
                    d.option(key)
                        .map(|v| v.trim().parse::<f64>().with_context(|| at(key)))
                        .transpose()
                };
                if !d.argument.is_empty() {
                    scene.preset = Some(d.argument.parse().with_context(|| at("preset"))?);
                }
                scene.pace = number("pace")?;
                scene.beat = number("beat")?.filter(|beat| *beat > 0.0);
            }
            Ok(scene)
        })
        .collect()
}

/// A run of text in one style.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools, reason = "independent inline switches")]
pub(crate) struct Run {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    /// `__underline__`, and `==` outside a heading.
    pub underline: bool,
    /// `==marker==` in a heading.
    pub mark: bool,
}

/// `inlines` as runs in `style` and their own marks.
fn runs(inlines: &[Inline], style: &Run) -> Vec<Run> {
    let mut out = Vec::new();
    for inline in inlines {
        let text = |text: &str| Run {
            text: text.to_string(),
            ..style.clone()
        };
        let (content, inner) = match inline {
            Inline::Text(t) | Inline::Html(t) => {
                out.push(text(t));
                continue;
            },
            Inline::Code(t) => {
                out.push(Run {
                    code: true,
                    ..text(t)
                });
                continue;
            },
            Inline::Image { alt, .. } => {
                out.push(text(alt));
                continue;
            },
            Inline::SoftBreak | Inline::LineBreak => {
                out.push(text(" "));
                continue;
            },
            Inline::Strong(c) => (
                c,
                Run {
                    bold: true,
                    ..text("")
                },
            ),
            Inline::Emph(c) => (
                c,
                Run {
                    italic: true,
                    ..text("")
                },
            ),
            Inline::Underline(c) => (
                c,
                Run {
                    underline: true,
                    ..text("")
                },
            ),
            Inline::Highlight(c) => (
                c,
                Run {
                    mark: true,
                    ..text("")
                },
            ),
            Inline::Strike(c) | Inline::Link { content: c, .. } => (c, text("")),
        };
        out.extend(runs(content, &inner));
    }
    out
}

/// One word of a card.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub(crate) struct Word {
    pub id: usize,
    #[serde(flatten)]
    pub run: Run,
    /// Punctuation (or the rest of a word in another style) glued to it, plain.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub tail: String,
    /// A `//` before it.
    #[serde(skip)]
    pub pause: bool,
}

/// The words of `runs`; `pause` carries a `//` over to the next word.
pub(crate) fn tokenize(runs: &[Run], pause: &mut bool) -> Vec<Word> {
    let mut words: Vec<Word> = Vec::new();
    let mut glue = false;
    for run in runs {
        for (i, token) in run.text.split(char::is_whitespace).enumerate() {
            let joined = i == 0 && glue;
            if token.is_empty() {
                continue;
            }
            if token == "//" && !joined {
                *pause = true;
                continue;
            }
            match words.last_mut() {
                Some(last) if joined || !token.chars().any(char::is_alphanumeric) => {
                    if !joined {
                        last.tail.push(' ');
                    }
                    last.tail.push_str(token);
                },
                _ => words.push(Word {
                    run: Run {
                        text: token.to_string(),
                        ..run.clone()
                    },
                    pause: std::mem::take(pause),
                    ..Word::default()
                }),
            }
        }
        if !run.text.is_empty() {
            glue = !run.text.ends_with(char::is_whitespace);
        }
    }
    words
}

/// The card's text as numbered words in reading order (the heading from `heading`, then every
/// line, item, label and value of the blocks) and a cue for each.
pub(crate) fn number(card: &mut Value, heading: &[Run]) -> Vec<Cue> {
    let mut words = Words::default();
    let runs = |value: &Value| Vec::<Run>::deserialize(value).unwrap_or_default();
    let plain = |value: &Value| {
        vec![Run {
            text: value.as_str().unwrap_or_default().to_string(),
            ..Run::default()
        }]
    };
    if !card["title"].is_null() {
        let heading = if heading.is_empty() {
            runs(&card["title"])
        } else {
            heading.to_vec()
        };
        card["title"] = words.line(&heading, 0);
    }
    let mut reveal = 0;
    for block in card["blocks"].as_array_mut().into_iter().flatten() {
        words.block += 1;
        match block["kind"].as_str().unwrap_or_default() {
            "paragraph" | "quote" => {
                for line in block["lines"].as_array_mut().into_iter().flatten() {
                    *line = words.line(&runs(line), 0);
                }
            },
            "list" => {
                for item in block["items"].as_array_mut().into_iter().flatten() {
                    let step = if item["reveal"] == true {
                        reveal += 1;
                        reveal
                    } else {
                        0
                    };
                    item["spans"] = words.line(&runs(&item["spans"]), step);
                }
            },
            "subheading" => block["text"] = words.line(&plain(&block["text"]), 0),
            "metrics" => {
                for metric in block["items"].as_array_mut().into_iter().flatten() {
                    metric["value"] = words.value(metric["value"].as_str().unwrap_or_default());
                    metric["label"] = words.line(&plain(&metric["label"]), 0);
                }
            },
            _ => {},
        }
    }
    words.cues
}

/// Numbers words in reading order.
#[derive(Debug, Default)]
struct Words {
    cues: Vec<Cue>,
    block: usize,
    values: usize,
    pause: bool,
}

impl Words {
    /// `runs` as numbered words, brought by key press `step`.
    fn line(&mut self, runs: &[Run], step: usize) -> Value {
        let mut words = tokenize(runs, &mut self.pause);
        for word in &mut words {
            word.id = self.cues.len();
            self.cues.push(Cue {
                block: self.block,
                step,
                pause: word.pause,
                value: None,
            });
        }
        json!(words)
    }

    /// A metric's value: one word that counts up.
    fn value(&mut self, value: &str) -> Value {
        let id = self.cues.len();
        self.cues.push(Cue {
            block: self.block,
            pause: std::mem::take(&mut self.pause),
            value: Some(value.to_string()),
            ..Cue::default()
        });
        self.values += 1;
        json!({ "id": id, "text": value, "count": self.values - 1 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn texts(words: &[Word]) -> Vec<String> {
        words
            .iter()
            .map(|w| format!("{}{}", w.run.text, w.tail))
            .collect()
    }

    #[test]
    fn punctuation_glues_to_the_word_before_and_a_pause_to_the_word_after() {
        let source = "# Ship ==small==, ship // often\n\nPick **one** - then // ship.\n";
        let scenes = read(source).unwrap();
        let mut pause = false;
        let heading = tokenize(&scenes[0].heading, &mut pause);
        assert_eq!(texts(&heading), ["Ship", "small,", "ship", "often"]);
        assert!(heading[1].run.mark && !heading[1].run.underline);
        assert_eq!(heading[1].tail, ",");
        assert!(heading[3].pause && !heading[2].pause);
        let body = deckmark::deck::inline("Pick **one** - then // ship.");
        let runs: Vec<Run> = body
            .iter()
            .map(|s| Run::deserialize(json!(s)).unwrap())
            .collect();
        let words = tokenize(&runs, &mut pause);
        assert_eq!(texts(&words), ["Pick", "one -", "then", "ship."]);
        assert!(words[1].run.bold && words[3].pause);
    }

    #[test]
    fn a_scene_reads_its_preset_and_options_and_rejects_an_unknown_one() {
        let source = "# A\n\n:::{scene} slide\n:pace: 120\n:beat: 0.5\n:::\n\ntext\n\n---\n\n# B\n";
        let scenes = read(source).unwrap();
        assert_eq!(scenes.len(), 2);
        assert_eq!(
            (scenes[0].preset, scenes[0].pace, scenes[0].beat),
            (Some(Preset::Slide), Some(120.0), Some(0.5))
        );
        assert_eq!(scenes[1].preset, None);
        let error = format!("{:#}", read("# A\n\n:::{scene} wobble\n:::\n").unwrap_err());
        assert!(
            error.contains("line 3") && error.contains("wobble"),
            "{error}"
        );
    }

    #[test]
    fn a_card_is_numbered_in_reading_order_with_its_steps() {
        let mut card = json!({
            "title": [{ "text": "Two words" }],
            "blocks": [
                { "kind": "list", "items": [
                    { "spans": [{ "text": "a b" }], "reveal": false },
                    { "spans": [{ "text": "c" }], "reveal": true },
                ]},
                { "kind": "metrics", "items": [{ "value": "1 200", "label": "things" }] },
            ],
        });
        let cues = number(&mut card, &[]);
        let at = |c: &Cue| (c.block, c.step, c.value.is_some());
        let got: Vec<_> = cues.iter().map(at).collect();
        assert_eq!(
            got,
            [
                (0, 0, false),
                (0, 0, false),
                (1, 0, false),
                (1, 0, false),
                (1, 1, false),
                (2, 0, true),
                (2, 0, false)
            ]
        );
        assert_eq!(card["blocks"][0]["items"][1]["spans"][0]["id"], 4);
        assert_eq!(card["blocks"][1]["items"][0]["value"]["count"], 0);
    }
}
