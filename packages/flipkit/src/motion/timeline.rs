//! Time in a scene, in half-beats. The blocks come in order, each on the beat after the one
//! before has landed: a block says next, never when. In a block the words follow each other by
//! the preset's stagger; `type` and `karaoke` take each word its letters at the pace, half a beat
//! more after `, ; :` and a beat after `. ! ?`. `//` waits a beat. A marked word draws its mark
//! (tints and pulses when bold) once it has landed, the heading's marker once the heading has.

use serde_json::json;

use super::{Curve, Preset, count};
use crate::script::Word;

/// Half-beats a word takes to come in, and a mark to draw.
const ENTER: f64 = 2.0;
/// Half-beats a metric counts up.
const COUNT: f64 = 6.0;
/// Half-beats of rest after the entrances, at least.
const REST: f64 = 8.0;
/// Letters of an average word, its space included: words a minute times this is letters.
const LETTERS: f64 = 5.5;

/// One word of a card, in reading order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Cue {
    /// Its block; 0 is the heading.
    pub block: usize,
    /// The key press that brings it.
    pub step: usize,
    /// A metric's value, which counts up.
    pub value: Option<String>,
    /// The word with its marks; `run.pause`: a `//` before it.
    pub word: Word,
}

impl Cue {
    /// It has a mark that draws once it has landed.
    const fn marked(&self) -> bool {
        let run = &self.word.run;
        run.bold || run.underline || run.highlight || run.strike
    }

    /// Half-beats a paced reading waits after it: punctuation at its end.
    fn breath(&self) -> f64 {
        let word = &self.word;
        match word
            .tail
            .chars()
            .last()
            .or_else(|| word.run.text.chars().last())
        {
            Some(',' | ';' | ':') => 1.0,
            Some('.' | '!' | '?') => 2.0,
            _ => 0.0,
        }
    }
}

/// When the words of one scene come in, and how they look at any instant.
#[derive(Clone, Debug)]
pub(crate) struct Timeline {
    preset: Preset,
    /// Seconds per half-beat.
    half: f64,
    /// Words a minute.
    pace: f64,
    cues: Vec<Cue>,
    /// The half-beat each word starts to come in and the half-beats it takes; `None` for a word
    /// already there.
    starts: Vec<Option<(f64, f64)>>,
    /// The half-beat the heading's marker starts to draw, if it has one and it comes in.
    mark: Option<f64>,
    marked: bool,
    /// The half-beat the last entrance (or mark) ends.
    end: f64,
}

impl Timeline {
    /// The words of `cues` that come in at `step`, a `beat` seconds long, read at `pace` words a
    /// minute; `marked`: the heading has a marker.
    pub(crate) fn new(
        preset: Preset,
        beat: f64,
        pace: f64,
        step: usize,
        cues: Vec<Cue>,
        marked: bool,
    ) -> Self {
        let half = beat / 2.0;
        // half-beats a letter takes at the pace
        let letter = if pace > 0.0 {
            60.0 / (pace * LETTERS) / half
        } else {
            0.0
        };
        let mut starts = vec![None; cues.len()];
        let (mut t, mut landed, mut end, mut mark, mut block) = (0.0, 0.0, 0.0_f64, None, None);
        let moving = !matches!(preset, Preset::Still | Preset::Flip);
        for (i, cue) in cues.iter().enumerate() {
            if cue.step != step || !moving {
                continue;
            }
            if block != Some(cue.block) {
                if block == Some(0) && marked {
                    mark = Some(landed);
                }
                (t, block) = ((landed / 2.0_f64).ceil() * 2.0, Some(cue.block));
            }
            if cue.word.run.pause {
                t += 2.0;
            }
            let len = match &cue.value {
                Some(_) => COUNT,
                None if preset.paced() => (cue.word.letters() + 1) as f64 * letter,
                None => ENTER,
            };
            starts[i] = Some((t, len));
            landed = landed.max(t + len);
            end = end.max(t + len + if cue.marked() { ENTER } else { 0.0 });
            t += if preset.paced() {
                len + cue.breath()
            } else {
                f64::from(preset.stagger())
            };
        }
        if block == Some(0) && marked {
            mark = Some(landed);
        }
        Self {
            preset,
            half,
            pace,
            starts,
            end: mark.map_or(end, |m| end.max(m + ENTER)),
            mark,
            marked,
            cues,
        }
    }

    /// Seconds of the entrances.
    pub(crate) fn entrances(&self) -> f64 {
        self.end * self.half
    }

    /// Seconds the scene stays: the entrances, then its new words read at the pace (a paced
    /// preset read them coming in), at least four beats.
    pub(crate) fn length(&self, step: usize) -> f64 {
        let words = self.cues.iter().filter(|c| c.step == step).count() as f64;
        let read = if self.pace > 0.0 && !self.preset.paced() {
            words * 60.0 / self.pace
        } else {
            0.0
        };
        (self.end + (read / self.half).ceil().max(REST)) * self.half
    }

    /// The looks of every word, the metrics' values and where the caret is at `t` seconds, as the
    /// JSON `word()` reads.
    pub(crate) fn state(&self, t: f64) -> String {
        // in half-beats, snapped so a frame on a half-beat lands on it exactly
        let now = (t / self.half * 1e6).round() / 1e6;
        let p =
            |start: Option<f64>, len: f64| start.map_or(1.0, |s| ((now - s) / len).clamp(0.0, 1.0));
        let draw = Curve::Strong.at(p(self.mark, ENTER));
        let mut values = Vec::new();
        let looks: Vec<_> = self
            .cues
            .iter()
            .zip(&self.starts)
            .map(|(cue, &start)| {
                let (at, len) = start.map_or((None, 1.0), |(s, l)| (Some(s), l));
                let mut look = cue.value.as_ref().map_or_else(
                    || self.preset.look(p(at, len), cue.word.letters()),
                    |value| {
                        values.push(count(value, Curve::Ease.at(p(at, COUNT))));
                        Preset::Rise.look(p(at, ENTER), 1)
                    },
                );
                let after = p(at.map(|s| s + len), ENTER);
                let run = &cue.word.run;
                if run.bold || run.strike {
                    look.tint = Curve::Ease.at(after);
                }
                if run.bold {
                    // up on a spring over half a beat, back on one over the other half
                    let (up, back) = (
                        Curve::Spring.at(2.0 * after),
                        Curve::Spring.at(2.0f64.mul_add(after, -1.0)),
                    );
                    look.s *= 0.12f64.mul_add(up - back, 1.0);
                }
                if run.underline || run.highlight || run.strike {
                    look.draw = Curve::Strong.at(after);
                }
                if cue.block == 0 && self.marked {
                    look.draw = draw;
                }
                look.json()
            })
            .collect();
        let mut state = json!({ "w": looks, "n": values });
        if self.preset == Preset::Type && now < self.end && (now / 2.0).floor() % 2.0 == 0.0 {
            // the caret blinks a beat on, a beat off, after the word typed last
            let typed = self
                .starts
                .iter()
                .rposition(|s| s.is_some_and(|(s, _)| s <= now));
            state["c"] = typed.into();
        }
        state.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::script::Run;

    fn words(blocks: &[usize]) -> Vec<Cue> {
        blocks.iter().map(|&block| cue(block, "word")).collect()
    }

    fn cue(block: usize, text: &str) -> Cue {
        let mut word = Word::default();
        word.run.text = text.into();
        Cue {
            block,
            word,
            ..Cue::default()
        }
    }

    /// When each word starts, in half-beats.
    fn starts(timeline: &Timeline) -> Vec<Option<f64>> {
        timeline.starts.iter().map(|s| s.map(|(t, _)| t)).collect()
    }

    fn rise(cues: Vec<Cue>) -> Timeline {
        Timeline::new(Preset::Rise, 0.4, 200.0, 0, cues, false)
    }

    #[test]
    fn blocks_come_in_on_the_beat_after_the_one_before_has_landed() {
        // heading of 3 words: starts 0, 1, 2, lands at 4; block 1 on beat 2 (half-beat 4)
        let timeline = rise(words(&[0, 0, 0, 1, 1]));
        let on = |v: &[f64]| v.iter().map(|&t| Some(t)).collect::<Vec<_>>();
        assert_eq!(starts(&timeline), on(&[0.0, 1.0, 2.0, 4.0, 5.0]));
        assert_eq!(timeline.end, 7.0);
        let faded = Timeline::new(Preset::Fade, 0.4, 200.0, 0, words(&[0, 0, 1]), false);
        assert_eq!(starts(&faded), on(&[0.0, 0.0, 2.0]));
    }

    #[test]
    fn the_hold_is_the_entrances_then_the_reading_time_at_least_four_beats() {
        // 5 words at 200 wpm: 1.5 s = 7.5 half-beats, so 8, the minimum; at 60 wpm 5 s = 25
        let half_beats = |pace| {
            let timeline =
                Timeline::new(Preset::Rise, 0.4, pace, 0, words(&[0, 0, 0, 1, 1]), false);
            (timeline.length(0) / 0.2).round() as u32
        };
        assert_eq!(
            (half_beats(200.0), half_beats(60.0), half_beats(0.0)),
            (7 + 8, 7 + 25, 7 + 8)
        );
        let still = Timeline::new(Preset::Still, 0.4, 200.0, 0, words(&[0, 1]), false);
        assert_eq!((still.end, starts(&still)), (0.0, vec![None, None]));
    }

    #[test]
    fn a_pause_waits_one_beat() {
        let mut cues = words(&[0, 0, 0]);
        cues[2].word.run.pause = true;
        assert_eq!(starts(&rise(cues)), [Some(0.0), Some(1.0), Some(4.0)]);
    }

    #[test]
    fn a_later_step_brings_only_its_words_and_the_marker_draws_after_the_heading() {
        let mut cues = words(&[0, 0, 1, 1]);
        cues[3].step = 1;
        let first = Timeline::new(Preset::Rise, 0.4, 200.0, 0, cues.clone(), true);
        assert_eq!((first.mark, first.end), (Some(3.0), 6.0));
        let second = Timeline::new(Preset::Rise, 0.4, 200.0, 1, cues, true);
        assert_eq!(
            (starts(&second), second.mark),
            (vec![None, None, None, Some(0.0)], None)
        );
        assert_eq!(
            second.state(0.0),
            r#"{"n":[],"w":[{},{},{},{"o":0.0,"y":0.5}]}"#
        );
    }

    #[test]
    fn a_metric_rises_and_counts_up_over_three_beats() {
        let cues = vec![Cue {
            block: 1,
            value: Some("1 200".into()),
            ..Cue::default()
        }];
        let timeline = Timeline::new(Preset::Fade, 0.4, 200.0, 0, cues, false);
        assert_eq!(timeline.end, 6.0);
        assert!(timeline.state(0.0).contains(r#""n":["0"]"#));
        assert!(timeline.state(1.2).contains(r#""n":["1 200"]"#));
    }

    #[test]
    fn a_paced_reveal_takes_its_words_at_the_pace() {
        // words of 4 and 5 letters: 5.5 with the space, the average the pace counts
        for (preset, pace) in [(Preset::Type, 200.0), (Preset::Karaoke, 120.0)] {
            let cues: Vec<Cue> = (0..20)
                .map(|i| cue(1, if i % 2 == 0 { "word" } else { "words" }))
                .collect();
            let timeline = Timeline::new(preset, 0.4, pace, 0, cues, false);
            let read = 20.0 * 60.0 / pace;
            assert!((timeline.entrances() - read).abs() <= 0.4, "{preset:?}");
            // read coming in: four beats of rest after
            assert!((timeline.length(0) - timeline.entrances() - 1.6).abs() < 1e-9);
        }
    }

    #[test]
    fn punctuation_breathes_half_a_beat_after_a_comma_and_a_beat_after_a_stop() {
        let mut cues = vec![
            cue(1, "one"),
            cue(1, "two"),
            cue(1, "three"),
            cue(1, "four"),
        ];
        cues[0].word.tail = ",".into();
        cues[1].word.run.text = "two.".into();
        let timeline = Timeline::new(Preset::Type, 0.4, 200.0, 0, cues.clone(), false);
        let plain: Vec<Cue> = ["one", "two", "three", "four"]
            .iter()
            .map(|t| cue(1, t))
            .collect();
        let even = Timeline::new(Preset::Type, 0.4, 200.0, 0, plain, false);
        let gap = |t: &Timeline, i: usize| starts(t)[i].unwrap() - starts(&even)[i].unwrap();
        // "one," has a letter more than "one": its time, then the comma's half-beat
        let letter = 60.0 / (200.0 * LETTERS) / 0.2;
        assert!((gap(&timeline, 1) - letter - 1.0).abs() < 1e-9);
        assert!((gap(&timeline, 2) - gap(&timeline, 1) - letter - 2.0).abs() < 1e-9);
        assert_eq!(starts(&timeline)[0], starts(&even)[0]);
    }

    #[test]
    fn the_caret_blinks_on_the_beat_grid_and_is_gone_at_rest() {
        let cues: Vec<Cue> = (0..30).map(|_| cue(1, "word")).collect();
        let timeline = Timeline::new(Preset::Type, 0.4, 200.0, 0, cues, false);
        let caret = |t: f64| timeline.state(t).contains(r#""c":"#);
        assert!(caret(0.0) && caret(0.39) && !caret(0.4) && !caret(0.79) && caret(0.8));
        // a word of 4 letters and its space at 200 wpm: 0.27 s
        assert!(
            timeline.state(0.1).contains(r#""c":0"#) && timeline.state(0.3).contains(r#""c":1"#)
        );
        assert!(!caret(timeline.entrances()));
        let rose = rise(words(&[1, 1]));
        assert!(!rose.state(0.0).contains(r#""c":"#));
    }

    #[test]
    fn a_mark_draws_once_its_word_has_landed_and_bold_tints_and_pulses() {
        let mut cues = words(&[1, 1, 1, 1]);
        let marks: [fn(&mut Run); 4] = [
            |r| r.bold = true,
            |r| r.highlight = true,
            |r| r.underline = true,
            |r| r.strike = true,
        ];
        for (cue, mark) in cues.iter_mut().zip(marks) {
            mark(&mut cue.word.run);
        }
        let timeline = Timeline::new(Preset::Fade, 0.4, 200.0, 0, cues, false);
        // all land at 2 half-beats; the marks draw over the next 2
        assert_eq!(timeline.end, 4.0);
        let look = |t: f64, i: usize| {
            let state: serde_json::Value = serde_json::from_str(&timeline.state(t)).unwrap();
            state["w"][i].clone()
        };
        assert_eq!(look(0.4, 0)["tint"], 0.0);
        assert_eq!(look(0.4, 1)["draw"], 0.0);
        assert_eq!(look(0.6, 0)["s"], 1.12);
        assert!(look(0.6, 3)["tint"].as_f64().unwrap() > 0.5);
        for i in 0..4 {
            assert_eq!(look(0.8, i), serde_json::json!({}), "{i}");
        }
    }
}
