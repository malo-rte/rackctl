---
name: device-editor-gui
description: Use whenever building, extending, or reviewing the egui/eframe front-end of a hardware patch/preset editor — a desktop GUI that reads, edits, and writes the patches/scenes/blocks of an external device (synth, multi-effect, mixer, …) over a slow link (MIDI SysEx, USB, serial). Covers the practical egui idioms and UX conventions this kind of app converges on: the action-collected-then-applied loop, keeping device I/O off the UI thread, the staged-edit model and live-preview-only-when-writable rule, a single connection/mode gate plus an offline mode, list/button/icon conventions (color by consequence, vendored icon fonts, scrollbar gutters, drag-and-drop drop targets), confirm-before-destroy rails, and persisting UI state. Apply this any time such a GUI is authored or judged — not only when "GUI rules" are named. It is the egui-specific companion to `architecture.adoc` (which owns the device-agnostic scene/patch/block model) and to `software-design-rules`/`rust-coding-rules`. Rules use stable IDs GUI-NN; treat it as a baseline to curate.
---

# device-editor-gui

The project's baseline for the **egui/eframe GUI of a hardware patch editor** — an
app that mirrors a device's editable state (scenes, patches, blocks), lets the user
change it, and writes it back over a slow, failure-prone link. These are the patterns
that worked; most were learned the hard way.

This skill is GUI-specific. The device-neutral **model** (scene / patch / block, the
library pattern, the device interface, the typed-vs-wire representations) lives in
`docs/architecture.adoc` — follow it; don't restate it here. Button *colours* are the
shared `ui-common::ActionKind` convention. Rust idiom is `rust-coding-rules`;
structure is `software-design-rules`.

Rules are SHOULD unless marked MUST. Stable IDs `GUI-NN`.

## App loop & state

- **GUI-1** MUST use the **action-collected-then-applied** loop: during render, push
  intent (`Action` enum values) into a `Vec`; after the frame, run one `apply(action)`
  pass that mutates state. Rendering borrows `self`/row buffers while iterating, so
  mutating mid-render fights the borrow checker — collect, then apply.
- **GUI-2** Widget callbacks contain **no logic** — they only push actions. The view
  is a pure function of state; all state change goes through `apply`.
- **GUI-3** Keep `update()` small; extract a render fn per tab/panel. (clippy
  `too_many_lines` is the backstop.)

## The device edge (I/O off the UI thread)

- **GUI-10** MUST NOT block the UI thread on device I/O. Run reads/writes on a
  background thread that locks the device **per item** and streams results back over a
  channel; the UI drains the channel each frame and requests a repaint while work is in
  flight. A whole-bank read is seconds-to-minutes — the window must stay responsive.
- **GUI-11** Read **incrementally** and show partial results (the list fills
  slot-by-slot). Cache the last-read state to disk so a relaunch shows it instantly,
  before the re-read fills it in.
- **GUI-12** Batch writes off-thread with a **per-item read-back verify** and a
  progress bar; report which items failed rather than failing the batch. A failed write
  usually means the device left its write mode — re-gate on that (GUI-30).
- **GUI-13** Convert at the edge only: the **typed model** lives in the app; serialise
  to the device wire form only when talking to the device (architecture.adoc
  *Representations*). On-disk storage is JSON of the typed model.

## Editing & staging

- **GUI-20** Stage edits: keep `pending` separate from `stored`. A row is **dirty**
  when `pending != stored`, and that same condition is the enable state of Save/Revert
  — the buttons *are* the modified indicator.
- **GUI-21** Live-preview an edit **only when the device is writable**; otherwise just
  stage it. This is what makes editing work offline and avoids errors when the device
  isn't in its write mode.
- **GUI-22** Allow **offline editing** of anything that doesn't need the device
  (library items, composed scenes) via a scratch buffer that saves back to its source,
  with no preview. A sentinel "slot" can route the shared editor at the scratch.

## Connection & mode gating

- **GUI-30** One predicate decides whether device-touching controls are enabled —
  `editable()` = connected AND in the required device mode AND no load/write in flight.
  Gate widgets with it; don't scatter ad-hoc `connected` checks.
- **GUI-31** Provide an explicit **offline launch** (`--offline`): skip the connect
  attempt and the mode gate, start on a tab that works without hardware, and offer a
  Connect action to go online later.

## Lists, buttons, and icons

- **GUI-40** MUST colour buttons **by consequence** (Commit / Read / Caution /
  Destructive / Neutral) via the shared `ActionKind` — never per-app colours. Clear and
  Delete are Destructive even when staged/undoable; the *intent* is to throw content
  away.
- **GUI-41** Keep row controls **consistent across every list**: action buttons on the
  same side (left, in this project) with identity (slot/name/level) following, and in
  **one canonical order** so a given action sits in the same relative position in every
  list. Render each list as a *subset* of that order — never reorder per list. The
  project's order is **Edit · Load · Save · Revert · Copy · Paste · Clear · Delete**
  (primary action first, destructive last).
- **GUI-42** An icon button MUST keep a **text tooltip** — an icon alone is ambiguous.
- **GUI-43** If using an icon font, **vendor it with its license**, verify every glyph
  against the font's `cmap` (no tofu) before shipping it, and pick the spacing variant
  whose glyphs aren't clipped — for Nerd Fonts use the **proportional ("Propo")**
  variant for the proportional/UI face, not the mono variant (which crams wide icons
  into one cell). Keep a real monospace face for cell-aligned text (diagrams, tables).
- **GUI-44** In dense rows prefer a **`DragValue`** over a `Slider` — a slider grows to
  fill the row and bloats the list.

## Layout & scrolling

- **GUI-50** Reserve a **scrollbar gutter** (`style.spacing.scroll.floating = false`)
  so a list's vertical scrollbar never overlaps the trailing text. egui floats
  scrollbars over content by default.
- **GUI-51** For a fixed-width **but left-aligned** cell use
  `allocate_ui_with_layout(size, Layout::left_to_right(..))`, not `add_sized` (which
  centres its content).

## Drag and drop

- **GUI-60** Make a row's **drop target an explicitly interacted rect** over the whole
  row — `ui.interact(row_rect, id, Sense::hover())` then `dnd_release_payload`, or
  `ui.dnd_drop_zone`. A bare layout `Response` misses drops released over the row's
  interactive children (buttons), so drops silently do nothing.
- **GUI-61** Use **one payload enum** for all drags within a view (e.g.
  `enum Drag { FromLibrary(String), FromBank(u16), Slot(usize) }`), matched at the drop
  site — not several distinct payload types.

## Safety rails

- **GUI-70** Confirm **destructive or overwriting** actions with a modal. A Refresh /
  re-read that would discard unsaved staged edits MUST warn first.
- **GUI-71** Never silently discard user edits — offer **Revert** to the last
  stored/loaded baseline (snapshot the baseline when state is established or saved).

## Persistence

- **GUI-80** Persist UI state to a config file: zoom, **window size**, last-active tab,
  last device port. Read the window size from `ctx.screen_rect().size() * zoom`
  (always available) — **not** the viewport `inner_rect`, which is `None` on some
  platforms (e.g. Wayland) and silently leaves the size unsaved. Restore on launch; a
  CLI flag (`--port`) overrides the saved value and updates it.
- **GUI-81** A transient screen reached *from* an item (an editor opened by a per-row
  Edit) is not a persistable destination — don't restore into it with nothing selected.

## What this skill does not cover

- The device-neutral model — scene/patch/block, library operations, device interface,
  typed/wire representations, file format & versioning → `docs/architecture.adoc`.
- Button-colour semantics → `ui-common::ActionKind` (the project-wide convention).
- Rust idiom and lints → `rust-coding-rules`; structure/layering → `software-design-rules`.
- Comment content → `code-comments`; commit hygiene → `git-commit`.
- Findings too big to fix in place → tasks via `task-from-sources`.
