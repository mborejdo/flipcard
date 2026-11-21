//! Time in a scene, in whole half-beats. The blocks come in order, each on the beat after the one
//! before has landed: a block says next, never when. In a block the words follow each other by
//! the preset's stagger; `//` waits a beat. The heading's marker draws once its words have landed.

use serde_json::json;

use super::{Curve, Preset, count};

/// Half-beats a word takes to come in.
const ENTER: u32 = 2;
/// Half-beats a metric counts up.
const COUNT: u32 = 6;
/// Half-beats of rest after the entrances, at least.
const REST: u32 = 8;

/// One word of a card, in reading order.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Cue {
    /// Its block; 0 is the heading.
    pub block: usize,
    /// The key press that brings it.
    pub step: usize,
    /// A `//` before it.
    pub pause: bool,
    /// A metric's value, which counts up.
    pub value: Option<String>,
}

/// When the words of one scene come in, and how they look at any instant.
#[derive(Clone, Debug)]
pub(crate) struct Timeline {
    preset: Preset,
    /// Seconds per half-beat.
    half: f64,
    cues: Vec<Cue>,
    /// The half-beat each word starts to come in; `None` for a word already there.
    starts: Vec<Option<u32>>,
    /// The half-beat the heading's marker starts to draw, if it has one and it comes in.
    mark: Option<u32>,
    marked: bool,
    /// The half-beat the last entrance ends.
    pub end: u32,
}

impl Timeline {
    /// The words of `cues` that come in at `step`, a `beat` seconds long; `marked`: the heading
    /// has a marker.
    pub(crate) fn new(
        preset: Preset,
        beat: f64,
        step: usize,
        cues: Vec<Cue>,
        marked: bool,
    ) -> Self {
        let mut starts = vec![None; cues.len()];
        let (mut t, mut end, mut mark, mut block) = (0_u32, 0_u32, None, None);
        let moving = !matches!(preset, Preset::Still | Preset::Flip);
        for (i, cue) in cues.iter().enumerate() {
            if cue.step != step || !moving {
                continue;
            }
            if block != Some(cue.block) {
                if block == Some(0) && marked {
                    mark = Some(end);
                }
                (t, block) = (end.next_multiple_of(2), Some(cue.block));
            }
            if cue.pause {
                t += 2;
            }
            starts[i] = Some(t);
            end = end.max(t + if cue.value.is_some() { COUNT } else { ENTER });
            t += preset.stagger();
        }
        if block == Some(0) && marked {
            mark = Some(end);
        }
        Self {
            preset,
            half: beat / 2.0,
            starts,
            end: mark.map_or(end, |m| end.max(m + ENTER)),
            mark,
            marked,
            cues,
        }
    }

    /// Seconds of the entrances.
    pub(crate) fn entrances(&self) -> f64 {
        f64::from(self.end) * self.half
    }

    /// Seconds the scene stays: the entrances, then its new words read at `pace` words a
    /// minute, at least four beats.
    pub(crate) fn length(&self, step: usize, pace: f64) -> f64 {
        let words = self.cues.iter().filter(|c| c.step == step).count() as f64;
        let read = if pace > 0.0 { words * 60.0 / pace } else { 0.0 };
        let read = ((read / self.half).ceil() as u32).max(REST);
        f64::from(self.end + read) * self.half
    }

    /// The looks of every word and the metrics' values at `t` seconds, as the JSON `word()` reads.
    pub(crate) fn state(&self, t: f64) -> String {
        // in half-beats, snapped so a frame on a half-beat lands on it exactly
        let now = (t / self.half * 1e6).round() / 1e6;
        let p = |start: Option<u32>, len: u32| {
            start.map_or(1.0, |s| {
                ((now - f64::from(s)) / f64::from(len)).clamp(0.0, 1.0)
            })
        };
        let draw = Curve::Strong.at(p(self.mark, ENTER));
        let mut values = Vec::new();
        let looks: Vec<_> = self
            .cues
            .iter()
            .zip(&self.starts)
            .map(|(cue, &start)| {
                let preset = if cue.value.is_some() {
                    Preset::Rise
                } else {
                    self.preset
                };
                let counted = cue
                    .value
                    .as_ref()
                    .map(|v| count(v, Curve::Ease.at(p(start, COUNT))));
                values.extend(counted);
                let mut look = preset.look(p(start, ENTER));
                if cue.block == 0 && self.marked {
                    look.draw = draw;
                }
                look.json()
            })
            .collect();
        json!({ "w": looks, "n": values }).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words(blocks: &[usize]) -> Vec<Cue> {
        blocks
            .iter()
            .map(|&block| Cue {
                block,
                ..Cue::default()
            })
            .collect()
    }

    #[test]
    fn blocks_come_in_on_the_beat_after_the_one_before_has_landed() {
        // heading of 3 words: starts 0, 1, 2, lands at 4; block 1 on beat 2 (half-beat 4)
        let timeline = Timeline::new(Preset::Rise, 0.4, 0, words(&[0, 0, 0, 1, 1]), false);
        assert_eq!(
            timeline.starts,
            [Some(0), Some(1), Some(2), Some(4), Some(5)]
        );
        assert_eq!(timeline.end, 7);
        let faded = Timeline::new(Preset::Fade, 0.4, 0, words(&[0, 0, 1]), false);
        assert_eq!(faded.starts, [Some(0), Some(0), Some(2)]);
    }

    #[test]
    fn the_hold_is_the_entrances_then_the_reading_time_at_least_four_beats() {
        let timeline = Timeline::new(Preset::Rise, 0.4, 0, words(&[0, 0, 0, 1, 1]), false);
        // 5 words at 200 wpm: 1.5 s = 7.5 half-beats, so 8, the minimum; at 60 wpm 5 s = 25
        let half_beats = |pace| (timeline.length(0, pace) / 0.2).round() as u32;
        assert_eq!(
            (half_beats(200.0), half_beats(60.0), half_beats(0.0)),
            (7 + 8, 7 + 25, 7 + 8)
        );
        let still = Timeline::new(Preset::Still, 0.4, 0, words(&[0, 1]), false);
        assert_eq!((still.end, still.starts), (0, vec![None, None]));
    }

    #[test]
    fn a_pause_waits_one_beat() {
        let mut cues = words(&[0, 0, 0]);
        cues[2].pause = true;
        let timeline = Timeline::new(Preset::Rise, 0.4, 0, cues, false);
        assert_eq!(timeline.starts, [Some(0), Some(1), Some(4)]);
    }

    #[test]
    fn a_later_step_brings_only_its_words_and_the_marker_draws_after_the_heading() {
        let mut cues = words(&[0, 0, 1, 1]);
        cues[3].step = 1;
        let first = Timeline::new(Preset::Rise, 0.4, 0, cues.clone(), true);
        assert_eq!(first.mark, Some(3));
        assert_eq!(first.end, 6);
        let second = Timeline::new(Preset::Rise, 0.4, 1, cues, true);
        assert_eq!(
            (second.starts.clone(), second.mark),
            (vec![None, None, None, Some(0)], None)
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
        let timeline = Timeline::new(Preset::Fade, 0.4, 0, cues, false);
        assert_eq!(timeline.end, 6);
        assert!(timeline.state(0.0).contains(r#""n":["0"]"#));
        assert!(timeline.state(1.2).contains(r#""n":["1 200"]"#));
    }
}
