//! Time in a scene, in half-beats. The blocks come in order, each on the beat after the one
//! before has landed: a block says next, never when. In a block the words follow each other by
//! the preset's stagger; `type` and `karaoke` take each word its letters at the pace, half a beat
//! more after `, ; :` and a beat after `. ! ?`. `//` waits a beat. A marked word draws its mark
//! (tints and pulses when bold) once it has landed, a run of words marked alike as one sweep from
//! the left; the heading's marker (or bar) draws once the heading has, a metric's bar as its value
//! starts to count. Every scene rests after its last animation (see [`Timeline::length`]).
//!
//! The kinetic presets know where the words are ([`Timeline::arrange`]): `move` glides the words
//! the card before has too from there first, then the others come in; `stack` brings line after
//! line, each read at the pace, the lines in so far moving up by the room the new one takes, on a
//! spring that carries its momentum into the next line's move.

use serde_json::json;

use super::look::shape;
use super::preset::ENTER;
use super::spring::STACK;
use super::{Curve, Look, Preset, Spot, count};
use crate::script::Word;

/// Half-beats a run of marked words takes to draw, left to right as one sweep.
const MARK: f64 = 3.5;
/// Half-beats a mark waits after the last word it needs has landed.
const SETTLE: f64 = 0.5;
/// Half-beats the heading's bar (or marker) takes to draw; its hot tip fades from halfway over
/// [`FLASH`].
const BAR: f64 = 4.0;
/// Half-beats the bar's hot tip takes to fade.
const FLASH: f64 = 2.0;
/// Half-beats a laser (the heading's bar, a metric's) is busy: drawing, then its tip fading.
const LASER: f64 = if BAR > BAR / 2.0 + FLASH {
    BAR
} else {
    BAR / 2.0 + FLASH
};
/// Half-beats a word of the card before glides to its place.
const GLIDE: f64 = 3.0;
/// Half-beats a metric counts up.
const COUNT: f64 = 6.0;
/// Half-beats of rest after the entrances, at least.
const REST: f64 = 8.0;
/// Letters of an average word, its space included: words a minute times this is letters.
const LETTERS: f64 = 5.5;

/// One word of a card, in reading order.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Cue {
    /// Its block; 0 is the heading.
    pub block: usize,
    /// The key press that brings it.
    pub step: usize,
    /// A metric's value, which counts up.
    pub value: Option<String>,
    /// The word with its marks; `run.pause`: a `//` before it.
    pub word: Word,
    /// Where it is at rest (kinetic presets).
    pub spot: Option<Spot>,
    /// `move`: how it looks where it was on the card before.
    pub from: Option<Look>,
    /// `stack`: its line among the words that come in.
    pub line: usize,
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

/// A word's share of its run's mark: the run draws from `start` over [`MARK`] half-beats, this
/// word from `from` to `to` (0 to 1) of the way, by its letters.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Sweep {
    start: f64,
    from: f64,
    to: f64,
}

/// The same marks on the same line: `b` runs on from `a` as one sweep.
fn alike(a: &Cue, b: &Cue) -> bool {
    let (x, y) = (&a.word.run, &b.word.run);
    a.block == b.block
        && a.line == b.line
        && a.step == b.step
        && (x.bold, x.underline, x.highlight, x.strike)
            == (y.bold, y.underline, y.highlight, y.strike)
}

/// When the words of one scene come in, and how they look at any instant.
#[derive(Clone, Debug)]
pub(crate) struct Timeline {
    preset: Preset,
    /// How the heading comes in (the preset unless set).
    heading: Preset,
    /// Seconds per half-beat.
    half: f64,
    /// Words a minute.
    pace: f64,
    /// The key press it shows.
    step: usize,
    cues: Vec<Cue>,
    /// The heading has a marker.
    marked: bool,
    /// The half-beat each word starts to come in and the half-beats it takes; `None` for a word
    /// already there.
    starts: Vec<Option<(f64, f64)>>,
    /// When each word's mark draws; `None` for a word without one, or already there.
    sweeps: Vec<Option<Sweep>>,
    /// `stack`: each line's first word and its bottom, in points.
    lines: Vec<(usize, f64)>,
    /// The half-beat the heading's marker starts to draw, if it has one and it comes in.
    mark: Option<f64>,
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
        let mut timeline = Self {
            preset,
            heading: preset,
            half: beat / 2.0,
            pace,
            step,
            starts: Vec::new(),
            sweeps: Vec::new(),
            lines: Vec::new(),
            mark: None,
            end: 0.0,
            marked,
            cues,
        };
        timeline.schedule();
        timeline
    }

    /// The heading comes in by `preset`, the rest as before.
    pub(crate) fn heading(mut self, preset: Preset) -> Self {
        self.heading = preset;
        self.schedule();
        self
    }

    /// How `cue` comes in: the heading by its own preset.
    const fn of(&self, cue: &Cue) -> Preset {
        if cue.block == 0 {
            self.heading
        } else {
            self.preset
        }
    }

    /// It times its words by where they are.
    pub(crate) const fn kinetic(&self) -> bool {
        self.preset.kinetic() || self.heading.kinetic()
    }

    /// The words on screen once it is at rest.
    pub(crate) fn shown(&self) -> impl Iterator<Item = &Cue> {
        self.cues
            .iter()
            .filter(|c| c.step <= self.step || c.step == usize::MAX)
    }

    /// Time the words again by where they are: `spots` by word id, and for `move` the words the
    /// card before shows, by their text. A word glides from the first unused one of its text.
    pub(crate) fn arrange(&mut self, spots: &[Option<Spot>], before: &[(String, Spot)]) {
        let mut taken = vec![false; before.len()];
        let mut last: Option<Spot> = None;
        let (body, heading) = (self.preset, self.heading);
        for cue in &mut self.cues {
            let preset = if cue.block == 0 { heading } else { body };
            cue.spot = spots.get(cue.word.id).copied().flatten();
            let Some(spot) = cue.spot.filter(|_| cue.step == self.step) else {
                continue;
            };
            let text = &cue.word.run.text;
            if preset == Preset::Move
                && let Some(((_, was), taken)) = (before.iter().zip(&mut taken))
                    .find(|((t, _), taken)| !**taken && !text.is_empty() && t == text)
            {
                *taken = true;
                cue.from = Some(spot.from(*was));
            }
            if preset != Preset::Stack {
                continue;
            }
            // a new line once a word sits lower than half the one before
            if last.is_none_or(|l| spot.y > l.height.mul_add(0.5, l.y)) {
                self.lines.push((cue.word.id, 0.0));
            }
            if let Some(line) = self.lines.last_mut() {
                line.1 = line.1.max(spot.y + spot.height);
            }
            cue.line = self.lines.len() - 1;
            last = Some(spot);
        }
        self.schedule();
    }

    /// When every word comes in. The blocks (`stack`: the lines) come in on the beat after the
    /// one before has landed (`stack`: and been read); `move` glides first.
    fn schedule(&mut self) {
        let (half, cues) = (self.half, &self.cues);
        // half-beats a letter and a word take at the pace
        let at_pace = |per_minute: f64| {
            if self.pace > 0.0 {
                60.0 / (self.pace * per_minute) / half
            } else {
                0.0
            }
        };
        let letter = at_pace(LETTERS);
        let mut starts: Vec<_> = cues
            .iter()
            .map(|c| (c.step == self.step && c.from.is_some()).then_some((0.0, GLIDE)))
            .collect();
        let glide = if starts.iter().any(Option::is_some) {
            GLIDE
        } else {
            0.0
        };
        let (mut t, mut landed, mut end, mut mark, mut group) = (0.0, glide, glide, None, None);
        // the words of the group so far, and the half-beats each takes to read (`stack`)
        let (mut words, mut read) = (0.0_f64, 0.0);
        for (i, cue) in cues.iter().enumerate() {
            let preset = self.of(cue);
            let moving = !matches!(preset, Preset::Still | Preset::Flip);
            if cue.step != self.step || !moving || starts[i].is_some() {
                continue;
            }
            let stack = preset == Preset::Stack;
            let key = (stack, if stack { cue.line } else { cue.block });
            if group != Some(key) {
                let next = words.mul_add(read, landed);
                (t, group, words) = ((next / 2.0_f64).ceil() * 2.0, Some(key), 0.0);
                read = if stack { at_pace(1.0) } else { 0.0 };
            }
            if cue.word.run.pause {
                t += 2.0;
            }
            let len = match &cue.value {
                Some(_) => COUNT,
                None if preset.paced() => (cue.word.letters() + 1) as f64 * letter,
                None => preset.enter(cue.word.letters()),
            };
            starts[i] = Some((t, len));
            landed = landed.max(t + len);
            end = end.max(t + len);
            // a metric's bar draws as its value starts to count
            if cue.value.is_some() {
                end = end.max(t + LASER);
            }
            // the heading's marker (or bar) draws once the heading has landed; a stacked
            // heading's moves up with the words
            if cue.block == 0 && self.marked {
                mark = Some(landed);
            }
            words += 1.0;
            t += if preset.paced() {
                len + cue.breath()
            } else {
                f64::from(preset.stagger())
            };
        }
        let sweeps = self.sweeps(&starts);
        for sweep in sweeps.iter().flatten() {
            end = end.max(sweep.start + MARK);
        }
        // stack: the lines move up till their spring rests
        if !self.lines.is_empty() {
            let (from, moves) = self.lift(&starts);
            end = end.max(STACK.rests(from, &moves));
        }
        self.starts = starts;
        self.sweeps = sweeps;
        self.end = mark.map_or(end, |m| end.max(m + LASER));
        self.mark = mark;
    }

    /// `stack`: how far the lines sit lower at first, in points (the room of the lines still to
    /// come up), and from each line's start how far still: the moves of their spring.
    fn lift(&self, starts: &[Option<(f64, f64)>]) -> (f64, Vec<(f64, f64)>) {
        let rooms: Vec<(f64, f64)> = (self.lines.windows(2))
            .filter_map(|pair| {
                let ((_, above), (first, bottom)) = (pair[0], pair[1]);
                Some(((*starts.get(first)?)?.0, bottom - above))
            })
            .collect();
        let after = |k: usize| rooms[k..].iter().map(|r| r.1).sum::<f64>();
        let moves = (0..rooms.len()).map(|k| (rooms[k].0, after(k + 1)));
        (after(0), moves.collect())
    }

    /// When each marked word's mark draws: a run of words marked alike on one line draws as one
    /// sweep from the left, each word its share by letters, starting once every word has landed
    /// before the sweep reaches it.
    fn sweeps(&self, starts: &[Option<(f64, f64)>]) -> Vec<Option<Sweep>> {
        let cues = &self.cues;
        let mut sweeps = vec![None; cues.len()];
        let mut i = 0;
        while i < cues.len() {
            let fresh =
                |k: usize| starts[k].is_some() && cues[k].from.is_none() && cues[k].marked();
            if !fresh(i) {
                i += 1;
                continue;
            }
            let mut j = i + 1;
            while j < cues.len() && fresh(j) && alike(&cues[i], &cues[j]) {
                j += 1;
            }
            // each word its letters, the space after it too when the run goes on
            let widths: Vec<f64> = (i..j)
                .map(|k| (cues[k].word.letters() + usize::from(k + 1 < j)) as f64)
                .collect();
            let total = widths.iter().sum::<f64>().max(1.0);
            let mut x = 0.0;
            let mut start = 0.0_f64;
            let mut shares = Vec::new();
            for (k, width) in (i..j).zip(&widths) {
                let landed = starts[k].map_or(0.0, |(s, l)| s + l);
                start = start.max((x / total).mul_add(-MARK, landed));
                shares.push((x / total, (x + width) / total));
                x += width;
            }
            for (k, (from, to)) in (i..j).zip(shares) {
                sweeps[k] = Some(Sweep {
                    start: start + SETTLE,
                    from,
                    to,
                });
            }
            i = j;
        }
        sweeps
    }

    /// Seconds of the entrances.
    pub(crate) fn entrances(&self) -> f64 {
        self.end * self.half
    }

    /// Seconds the scene stays: the entrances, then its new words read at the pace (`type`,
    /// `karaoke` and `stack` read them coming in), at least four beats. The entrances end with the
    /// last animation (a mark, a count, a laser), so the finished card always rests that long.
    pub(crate) fn length(&self) -> f64 {
        let words = self.cues.iter().filter(|c| c.step == self.step).count() as f64;
        let read = if self.pace > 0.0 && !self.preset.read_in() {
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
        let draw = Curve::Ease.at(p(self.mark, BAR));
        // stack: the lines in so far sit lower by the room of the lines still coming up, moving
        // up on a spring that keeps its momentum as the next line comes
        let sink = if self.lines.is_empty() || now >= self.end {
            0.0
        } else {
            let (from, moves) = self.lift(&self.starts);
            STACK.follow(from, &moves, now).0
        };
        let (mut values, mut bars) = (Vec::new(), Vec::new());
        // the caret, and the half-beat the last typed word is typed to its end
        let (mut caret, mut typing) = (None, 0.0_f64);
        let looks: Vec<_> = self
            .cues
            .iter()
            .zip(&self.starts)
            .enumerate()
            .map(|(i, (cue, &start))| {
                let (at, len) = start.map_or((None, 1.0), |(s, l)| (Some(s), l));
                let mut letters = Vec::new();
                let mut look = match (&cue.value, cue.from) {
                    (Some(value), _) => {
                        values.push(count(value, Curve::Ease.at(p(at, COUNT))));
                        // its bar grows from the bottom as it starts to count; the bar moves
                        // with the value (film.typ reads its `y`)
                        bars.push(laser(
                            Curve::Ease.at(p(at, BAR)),
                            1.0 - Curve::Ease.at(p(at.map(|a| BAR.mul_add(0.5, a)), FLASH)),
                            0.0,
                        ));
                        Preset::Rise.look(p(at, ENTER))
                    },
                    (None, Some(from)) => from.toward(Curve::Ease.at(p(at, len))),
                    (None, None) => {
                        let graphemes = cue.word.graphemes();
                        let q = p(at, len);
                        // the caret sits after the letters typed so far of the word typed last
                        let preset = self.of(cue);
                        if preset == Preset::Type
                            && let Some(a) = at
                        {
                            typing = typing.max(a + len);
                            if a <= now {
                                caret = Some((cue.word.id, Preset::typed(q, graphemes.len())));
                            }
                        }
                        let word;
                        (word, letters) = shape(preset.letters(q, &graphemes, cue.word.id as u64));
                        word
                    },
                };
                if let Some(spot) = cue.spot.filter(|_| at.is_some()) {
                    look.y += sink / spot.size;
                }
                // its share of its run's sweep; a word that glides keeps its marks
                let sweep = self.sweeps[i];
                let start = sweep.map(|w| w.start);
                let share = sweep.map_or(1.0, |w| {
                    let q = Curve::Ease.at(p(start, MARK));
                    ((q - w.from) / (w.to - w.from)).clamp(0.0, 1.0)
                });
                let after = p(start, ENTER);
                let run = &cue.word.run;
                if run.bold || run.strike {
                    look.tint = share;
                }
                // a lone bold word pulses; a bold run only tints, its words would grow into
                // each other's spaces
                let bold = |j: Option<usize>| {
                    j.and_then(|j| self.cues.get(j))
                        .is_some_and(|c| c.block == cue.block && c.word.run.bold)
                };
                if run.bold && !bold(i.checked_sub(1)) && !bold(Some(i + 1)) {
                    // up on a spring over half a beat, back on one over the other half
                    let (up, back) = (
                        Curve::Bounce.at(2.0 * after),
                        Curve::Bounce.at(2.0f64.mul_add(after, -1.0)),
                    );
                    look.s *= 0.12f64.mul_add(up - back, 1.0);
                }
                if run.underline || run.highlight || run.strike {
                    look.draw = share;
                }
                if cue.block == 0 && self.marked {
                    look.draw = draw;
                }
                let mut json = look.json();
                if !letters.is_empty() {
                    json["l"] = letters.into_iter().map(Look::json).collect();
                }
                json
            })
            .collect();
        let mut state = json!({ "w": looks, "n": values });
        // the metrics' bars, while one still draws
        if bars.iter().any(|b| !b.is_empty()) {
            state["m"] = bars.into();
        }
        // the heading's bar: how far it is drawn, how hot its tip is (fading from halfway)
        // and how far down the stacked lines still hold it, in points
        let hot = 1.0 - Curve::Ease.at(p(self.mark.map(|m| BAR.mul_add(0.5, m)), FLASH));
        let bar = laser(draw, hot, sink);
        if self.marked && !bar.is_empty() {
            state["bar"] = bar.into();
        }
        if let Some(caret) = caret.filter(|_| now < typing && (now / 2.0).floor() % 2.0 == 0.0) {
            // the caret blinks a beat on, a beat off while letters are typed: its word, and the
            // letters before it; gone once the last word is typed, as its marks draw
            state["c"] = json!([caret.0, caret.1]);
        }
        state.to_string()
    }
}

/// A laser's look as JSON: how far it is drawn, how hot its tip is and how far down it is held,
/// in points; values rounded to 1e-4, a field at rest left out.
fn laser(draw: f64, hot: f64, dy: f64) -> serde_json::Map<String, serde_json::Value> {
    [("draw", draw, 1.0), ("hot", hot, 0.0), ("dy", dy, 0.0)]
        .into_iter()
        .map(|(k, v, rest)| (k, (v * 1e4).round() / 1e4, rest))
        .filter(|&(_, v, rest)| (v - rest).abs() > 5e-5)
        .map(|(k, v, _)| (k.to_string(), v.into()))
        .collect()
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
            (timeline.length() / 0.2).round() as u32
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
        // the bar draws over two beats from when the heading has landed
        assert_eq!((first.mark, first.end), (Some(3.0), 7.0));
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
    fn a_metrics_bar_is_hidden_till_its_value_comes_in_then_draws_as_a_laser() {
        // a heading word, then the metric on the beat after it has landed: half-beat 2
        let mut cues = words(&[0]);
        cues.push(Cue {
            block: 1,
            value: Some("47".into()),
            ..Cue::default()
        });
        let timeline = rise(cues);
        assert_eq!(starts(&timeline), [Some(0.0), Some(2.0)]);
        let bar = |t: f64| -> serde_json::Value {
            let state: serde_json::Value = serde_json::from_str(&timeline.state(t)).unwrap();
            state
                .get("m")
                .map_or_else(|| serde_json::json!({}), |m| m[0].clone())
        };
        // before its value: not drawn, the tip not lit yet
        assert_eq!(bar(0.0), serde_json::json!({ "draw": 0.0, "hot": 1.0 }));
        assert_eq!(bar(0.39)["draw"], 0.0);
        // drawing from the bottom as the value counts, hot while it draws
        let half = bar(0.6);
        let draw = half["draw"].as_f64().unwrap();
        assert!(draw > 0.5 && draw < 1.0, "{half}");
        assert_eq!(half["hot"], 1.0);
        // drawn over two beats, the tip cooled by then: at rest, and gone from the state
        assert_eq!(bar(0.4 + 0.8), serde_json::json!({}));
        assert!(!timeline.state(1.2).contains(r#""m":"#));
        // the value counts on to half-beat 8, after its bar
        assert_eq!(timeline.end, 2.0 + COUNT);
    }

    #[test]
    fn every_scene_rests_after_its_last_animation() {
        // each preset, a marked heading, marks and a metric: once nothing moves any more the
        // card rests at least four beats before the scene ends
        for preset in Preset::ALL {
            let mut cues = words(&[0, 0, 1, 1, 2]);
            cues[2].word.run.highlight = true;
            cues[3].word.run.bold = true;
            cues[4].value = Some("60 %".into());
            for (i, cue) in cues.iter_mut().enumerate() {
                cue.word.id = i;
            }
            let timeline = Timeline::new(preset, 0.4, 200.0, 0, cues, true);
            let fps = 100.0;
            let frames = (timeline.length() * fps) as usize;
            let last = (0..frames)
                .map(|k| k as f64 / fps)
                .collect::<Vec<_>>()
                .windows(2)
                .filter(|w| timeline.state(w[0]) != timeline.state(w[1]))
                .map(|w| w[1])
                .fold(0.0, f64::max);
            assert!(last <= timeline.entrances() + 1e-9, "{preset:?}");
            assert!(timeline.length() - last >= 1.6 - 1e-9, "{preset:?}");
        }
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
            assert!((timeline.length() - timeline.entrances() - 1.6).abs() < 1e-9);
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
        let mut cues: Vec<Cue> = (0..30).map(|_| cue(1, "word")).collect();
        for (i, cue) in cues.iter_mut().enumerate() {
            cue.word.id = i;
        }
        let timeline = Timeline::new(Preset::Type, 0.4, 200.0, 0, cues, false);
        let caret = |t: f64| timeline.state(t).contains(r#""c":"#);
        assert!(caret(0.0) && caret(0.39) && !caret(0.4) && !caret(0.79) && caret(0.8));
        // a word of 4 letters and its space at 200 wpm: 0.27 s; the caret after the letters
        // typed so far, of the word typed last
        let at = |t: f64| -> serde_json::Value {
            serde_json::from_str::<serde_json::Value>(&timeline.state(t)).unwrap()["c"].clone()
        };
        assert_eq!(
            (at(0.0), at(0.1), at(0.3)),
            (json!([0, 0]), json!([0, 1]), json!([1, 0]))
        );
        // the typed letters are at rest, the others not there yet
        let state: serde_json::Value = serde_json::from_str(&timeline.state(0.1)).unwrap();
        assert_eq!(
            state["w"][0],
            json!({ "l": [{}, { "o": 0.0 }, { "o": 0.0 }, { "o": 0.0 }] })
        );
        assert_eq!(state["w"][1], json!({ "o": 0.0 }));
        assert!(!caret(timeline.entrances()));
        // typed to its end, a mark still drawing: no caret
        let mut marked = vec![cue(1, "one"), cue(1, "two")];
        marked[1].word.id = 1;
        marked[1].word.run.highlight = true;
        let timeline = Timeline::new(Preset::Type, 0.4, 200.0, 0, marked, false);
        let typed = timeline.starts[1].map(|(s, l)| s + l).unwrap();
        assert!(timeline.end > typed + 2.0);
        for k in 0..20 {
            let t = f64::from(k).mul_add(0.1, typed) * 0.2;
            assert!(!timeline.state(t).contains(r#""c":"#), "{t}");
        }
        let rose = rise(words(&[1, 1]));
        assert!(!rose.state(0.0).contains(r#""c":"#));
    }

    #[test]
    fn the_heading_comes_in_by_its_own_preset_then_the_text_by_the_scenes() {
        let mut cues = vec![
            cue(0, "Ship"),
            cue(0, "small"),
            cue(1, "one"),
            cue(1, "two"),
        ];
        for (i, cue) in cues.iter_mut().enumerate() {
            cue.word.id = i;
        }
        let typed = Timeline::new(Preset::Type, 0.4, 200.0, 0, cues.clone(), true);
        let timeline = typed.clone().heading(Preset::Rise);
        // the heading rises word by word, half a beat apart, and lands at 3; its bar draws from
        // there, the text is typed from the beat after
        let at = starts(&timeline);
        assert_eq!((at[0], at[1], at[2]), (Some(0.0), Some(1.0), Some(4.0)));
        assert_eq!(timeline.mark, Some(3.0));
        let state =
            |t: f64| -> serde_json::Value { serde_json::from_str(&timeline.state(t)).unwrap() };
        assert_eq!(
            state(0.0)["w"][0],
            json!({ "draw": 0.0, "o": 0.0, "y": 0.5 })
        );
        // no caret in a rising heading; one in the typed text
        assert!(state(0.4).get("c").is_none());
        assert_eq!(state(0.8)["c"], json!([2, 0]));
        // the same preset for both is the timeline as it was
        let same = typed.clone().heading(Preset::Type);
        assert_eq!((starts(&same), same.end), (starts(&typed), typed.end));
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
        // all land at 2 half-beats; the marks draw half a half-beat later, over 3.5
        assert_eq!(timeline.end, 6.0);
        let look = |t: f64, i: usize| {
            let state: serde_json::Value = serde_json::from_str(&timeline.state(t)).unwrap();
            state["w"][i].clone()
        };
        assert_eq!(look(0.5, 0)["tint"], 0.0);
        assert_eq!(look(0.5, 1)["draw"], 0.0);
        assert_eq!(look(0.7, 0)["s"], 1.12);
        assert!(look(0.7, 3)["tint"].as_f64().unwrap() > 0.5);
        for i in 0..4 {
            assert_eq!(look(1.2, i), serde_json::json!({}), "{i}");
        }
    }

    #[test]
    fn a_bold_run_tints_without_a_pulse() {
        let mut cues = words(&[1, 1]);
        for cue in &mut cues {
            cue.word.run.bold = true;
        }
        let timeline = Timeline::new(Preset::Fade, 0.4, 200.0, 0, cues, false);
        let state =
            |t: f64| -> serde_json::Value { serde_json::from_str(&timeline.state(t)).unwrap() };
        for t in [0.6, 1.0] {
            for i in 0..2 {
                assert!(state(t)["w"][i].get("s").is_none(), "{i}");
            }
        }
        for i in 0..2 {
            assert!(
                state(1.0)["w"][i]["tint"].as_f64().unwrap_or(1.0) > 0.9,
                "{i}"
            );
        }
    }

    #[test]
    fn a_run_of_marks_draws_as_one_sweep_from_the_left() {
        let mut cues = vec![
            cue(1, "one"),
            cue(1, "two"),
            cue(1, "three"),
            cue(1, "plain"),
        ];
        for cue in &mut cues[..3] {
            cue.word.run.highlight = true;
        }
        let timeline = rise(cues);
        // rise: the words land at 2, 3 and 4 half-beats; the sweep reaches each after it has,
        // so it starts once the first has: one start for the run
        let start = 2.0 + SETTLE;
        let starts: Vec<_> = timeline.sweeps.iter().map(|w| w.map(|w| w.start)).collect();
        assert_eq!(starts, [Some(start), Some(start), Some(start), None]);
        let draw = |t: f64, i: usize| {
            let state: serde_json::Value = serde_json::from_str(&timeline.state(t)).unwrap();
            state["w"][i]["draw"].as_f64().unwrap_or(1.0)
        };
        // part way: the first word drawn further than the second, the third not begun
        let t = (start + 0.6) * 0.2;
        assert!(
            draw(t, 0) > draw(t, 1) && draw(t, 2) == 0.0,
            "{} {} {}",
            draw(t, 0),
            draw(t, 1),
            draw(t, 2)
        );
        assert_eq!(timeline.end, start + MARK);
    }

    /// A word box of 10 pt text at (`x`, `y`).
    fn at(x: f64, y: f64) -> Spot {
        Spot {
            x,
            y,
            width: 30.0,
            height: 7.0,
            size: 10.0,
        }
    }

    #[test]
    fn move_glides_the_shared_words_first_then_the_others_come_in() {
        let mut cues = vec![cue(1, "a"), cue(1, "new"), cue(1, "b")];
        for (i, cue) in cues.iter_mut().enumerate() {
            cue.word.id = i;
        }
        let mut timeline = Timeline::new(Preset::Move, 0.4, 200.0, 0, cues, false);
        let before = [("b".to_string(), at(0.0, 0.0))];
        let spots = [at(0.0, 20.0), at(40.0, 20.0), at(80.0, 20.0)];
        timeline.arrange(&spots.map(Some), &before);
        // "b" glides 80 pt left, 20 pt up: 8 em and 2 em at 10 pt
        assert_eq!(
            starts(&timeline),
            [Some(4.0), Some(5.0), Some(0.0)],
            "the others on the beat after the glide"
        );
        let state: serde_json::Value = serde_json::from_str(&timeline.state(0.0)).unwrap();
        assert_eq!(state["w"][2], serde_json::json!({ "x": -8.0, "y": -2.0 }));
        assert_eq!(state["w"][0]["o"], 0.0);
    }

    #[test]
    fn stack_brings_line_after_line_and_the_lines_in_move_up() {
        let mut cues = words(&[1, 1, 2]);
        for (i, cue) in cues.iter_mut().enumerate() {
            cue.word.id = i;
        }
        let mut timeline = Timeline::new(Preset::Stack, 0.4, 200.0, 0, cues, false);
        let spots = [at(0.0, 0.0), at(40.0, 0.0), at(0.0, 12.0)];
        timeline.arrange(&spots.map(Some), &[]);
        // line one: 2 words take 0.6 s = 3 half-beats to read after landing at 2: on to 6
        assert_eq!(starts(&timeline), [Some(0.0), Some(0.0), Some(6.0)]);
        let y = |t: f64, i: usize| {
            let state: serde_json::Value = serde_json::from_str(&timeline.state(t)).unwrap();
            state["w"][i]["y"].as_f64().unwrap_or(0.0)
        };
        // the first line sits a line (12 pt) lower till the second comes up, then moves up on
        // its spring, a hair past its place, and rests there by the end
        assert_eq!((y(0.0, 0), y(1.2, 0), y(1.2, 2)), (1.2, 1.2, 1.2));
        assert!(y(1.4, 0) < 1.2 && y(1.4, 0) > 0.0);
        let past = (0..40)
            .map(|k| y(f64::from(k).mul_add(0.05, 1.2), 0))
            .fold(1.0, f64::min);
        assert!(past < 0.0 && past > -0.02, "{past}");
        let rest = STACK.rests(12.0, &[(6.0, 0.0)]);
        assert!((timeline.end - rest).abs() < 1e-9 && rest > 8.0);
        assert_eq!(y(timeline.entrances(), 0), 0.0);
    }

    #[test]
    fn stack_draws_each_lines_marks_once_that_line_has_landed() {
        let mut cues = words(&[1, 1, 2]);
        for (i, cue) in cues.iter_mut().enumerate() {
            cue.word.id = i;
        }
        cues[0].word.run.underline = true;
        cues[2].word.run.highlight = true;
        let mut timeline = Timeline::new(Preset::Stack, 0.4, 200.0, 0, cues, false);
        let spots = [at(0.0, 0.0), at(40.0, 0.0), at(0.0, 12.0)];
        timeline.arrange(&spots.map(Some), &[]);
        let look = |t: f64, i: usize| -> serde_json::Value {
            let state: serde_json::Value = serde_json::from_str(&timeline.state(t)).unwrap();
            state["w"][i].clone()
        };
        // line one lands at 2 half-beats and draws its mark before line two comes in at 6,
        // still a line lower: the mark moves with it
        let first = timeline.sweeps[0].unwrap().start;
        assert_eq!(first, 2.0 + SETTLE);
        assert_eq!(look(1.2, 0), serde_json::json!({ "y": 1.2 }));
        // line two's mark waits for line two
        assert_eq!(timeline.sweeps[2].unwrap().start, 8.0 + SETTLE);
        assert_eq!(look(1.6, 2)["draw"], 0.0);
        assert_eq!(timeline.end, 8.0 + SETTLE + MARK);
    }
}
