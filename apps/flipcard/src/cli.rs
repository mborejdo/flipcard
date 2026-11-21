//! The command line: `Cli`, `Command` and the `--help` texts.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

/// What the top of `--help` says.
const ABOUT: &str = "A Markdown deck in, a flip-card film out

Every slide a card drawn by Typst, 16:9, 9:16 or 1:1, every key press (a `+` bullet, a lit `steps` node) a \
build-in. As a looping GIF and an MP4.";

const ROOT_FOOTER: &str = "\
Examples:
  flipcard talk/slides.md                       talk/slides.gif
  flipcard talk/slides.md --gif --mp4           both
  flipcard talk/slides.md --out dist            into dist/ instead of next to the deck
  flipcard talk/slides.md --cards dist/cards    every card fully built as PNG (card03-007.png: slide 3, scene 7)
  flipcard talk/slides.md --preview             play the GIF inline in WezTerm or iTerm2
  flipcard talk/slides.md --width 960 --fps 12 --hold 5 --enter 1.2 --fade 0.3 --pace 160 --colours 64
  flipcard talk/slides.md --transition wipe     or push, pixelize, circle, split, cut
  flipcard talk/slides.md --format tall --mp4   a 9:16 film for phones, 1080x1920 at 30 fps
  flipcard talk/slides.md --format tall --safe none --gif-width 270 --gif-fps 10
  flipcard talk/slides.md --format tall --preset slide
                                                words in from the left: rise, slide, fade, pop, still, type, karaoke, flip
  flipcard lint talk/slides.md                  does every card fit
  flipcard config talk/slides.md                which flipcard.toml applies, and its values

A flip build-in is `enter` seconds of animation, then `hold` seconds of rest; the other presets
bring the words in on a beat and hold them for their reading time. Frontmatter (format, safe,
preset, fps, width, hold, enter, fade, transition, pace, colours, gif-width, gif-fps) and `[film]`
in flipcard.toml set the same values; a flag beats both, and the format brings its own defaults
under them. A card that doesn't fit is an error naming the slide; `flipcard lint` finds them all.

Exit codes:
  0  written
  2  a card overflows, a file can't be drawn or written (reason: one line on stderr)";

const LINT_FOOTER: &str = "\
Examples:
  flipcard lint talk/slides.md       one line per card that overflows, and per tall slide
                                     with more than 12 words (a warning)

Exit codes:
  0  every card fits
  1  at least one card overflows
  2  the deck can't be read or drawn";

const CONFIG_FOOTER: &str = "\
Examples:
  flipcard config                    look up from the current folder
  flipcard config talk/slides.md     look up from the deck's folder

The closest flipcard.toml from the deck's folder up wins, farther ones are ignored.

Exit codes:
  0  printed
  2  the file can't be read, has an unknown key or a malformed value";

/// The `flipcard` command line: a deck to render, or a subcommand.
#[derive(Parser)]
#[command(
    name = "flipcard",
    version,
    about = ABOUT,
    after_help = ROOT_FOOTER,
    args_conflicts_with_subcommands = true
)]
pub(crate) struct Cli {
    #[command(flatten)]
    pub(crate) film: FilmArgs,
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

/// The flags of a render.
#[derive(clap::Args)]
pub(crate) struct FilmArgs {
    /// The deck, e.g. talk/slides.md.
    pub(crate) file: Option<PathBuf>,
    /// Write the animated GIF (the default without --mp4).
    #[arg(long)]
    pub(crate) gif: bool,
    /// Write the MP4 (pure-Rust H.264).
    #[arg(long)]
    pub(crate) mp4: bool,
    /// Also write every card, fully built, as PNG here (--cards dist/cards).
    #[arg(long, value_name = "DIR")]
    pub(crate) cards: Option<PathBuf>,
    /// Write into this folder instead of next to the deck (--out dist).
    #[arg(long, value_name = "DIR")]
    pub(crate) out: Option<PathBuf>,
    /// Play the GIF inline in WezTerm or iTerm2 after writing.
    #[arg(long)]
    pub(crate) preview: bool,
    /// The canvas: wide (16:9), tall (9:16) or square (1:1); default wide.
    #[arg(long, value_name = "NAME")]
    pub(crate) format: Option<flipkit::Format>,
    /// The area the content stays in: none or phone (clear of a phone app's bars and buttons);
    /// default phone for tall, none otherwise.
    #[arg(long, value_name = "NAME")]
    pub(crate) safe: Option<flipkit::Safe>,
    /// How the words come in: flip, rise, slide, fade, pop, still, type or karaoke; default flip, rise for
    /// tall. A scene sets its own with `:::{scene} pop`.
    #[arg(long, value_name = "NAME")]
    pub(crate) preset: Option<flipkit::Preset>,
    /// Frames per second (--fps 12); default 10, 30 for tall and square.
    #[arg(long, value_name = "N")]
    pub(crate) fps: Option<u16>,
    /// Width in pixels, even (--width 960); default 640, 1080 for tall and square; the height
    /// follows the format.
    #[arg(long, value_name = "PX")]
    pub(crate) width: Option<u32>,
    /// Seconds every scene stays at least, build-in included (--hold 5); default 7 for flip.
    #[arg(long, value_name = "S")]
    pub(crate) hold: Option<f32>,
    /// Seconds of build-in at the start of every scene (--enter 1.2); default 1.6.
    #[arg(long, value_name = "S")]
    pub(crate) enter: Option<f32>,
    /// Seconds a transition between scenes takes (--fade 0.3); default 0.4.
    #[arg(long, value_name = "S")]
    pub(crate) fade: Option<f32>,
    /// How one scene gives way to the next: fade, wipe, push, pixelize, circle, split or cut
    /// (--transition wipe); default fade.
    #[arg(long, value_name = "NAME")]
    pub(crate) transition: Option<flipkit::Transition>,
    /// Words per minute a wordy scene is held for, 0 for a fixed hold (--pace 160); default 200.
    #[arg(long, value_name = "WPM")]
    pub(crate) pace: Option<f32>,
    /// Colours of the GIF palette, 2 to 255 (--colours 64); default 48.
    #[arg(long, value_name = "N")]
    pub(crate) colours: Option<usize>,
    /// Width of the GIF, the film scaled down by a whole factor; default the width, 360 for tall
    /// and square.
    #[arg(long, value_name = "PX")]
    pub(crate) gif_width: Option<u32>,
    /// Frames per second of the GIF, every n-th frame of the film; default the fps, 15 for tall
    /// and square.
    #[arg(long, value_name = "N")]
    pub(crate) gif_fps: Option<u16>,
}

impl FilmArgs {
    /// The timing and size flags as settings, to lay over the frontmatter and the config.
    pub(crate) const fn settings(&self) -> flipkit::Settings {
        flipkit::Settings {
            format: self.format,
            safe: self.safe,
            preset: self.preset,
            fps: self.fps,
            width: self.width,
            hold: self.hold,
            enter: self.enter,
            fade: self.fade,
            transition: self.transition,
            pace: self.pace,
            colours: self.colours,
            gif_width: self.gif_width,
            gif_fps: self.gif_fps,
        }
    }
}

/// The subcommands.
#[derive(Subcommand)]
pub(crate) enum Command {
    /// Draw every card fully built and report the ones that overflow.
    #[command(after_help = LINT_FOOTER)]
    Lint {
        /// The deck, e.g. talk/slides.md.
        file: PathBuf,
    },
    /// Show which flipcard.toml applies and its values.
    #[command(after_help = CONFIG_FOOTER)]
    Config {
        /// A deck (or its folder) to look up from; default: the current folder.
        path: Option<PathBuf>,
    },
}

#[cfg(test)]
mod tests {
    use clap::CommandFactory;

    use super::*;

    fn help_of(command: &clap::Command) -> String {
        command.clone().render_long_help().to_string()
    }

    #[test]
    fn cli_definition_is_consistent() {
        Cli::command().debug_assert();
    }

    #[test]
    fn every_help_has_examples_and_exit_codes_in_keyboard_characters() {
        let root = Cli::command();
        let mut commands = vec![root.clone()];
        commands.extend(
            root.get_subcommands()
                .filter(|c| c.get_name() != "help")
                .cloned(),
        );
        for command in commands {
            let help = help_of(&command);
            assert!(help.contains("Examples:"), "{}", command.get_name());
            assert!(help.contains("Exit codes:"), "{}", command.get_name());
            assert!(help.is_ascii(), "{}", command.get_name());
        }
    }

    #[test]
    fn every_flag_has_an_example() {
        let root = Cli::command();
        let help = help_of(&root);
        for arg in root.get_arguments() {
            let Some(long) = arg.get_long().filter(|l| !matches!(*l, "help" | "version")) else {
                continue;
            };
            let flag = format!("--{long}");
            let in_flag_help = arg
                .get_help()
                .is_some_and(|h| h.to_string().contains(&flag));
            let in_footer = help
                .lines()
                .filter(|l| l.trim_start().starts_with("flipcard "))
                .any(|l| l.contains(&flag));
            assert!(in_flag_help || in_footer, "{flag} has no example");
        }
    }
}
