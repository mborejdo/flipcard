# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow semver.

## 0.1.0 - unreleased

### Added

- `flipcard <deck.md>`: a looping GIF, `--mp4`, `--docx`, `--pdf`, `--cards DIR`, `--out DIR`,
  `--preview` (WezTerm or iTerm2), timing flags `--fps --width --hold --enter --fade
  --transition --pace --colours`; `flipcard lint`; `flipcard config`.
- `flipcard.toml`: `fonts`, `logo`, `[theme]`, `[film]`; the closest file wins.
- `flipkit`, the film as a library: `Config`, `Look`, `Settings`, `Options`, `render`, `check`,
  `Rendered` with `gif`, `mp4`, `docx`, `pdf`, `cards`.
- `filmstrip` from presiterm, with its history. Word file links in a fixed order, so a deck with
  several links gives the same bytes every run.

The film was `presiterm film` up to presiterm 0.2.0; same deck, same bytes here.
