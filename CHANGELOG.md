# Changelog

All notable changes to this project are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/), and the project follows
[Semantic Versioning](https://semver.org/). All tools in the workspace share one
version.

## [Unreleased]

## [0.7.2] - 2026-06-25

### Fixed

- **CI** — corrected an invalid key in `deny.toml` (`wildcard-dependencies` →
  `wildcards`, the real cargo-deny `[bans]` setting). The stricter cargo-deny in
  `cargo-deny-action@v2` rejected the unknown key and failed to parse the config;
  the local gate skips cargo-deny when it is not installed, so this went unseen.

## [0.7.1] - 2026-06-25

### Changed

- **Packaging** — the GUI desktop entry's `Name` now leads with the suite brand
  (`Rackctl US-16x08 Mixer`) instead of fronting the Tascam trademark, matching
  the rest of the project after the rename. Discoverability is unchanged — the
  entry's `Comment` and `Keywords` still carry "Tascam"/"us-16x08".

## [0.7.0] - 2026-06-25

### Changed

- **BREAKING — renamed the project to Rackctl**, a suite of Linux utilities for
  controlling outboard audio hardware. The Tascam US-16x08 tools are its first
  member (a BOSS GX-700 is planned). Renames:
  - binaries `tascamctl` → `rackctl-us16x08`, `tascam-mixer` → `rackctl-us16x08-gui`;
  - crates `tascam-us16x08`/`tascam-cli`/`tascam-gui` →
    `rackctl-us16x08`/`rackctl-us16x08-cli`/`rackctl-us16x08-gui`
    (in `crates/us16x08`, `crates/us16x08-cli`, `crates/us16x08-gui`);
  - repository `malo-rte/tascam-mixer` → `malo-rte/rackctl`.

  Settings move to `~/.config/rackctl/us16x08/` (the default preset, scenes, and
  per-section presets) and are migrated automatically from the previous
  `~/.config/tascam-mixer/` on first run — saved presets and the default carry
  over — also dropping the inherited `paraair` namespace. Update any scripts,
  udev rules, or systemd units that referenced the old binary names. "Tascam
  US-16x08" remains in the tools' descriptive text, naming the hardware they
  control.
- **License/credits** — the `LICENSE` file now carries this project's own MIT
  license (© 2026 Mats Loman) instead of the inherited upstream copyright, and the
  workspace `license` field is corrected from `ISC` to `MIT` to match. The README
  gains an acknowledgment of [tascam-gtk](https://github.com/onkelDead/tascam-gtk)
  by onkelDead, the (independently reimplemented, not copied) inspiration for this
  project, and a trademark disclaimer.

### Added

- **Packaging** — a Nix flake (`flake.nix`): `nix run` the GUI or
  `nix run .#rackctl-us16x08` the CLI, `nix build`, and `nix develop` for a dev
  shell with the Rust toolchain and the eframe system libraries. The package
  version is read from the workspace `Cargo.toml`, and the GUI binary is wrapped
  so it finds the Wayland/GL/xkbcommon libraries it loads at run time.
- **CLI** — `rackctl-us16x08 completions <shell>` prints a shell completion script
  (bash, zsh, fish, elvish, or powershell) generated from the command definition,
  so it always matches the current commands and flags.
- **Packaging** — a desktop entry (`packaging/rackctl-us16x08-gui.desktop`) for
  launching the GUI from a desktop menu, with `packaging/README.adoc` covering a
  manual install of the binaries, the desktop entry, and the completions. The Nix
  package installs the completions and the desktop entry automatically.

## [0.6.0] - 2026-06-25

### Fixed

- **GUI** — the continuous device reads (meters, and the periodic re-read of the
  whole control surface) now run on a background thread instead of the UI thread.
  Those ~280-control reads previously blocked the UI thread — which is also the
  Wayland event loop — long enough that compositors (e.g. Hyprland) sometimes
  flagged the window as "not responding," especially around a workspace switch.
  The UI thread now shares the device with the reader thread and only touches it
  for brief user-initiated writes and loads, so it stays responsive to the
  compositor's keep-alive pings.

### Changed

- **GUI** — copy/paste now uses one layered clipboard instead of separate
  channel/EQ/compressor buffers. *Copy channel* puts the whole strip on it; *Copy*
  in the EQ or Compressor box overlays just that section. Each *Paste* applies only
  the part it handles, and only controls that were actually copied are written. So
  you can copy a channel, overlay another channel's EQ, and paste the combination
  onto many channels.

### Added

- **GUI** — meter peak-hold and a clip indicator: each level meter (channels and
  master) shows a held peak marker that holds, then falls back slowly; it turns
  red when the held peak reached clipping, and a red cap latches at the top of the
  meter when it hits full scale.
- **GUI** — `tascam-mixer` now parses its arguments with clap, so `--version` and
  `--help` work (alongside the existing `--mock`).

## [0.5.0] - 2026-06-25

### Added

- **GUI** — solo: an *S* button per channel in the meter bridge (and a *Solo*
  switch in the editor, `s` shortcut). While any channel is soloed the others are
  muted; clearing solo restores the mutes that were in place before. A GUI-only
  monitoring aid; linked pairs solo together.
- **GUI** — channel names: name each of the 16 input channels (e.g. "Kick",
  "Vox") in the INPUT box. Names are GUI-only, remembered between sessions, and
  shown in the meter bridge.

## [0.4.0] - 2026-06-25

### Added

- **CLI** — `tascamctl save --section eq|comp` saves just a channel's EQ or
  compressor section (a partial strip preset), matching the GUI's EQ/Comp preset
  tabs. `load` already applies such a preset with `-c`, setting only the controls
  it holds. The library gains a `Section` type and `Us16x08::capture_section`.
- **Releases** — pushing a `vX.Y.Z` tag now builds the binaries and publishes a
  GitHub Release automatically (with notes from this changelog and a tarball of
  `tascamctl` + `tascam-mixer`), via a new release workflow.

## [0.3.0] - 2026-06-25

### Added

- **GUI** — a *Scenes* tab: save the whole mixer as a named snapshot and recall
  it later. Scenes are stored as ordinary preset files in the settings directory
  (no fixed limit), each with Save (overwrite in place), Load, and Delete;
  equivalent to a hardware mixer's scene memories, kept on the host.
- **GUI** — a *Channel presets* tab: the single-channel counterpart of scenes.
  Save and recall one channel's settings (strip presets) from the settings
  directory; saves from the focused channel and loads onto it, so it doubles as a
  quick way to copy settings between channels.
- **GUI** — copy/paste between channels: *Copy channel* / *Paste channel* above
  the editor for the whole strip, and *Copy* / *Paste* in the EQ and Compressor
  title rows for just that section. Pasting onto a linked pair applies to both.
- **GUI** — *EQ presets* and *Comp presets* tabs: save and recall a single
  channel's EQ or compressor section, like the channel-presets tab but per
  section. Stored as partial strip presets, so `tascamctl load` can apply one to
  a channel too.
- **GUI** — a *Copy* button on each channel/EQ/compressor preset row that copies
  the preset into the editor's copy/paste clipboard, ready to paste onto channels.
- **GUI** — a *Reset channel* button that returns the whole focused channel to a
  neutral default (flat EQ, no compression, centre pan, fader fully down at
  -127 dB, switches off), alongside the existing per-section EQ and Compressor
  resets.

### Removed

- **GUI** — the toolbar's *Presets* file-dialog menu, superseded by the Scenes
  and Channel-presets tabs. (Drops the `rfd` dependency.)

### Fixed

- **GUI** — loading or pasting a channel, EQ, or compressor preset onto a linked
  channel now applies it to both halves of the pair and keeps the pair
  hard-panned, instead of changing only one side or collapsing the stereo image.
- **GUI** — the stereo-link grouping is now saved inside every whole-mixer preset
  and scene (as an extra `links` field that `tascamctl` ignores), so loading a
  preset, scene, or the default restores which pairs are linked. Previously the
  grouping only travelled with the shared default.

## [0.2.1] - 2026-06-24

### Fixed

- **Reliable, silent full-mixer loads on a settling device.** Loading a whole
  mixer (the GUI replug restore, *Load default* / *Load mixer*, and `tascamctl
  load` / `default`) used to write each control once and ignore failures, so a
  just-re-enumerated card — which answers reads while still silently dropping
  writes — kept its power-up state (often muted), and the restore "did not always"
  take or left the device silent. A full load now waits until a control write
  actually round-trips, then applies the whole mixer as one transaction: mute the
  master, write every control, and finally set the master mute to its loaded
  value. If a write errors, the body restarts from muting the master; and the
  master mute is *always* restored to its target at the end — even if the body
  could not be fully written — so a failed or interrupted load is never left
  silent. (A device that drops in and out on the USB bus can still interrupt a
  load, but it will not be stuck muted.) Each control write is also paced (10 ms
  apart) so sending the whole mixer back-to-back does not outrun the device's USB
  control channel and silently drop values.

## [0.2.0] - 2026-06-24

### Added

- **GUI** — the mixer now survives the interface being unplugged. While the
  device is gone the controls are hidden behind a centred "Tascam US-16x08 is
  disconnected" notice and a "reconnecting" status; it retries about once a
  second, and on replug reopens the card and re-applies the on-screen mix (which
  the re-enumerated device would otherwise have reset to its defaults).
- **GUI** — the mixer also starts when the card is absent, showing the
  disconnected notice until it appears. On that first connection it applies the
  saved default preset (or, with none saved, reads the device as-is); when the
  card is already running at startup it just reads its current settings.

### Changed

- **CLI** — clearer `--help`: a short summary and command list with `-h`, fuller
  per-command detail with `--help`, an examples section, and a readable breakdown
  of the `set` value forms. Help and error output is now colorized on a terminal
  (and stays plain when piped or redirected).

### Documentation

- **Manual** — a recipe for applying the default preset automatically on connect
  via a `udev` rule and a `systemd` service that runs `tascamctl default`.

## [0.1.0] - 2026-06-21

Initial release: a Rust rewrite of the Tascam US-16x08 DSP mixer, as a workspace
of a control-surface library, a command-line tool, and a graphical mixer. Linux
only, via the `snd-usb-audio` driver.

### Added

- **`tascam-us16x08` library** — a typed, UI-agnostic wrapper over the ALSA
  high-level control (HCTL) interface: the full `Control` catalog, `AlsaBackend`
  (default) and an in-memory `MockBackend`, a `Watcher` for external changes,
  level and gain-reduction meter decoding, JSON presets (whole-mixer and
  per-strip), and a shared `units` module for dB / Hz / ms / pan conversions.
- **`tascamctl` command-line tool** — `list`, `topology`, `info`, `get`, `set`,
  `meters`, `monitor`, `watch`, `save`, `load`, and a shared `default` preset.
  Values read and write in display units; `set` accepts absolute, relative
  (`+N`/`-N`), and `toggle` forms.
- **`tascam-mixer` graphical mixer** (egui/eframe) — an always-visible meter
  bridge with per-channel and master mute, a focused channel/pair editor with a
  faithful biquad EQ response and a square compressor transfer graph (with a
  gain-reduction meter), EQ/compressor reset, stereo link with a level-based
  balance, an output panel, an output-routing tab, JSON presets and a shared
  default, persisted interface zoom and window size, keyboard shortcuts (channel
  navigation, `m`/`M` mute, Esc/Q to quit), and a stable Wayland `app_id`.
- **User manual** (`docs/user-manual.adoc`) covering both tools and the verified
  signal chain, renderable to PDF.

### Notes

- The tools drive the DSP mixer control surface only; they do not stream audio.
  Capture to the computer is taken pre-DSP (the dry input).

[Unreleased]: https://github.com/malo-rte/rackctl/compare/v0.7.2...HEAD
[0.7.2]: https://github.com/malo-rte/rackctl/compare/v0.7.1...v0.7.2
[0.7.1]: https://github.com/malo-rte/rackctl/compare/v0.7.0...v0.7.1
[0.7.0]: https://github.com/malo-rte/rackctl/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/malo-rte/rackctl/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/malo-rte/rackctl/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/malo-rte/rackctl/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/malo-rte/rackctl/compare/v0.2.1...v0.3.0
[0.2.1]: https://github.com/malo-rte/rackctl/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/malo-rte/rackctl/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/malo-rte/rackctl/releases/tag/v0.1.0
