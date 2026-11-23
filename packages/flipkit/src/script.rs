//! The script around the deck: `:::{scene} <preset>` per slide with `:pace:` and `:beat:`, and a
//! card's text as numbered words with their marks, punctuation glued to the word before.

use anyhow::{Context, Result};
use colonmark::Kind;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use unicode_segmentation::UnicodeSegmentation;

use crate::motion::{Cue, Preset};

/// What one slide's `:::{scene}` says.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Scene {
    /// `:::{scene} <preset>`.
    pub preset: Option<Preset>,
    /// `:pace:`, words a minute.
    pub pace: Option<f64>,
    /// `:beat:`, seconds.
    pub beat: Option<f64>,
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
            let mut scene = Scene::default();
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

/// A run of text in one style: a `deckmark` span as the card JSON carries it.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
#[allow(clippy::struct_excessive_bools, reason = "independent inline switches")]
pub(crate) struct Run {
    pub text: String,
    pub bold: bool,
    pub italic: bool,
    pub code: bool,
    pub underline: bool,
    pub highlight: bool,
    pub strike: bool,
    /// A `//` before it.
    pub pause: bool,
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
    /// Its mark runs on over the space to the next word, marked alike.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub bridge: bool,
    /// Its mark runs on from the word before, over the space between.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub bridged: bool,
}

impl Word {
    /// Its letters, the tail's included: grapheme clusters, as Typst's `str.clusters()` cuts them.
    pub(crate) fn graphemes(&self) -> Vec<&str> {
        let (text, tail) = (&self.run.text, &self.tail);
        text.graphemes(true).chain(tail.graphemes(true)).collect()
    }

    /// How many letters it has, the tail's included.
    pub(crate) fn letters(&self) -> usize {
        self.graphemes().len()
    }
}

/// Punctuation that ends a word: kept out of its marks.
const TRAILING: [char; 12] = [
    ',', '.', ';', ':', '!', '?', '"', '\'', ')', ']', '\u{201d}', '\u{2019}',
];

/// The words of `runs`; `pause` carries a `//` over to the next word.
pub(crate) fn tokenize(runs: &[Run], pause: &mut bool) -> Vec<Word> {
    let mut words: Vec<Word> = Vec::new();
    let mut glue = false;
    for run in runs {
        *pause |= run.pause;
        for (i, token) in run.text.split(char::is_whitespace).enumerate() {
            let joined = i == 0 && glue;
            if token.is_empty() {
                continue;
            }
            match words.last_mut() {
                Some(last) if joined || !token.chars().any(char::is_alphanumeric) => {
                    if !joined {
                        last.tail.push(' ');
                    }
                    last.tail.push_str(token);
                },
                last => {
                    // a line mark runs on to the next word of its run, unless punctuation is between
                    let bridged = last
                        .filter(|l| i > 0 && l.tail.is_empty())
                        .is_some_and(|last| {
                            last.bridge = run.underline || run.highlight || run.strike;
                            last.bridge
                        });
                    // trailing punctuation goes to the tail, so a mark leaves it out
                    let cut = token.trim_end_matches(TRAILING);
                    let cut = if run.code || cut.is_empty() {
                        token
                    } else {
                        cut
                    };
                    words.push(Word {
                        run: Run {
                            text: cut.to_string(),
                            pause: std::mem::take(pause),
                            ..run.clone()
                        },
                        tail: token[cut.len()..].to_string(),
                        bridged,
                        ..Word::default()
                    });
                },
            }
        }
        if !run.text.is_empty() {
            glue = !run.text.ends_with(char::is_whitespace);
        }
    }
    words
}

/// The card's text as numbered words in reading order (the heading, then every line, item, label
/// and value of the blocks) and a cue for each.
pub(crate) fn number(card: &mut Value) -> Vec<Cue> {
    let mut words = Words::default();
    let runs = |value: &Value| Vec::<Run>::deserialize(value).unwrap_or_default();
    let plain = |value: &Value| {
        vec![Run {
            text: value.as_str().unwrap_or_default().to_string(),
            ..Run::default()
        }]
    };
    if !card["title"].is_null() {
        card["title"] = words.line(&runs(&card["title"]), 0);
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
                word: word.clone(),
                ..Cue::default()
            });
        }
        json!(words)
    }

    /// A metric's value: one word that counts up.
    fn value(&mut self, value: &str) -> Value {
        let id = self.cues.len();
        let pause = std::mem::take(&mut self.pause);
        self.cues.push(Cue {
            block: self.block,
            value: Some(value.to_string()),
            word: Word {
                id,
                run: Run {
                    pause,
                    ..Run::default()
                },
                ..Word::default()
            },
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

    fn words(text: &str) -> Vec<Word> {
        let runs: Vec<Run> = deckmark::deck::inline(text)
            .iter()
            .map(|s| Run::deserialize(json!(s)).unwrap())
            .collect();
        tokenize(&runs, &mut false)
    }

    #[test]
    fn punctuation_and_hyphens_glue_and_a_pause_goes_to_the_word_after() {
        let heading = words("Ship ==small==, the KI-Plattform // often");
        assert_eq!(
            texts(&heading),
            ["Ship", "small,", "the", "KI-Plattform", "often"]
        );
        assert!(heading[1].run.highlight && !heading[1].run.underline);
        assert_eq!(
            (heading[1].tail.as_str(), heading[3].tail.as_str()),
            (",", "")
        );
        assert!(heading[4].run.pause && !heading[3].run.pause);
        let body = words("Pick **one** - then //**ship**. **Done!** `f();`");
        assert_eq!(
            texts(&body),
            ["Pick", "one -", "then", "ship.", "Done!", "f();"]
        );
        assert!(body[1].run.bold && body[3].run.bold && body[3].run.pause);
        let tails: Vec<_> = body.iter().map(|w| w.tail.as_str()).collect();
        assert_eq!(tails, ["", " -", "", ".", "!", ""]);
    }

    #[test]
    fn a_mark_bridges_the_space_between_two_words_of_its_run() {
        let bridges = |text: &str| -> Vec<bool> { words(text).iter().map(|w| w.bridge).collect() };
        assert_eq!(bridges("No ~~big bang~~ here"), [false, true, false, false]);
        assert_eq!(bridges("==one, two== **bold words**"), [false; 4]);
        assert_eq!(bridges("==a== ==b=="), [false, false]);
        let from = |text: &str| -> Vec<bool> { words(text).iter().map(|w| w.bridged).collect() };
        assert_eq!(from("No ==big bang== here"), [false, false, true, false]);
        assert_eq!(from("==one, two== ==a== ==b=="), [false; 4]);
    }

    #[test]
    fn every_word_carries_its_marks_in_the_card() {
        let mut card = json!({
            "title": deckmark::deck::inline("A ==b=="),
            "blocks": [{ "kind": "paragraph", "lines": [
                deckmark::deck::inline("**c** __d__ ~~e~~ f"),
            ]}],
        });
        let cues = number(&mut card);
        let word = |w: &Value| {
            ["bold", "underline", "highlight", "strike"]
                .iter()
                .position(|k| w[k] == true)
        };
        let line = card["blocks"][0]["lines"][0].as_array().unwrap();
        let marks: Vec<_> = card["title"]
            .as_array()
            .unwrap()
            .iter()
            .chain(line)
            .map(word)
            .collect();
        assert_eq!(marks, [None, Some(2), Some(0), Some(1), Some(3), None]);
        assert!(cues[2].word.run.bold && cues[4].word.run.strike);
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
        let cues = number(&mut card);
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
