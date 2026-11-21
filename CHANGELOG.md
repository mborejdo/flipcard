# Changelog

Format: [Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow semver.

## 0.1.0 - unreleased

### Added

- `flipcard <deck.md>`: the deck as a looping GIF, `--mp4`, `--docx`, `--pdf`, `--cards DIR`,
  `--out DIR`, `--preview` (WezTerm or iTerm2), and the timing flags `--fps --width --hold
  --enter --fade --transition --pace --colours`; `flipcard lint`; `flipcard config`.
- `flipcard.toml`: `fonts`, `logo`, `[theme]`, `[film]`; the closest file wins.
- `flipkit`: the film as a library (`Config`, `Look`, `Settings`, `Options`, `render`, `check`,
  `Rendered` with `gif`, `mp4`, `docx`, `pdf`, `cards`).
- `filmstrip`, with its history, from presiterm. The links of a Word file are in a fixed order,
  so a deck with several links gives the same bytes every run.

The film was `presiterm film` up to presiterm 0.2.0; the same deck gives the same bytes here.
