# Frontend task: React COPY of the PySide6 UI (not a redesign)

## Goal

Build the React frontend (`humidi-tauri`'s existing Vite/React scaffold — currently just a `run_pedal_smoketest` test button) as a **faithful visual and behavioral copy** of the original PySide6 desktop UI at `../piano-midi-autoplayer`. Same layout, same widgets, same color scheme, same fonts/icons, same interaction behavior. This is explicitly **not** a redesign or a "reimagine this for the web" exercise — match what already exists.

The backend (Rust/Tauri) side of this port is already complete: every `PlaybackConfig` field, every compile/play/save/load/translate operation, and the real global hotkey hook are wired up as Tauri commands (see "Backend surface" below). This task is purely the frontend.

## Where the visual/behavioral source of truth lives

The Python source (`../piano-midi-autoplayer`) has an **exhaustive, actively-maintained per-file doc set under `docs/ui/**`** that mirrors `ui/**/*.py` file-for-file. These docs are more precise than any summary — **read them directly**, don't rely solely on this document, especially for exact widget geometry, animation timing, and the many documented "invariants and gotchas" that matter for pixel/behavior parity:

- `piano-midi-autoplayer/docs/ui/**/*.md` — per-widget/tab/dialog behavioral spec (33 files, one per `ui/**/*.py` file)
- `piano-midi-autoplayer/docs/ui/theme.md` — the theming engine in full
- `piano-midi-autoplayer/docs/main.md` — composition root / how everything is wired together
- `piano-midi-autoplayer/docs/controllers/*.md` — the four coordinators (`PlaybackUICoordinator`, `LoadCoordinator`, `SettingsCoordinator`, `TranslatorCoordinator`) containing the business logic each control triggers — the React equivalent of this logic is what calls the Tauri commands
- `piano-midi-autoplayer/docs/core/config.md` — the `PlaybackConfig` schema (every setting key/type/default — already ported to `humidi-tauri/src-tauri/src/core/config.rs`)
- `piano-midi-autoplayer/docs/managers/HotkeyManager.md` — hotkey capture/rebinding behavior behind Settings > Hotkey
- `piano-midi-autoplayer/assets/icons/duotone/*.svg` — the actual icon files to reuse/re-vectorize
- `piano-midi-autoplayer/assets/humidi_logo.png` — the app logo

When in doubt about exact behavior, read the actual `.py` file too — the docs are precise but the code is ground truth.

## Screen inventory

### Main window shell (`ui/MainWindowUI.py`, `docs/ui/MainWindowUI.md`)

- **Collapsible sidebar**, floating overlay on the left. Collapsed width 44px, expands to 124px on hover (120ms leave-delay, animated width). Contents top to bottom: logo row (22×22 icon + "Hu*Midi*" wordmark, Georgia 14pt) → 6 nav buttons in order **Playback(0) / Visualizer(1) / Translator(2) / Settings(3) / Debug(4) / About-License(5)** → stretch → `StatusIndicator` (red/amber/green dot or animated hourglass) → Discord link button → GitHub link button. Page content area reserves a permanent 56px left margin so nothing shifts when the sidebar expands over it.
- **Page area**: one page per nav button (a `QStackedWidget` in Python → a router/conditional-render in React), pages described below.
- **Transport bar** pinned at the bottom: scrubber row (slider + time labels) above a button row `play | stop | [stretch] | save | collapse`.
- **Collapsed strip mode** (`Ctrl+K` or the collapse button): reparents scrubber + play/stop/save/collapse + filename label + a mirrored humanize toggle into a compact title-bar-like strip; window shrinks to 270×300 minimum.
- No traditional menu bar — navigation is entirely via the sidebar.

### Playback tab (`ui/playback/PlaybackTab.py`, `docs/ui/playback/PlaybackTab.md`)

Three sub-tabs via a Roman-ordinal segmented control (**I. File / II. Playback / III. Humanize**), with a persistent file-info header strip above the sub-tab bar (icon tile, filename + meta, Replace/Reveal buttons).

**Tab I — File:**
- Full-width **LOADED** card: horizontally-scrolling strip of per-track info cards ("Track Name / ♩ note-count · pitch range · hand") + a pedal-summary card + "Edit Selection" button (opens the Track Selection dialog).
- Two columns: **REPLACE** card (dashed animated border, drag-and-drop `.mid`/`.midi` zone, Browse.../Load Save buttons) and **SAVED SONGS** card (scrollable list of save-file cards, capped at 15, refresh icon + "all saves" icon opening the Load dialog).

**Tab II — Playback:**
- Two columns: **PERFORMANCE** card (pedal-style dropdown `Auto (Default)/PedalAI/Harmonic/Rhythmic/None` → internal `hybrid/ai/legato/rhythmic/none`; transpose spinbox −24..24; "Use MIDI Pedal" toggle, hidden unless the loaded MIDI has CC64 data; "Use Velocity" toggle; reset icon) and **OPTIONS** card (88-Key Layout / Countdown / Debug Output toggles, reset icon).
- Full-width **TEMPO** card: tempo slider+spinbox (10%–1000%, shown as %), editable "resulting BPM" spinbox, original-BPM label.
- Full-width **PEDAL AI THRESHOLDS** card, two states: pre-generate shows only a "Generate AI Pedal Events" button + hint; post-generate reveals On/Off threshold spinboxes (0.000–1.000), a 4-stat readout (avg/min/max hold duration, presses/min), and diagnostic rows when output looks chattery/sparse.

**Tab III — Humanize:**
- **GENERAL SETTINGS** master card: "Humanize all" master toggle + "Simulate Hands" + "Chord Roll" toggles, each with a muted description line.
- **TIMING & FEEL** card: Vary Timing (0–0.1s), Vary Articulation (50–100%), Tempo Sway (0–0.1s) + nested "Invert Sway" checkbox.
- **HANDS & IMPERFECTION** card: Hand Drift (0–100%), Mistake Chance (0–10%).
- Row pattern repeated throughout: checkbox+description (row 1 left) + value spinbox (row 1 right) + full-width slider (row 2); checkbox enables/disables the slider+spinbox.

**Apply toast**: slide-up bottom bar (56px), appears whenever a config change makes the already-compiled playback stale; static message + Apply/Discard buttons.

Every control here maps directly to a `PlaybackConfig` field (already defined in `humidi-tauri/src-tauri/src/core/config.rs`): `tempo`, `transpose`, `use_88_key_layout`, `vary_timing`/`timing_variance`, `vary_articulation`/`articulation`, `enable_drift_correction`/`drift_decay_factor`, `enable_chord_roll`, `enable_tempo_sway`/`tempo_sway_intensity`/`invert_tempo_sway`, `enable_mistakes`/`mistake_chance`, `use_velocity_accent`, `pedal_style`, `pedal_threshold_on`/`off`, `use_midi_pedal`, `use_ai_pedal`, `countdown`, `debug_mode`, `simulate_hands`.

### Transport controls (top-level, always visible)

Scrubber slider (drag scrubs live, seeks on release → `seek_playback`), play/pause button (→ `start_playback`/`toggle_pause`), stop button (→ `stop_playback`), save button (→ `save_playback`), time labels "MM:SS / MM:SS", collapse toggle (`Ctrl+K`).

### Settings tab (`ui/settings/SettingsTab.py`, `docs/ui/settings/SettingsTab.md`)

Horizontal nav bar (no card wrapper): **Display / Files / Hotkey / System / Privacy**.
- **Display:** Always on Top toggle, Opacity slider (20–100%); Show Timeline / Show Piano toggles; theme dropdown + "Customize..." (theme editor — lower priority, see "Suggested scope" below).
- **Files:** Save Directory / MIDI Directory / Themes File — each a read-only path field + open-in-explorer icon + browse-picker icon.
- **Hotkey:** Playback Toggle row (current binding label + "Change"/"Listening..." button) and Save Playback row (same pattern) — wire to `start_binding`/`start_save_binding` and listen for `hotkey_bound_updated`/`hotkey_bound_save_updated` events.
- **System:** manual "Check for Updates" button + "Automatically check for updates" toggle (backend `check_for_updates` is still `todo!()` — stub this control), pedal-prompt-threshold spinbox (1–200, default 8), "Reset All Settings" button.
- **Privacy:** "Redact file paths" toggle (default on).

### Translator tab (`ui/translator/TranslatorTab.py`, `docs/ui/translator/TranslatorTab.md`)

Format dropdown + Import/Export segmented toggle.
- **Import page:** paste box (multi-line, Courier New 9pt) + placeholder preview column; BPM spinbox (20–400, default 120), Humanize toggle, "Play Sheet" button → calls `translate_sheet_to_notes` then feeds the result into the compile/play pipeline.
- **Export page:** placeholder track-list column + read-only output text area (Courier New 9pt); "Generate Sheet" button → `notes_to_sheet`, "Copy" button → clipboard.

### Visualizer tab (`ui/visualizer/VisualizerTab.py` + `visualizer.py`, `docs/ui/visualizer/*.md`)

Vertical stack: horizontally-scrolling **timeline/piano-roll** (note rectangles colored by hand — left=accent, right=accent_play greenish, unknown=grey; measure grid lines; bottom pedal-hold strip; draggable playhead) above an **88-key horizontal piano** (white/black keys, active pitches highlighted in `accent_play`, optional pedal indicator strip). Both are custom-painted in Python (`QPainter`) — in React these are the two components most worth building as `<canvas>` or SVG, driven by `visualizer_updated`/`pedal_updated`/`progress_updated`/`section_changed` events plus the `TimelineData`/note data from `compile_notes`.

### Debug tab (`ui/debug/DebugTab.py`, `docs/ui/debug/DebugTab.md`)

Console card (filterable/leveled log, monospace, capped 5000 lines) + right column: Filter card (level dropdown + Auto-scroll toggle), Levels card (per-level tallies), Session Snapshot card (file/source/tracks/notes/duration/pedal/tempo/pedal_style key-value rows). Footer: Clear / Copy Log / Export Log. Lower priority — see "Suggested scope."

### License tab (`ui/license/LicenseTab.py`, `docs/ui/license/LicenseTab.md`)

Simple: left nav list (HuMidi / PedalAI Dataset / Third-Party Libs / Phosphor Icons) + read-only text panes. Low effort, low priority.

### Track Selection dialog (`ui/dialogs/TrackSelectionDialog.py`, `docs/ui/dialogs/TrackSelectionDialog.md`)

Modal, 720×400. Info label + 5-column table: Play checkbox / Track Name / Instrument / Notes / Hand-Assignment dropdown (Auto-Detect / Left Hand / Right Hand). Drum tracks (MIDI channel 9) default unchecked. OK/Cancel. Feeds `selected_tracks_info: [(index, role)]` into `compile_notes`. Backed by `parse_midi_structure`'s `TrackSummary[]` result.

### Save/Load UI

- Saved-songs list lives inline in Playback Tab I (see above).
- Load-all dialog (`ui/dialogs/LoadSaveDialog.py`, `docs/ui/dialogs/LoadSaveDialog.md`): modal, 820×520, split view — left: tree grouped by source MIDI filename (bold non-selectable parent rows, child rows = individual saves, newest-first); right: metadata detail pane (source MIDI, creation date, playback-settings grid, humanization grid). Bottom: Rename / Delete / Cancel / Load → `load_save_file`.

### Theme dialog (`ui/dialogs/ThemeDialog.py`, `docs/ui/dialogs/ThemeDialog.md`)

The most complex dialog — a live-previewed color-token editor with an "inspect mode" that lets you click any preview element to jump to its color field. **Treat as an explicit stretch goal, not part of the first pass** (see "Suggested scope").

## Theme system — build this as the foundation, not an afterthought

The Python app has **one theme engine driving everything** (`ui/theme.py`, `docs/ui/theme.md`): a `ThemeColors` dataclass with **16 semantic fields**, not scattered hardcoded colors. Build the React equivalent as a CSS custom-property / design-token system with these exact same 16 named tokens, so a theme switcher (and later, the theme editor) can work the same way.

Default active theme is **"Midnight"**. Three built-in presets, exact hex values:

| Token | Meaning | Midnight | Light | Hatsune Miku |
|---|---|---|---|---|
| `bg_primary` | window/dialog bg | `#0d1117` | `#f0f0f8` | `#111111` |
| `bg_secondary` | surfaces/cards/headers | `#161b22` | `#ffffff` | `#2c2c2c` |
| `bg_input` | inputs | `#1c2230` | `#fafafa` | `#1a1611` |
| `accent` | interactive/selection | `#58a6ff` | `#4a7adb` | `#00ffff` |
| `text_primary` | main text | `#e6edf3` | `#1a1a2e` | `#00bbcc` |
| `text_secondary` | muted labels | `#8b949e` | `#6868a0` | `#2a7fa3` |
| `border` | all borders | `#30363d` | `#d0d0e8` | `#2e5963` |
| `accent_play` | play button / active note (right hand) | `#3fb950` | `#2a9a60` | `#4affff` |
| `accent_stop` | stop/danger | `#f85149` | `#cc3333` | `#ff0000` |
| `pedal_color` | sustain pedal indicator | `#f0a030` | `#d08010` | `#51a4cb` |
| `accent_controls` | slider handle/checkbox fill | `#58a6ff` | `#4a7adb` | `#00ffff` |
| `bg_button` | generic button rest bg | `#161b22` | `#ffffff` | `#2c2c2c` |
| `accent_save` | save button tint/icon | `#58a6ff` | `#4a7adb` | `#ffec1c` |
| `accent_loaded` | "file loaded" warning dot | `#d4a020` | `#b87010` | `#d4bd0d` |
| `knob_color` | toggle-switch knob | `#dadbdc` | `#fcfcfd` | `#dbdbdb` |
| `toggle_on` | toggle track when checked | `#58a6ff` | `#4a7adb` | `#00ffff` |
| `toggle_off` | toggle track when unchecked | `#8b949e` | `#6868a0` | `#2a7fa3` |

Derived shades computed from the above (see `generate_stylesheet` in `docs/ui/theme.md`) — replicate via CSS `color-mix()` or precomputed values: `btn_hover` (16% accent blend), `accent_tint` (12% accent blend), `save_card_hover` (10% accent blend), `dropzone_border` (35% black blend).

**Fonts:** base UI `"Segoe UI", sans-serif, 9pt`; logo wordmark `Georgia, 14pt`; monospace areas (Debug console, Translator text) `"Courier New", 9pt`; page header titles `Georgia italic, 12pt`; drop-zone hint text `Georgia italic, 13pt`.

**Icons:** Phosphor **Duotone** SVGs, `fill="currentColor"` so one hex swap recolors the whole icon (including the duotone's opacity-0.2 background layer). Source files at `piano-midi-autoplayer/assets/icons/duotone/<name>-duotone.svg`. Stems used: `arrow-counter-clockwise`, `arrows-clockwise`, `bug`, `certificate`, `clock`, `delete-theme`, `discord-logo`, `export-theme`, `floppy-disk`, `folder-closed`, `folder-open`, `gear-six`, `github-logo`, `import-theme`, `inspect-mode`, `list-magnifying-glass`, `music-note`, `new-theme`, `palette`, `pause`, `play`, `rename-theme`, `resize-collapse`, `resize-expand`, `stop`, `toggle-left`, `toggle-right`, `translate`, `waveform`. Reuse these SVG files directly (they're already `currentColor`-based, ideal for React/CSS). Logo: `piano-midi-autoplayer/assets/humidi_logo.png`.

## Custom widgets with no native HTML/CSS equivalent

Build these as real components (not styled `<input>`s where the visual differs meaningfully):

- **Toggle switch** — replaces every checkbox app-wide. Rounded-rect track (28×16px) + sliding circular knob (12px), 180ms ease slide animation.
- **Status indicator** — sidebar dot: red (unloaded) / amber (loaded) / green (ready) / animated 4-frame hourglass at 350ms interval (loading), with a text label that streams state ("PREPPING"/"ANALYZING"/"GEN. PEDAL" etc. — drive this from `status_updated` events).
- **Timeline / piano-roll** — `<canvas>` or SVG, note rectangles by hand color, measure lines, pedal strip, draggable playhead.
- **Piano widget** — 88-key horizontal keyboard, active-pitch highlighting from `visualizer_updated`, optional pedal strip.
- **Dashed drop-zone border** — marching-ants animated dashed border (CSS `stroke-dashoffset` animation on an SVG rect, or `background-position` trick), solid+tinted on drag-over.
- **Apply toast** — slide-up-from-bottom notification bar with a shake animation.
- **Save-card / part-card** — pressable vs. static info cards with hover/press states.
- **Sub-tab bar** — Roman-numeral segmented control (I/II/III).

## Backend surface already available (Tauri commands + events)

All in `humidi-tauri/src-tauri/src/commands.rs` — see `humidi-tauri/docs/commands.md` for full details/signatures.

**Commands (invoke from React):**
- `run_pedal_smoketest` — existing scaffolding, can be removed once real commands are wired up
- `parse_midi_structure(filepath) -> TrackSummary[]` — feeds the Track Selection dialog
- `compile_notes(config, selected_tracks_info) -> TimelineData { final_notes, total_dur }`
- `compile_pedal(config) -> PedalData { pedal_intervals, ai_thresholds }`
- `compile_pedal_and_play(config) -> PedalData`
- `start_playback(config)`, `toggle_pause()`, `stop_playback()`, `seek_playback(target_time)`
- `save_playback(config, selected_tracks_info, original_filename) -> String` (output path)
- `load_save_file(filepath) -> Value` (parsed save JSON)
- `translate_sheet_to_notes(sheet_text, bpm, use_88_key_layout) -> Note[]`
- `notes_to_sheet(notes, use_88_key_layout, tempo_events, time_signatures) -> String`
- `start_binding()`, `start_save_binding()` — hotkey rebind capture

**Events (listen for, via Tauri's `listen()`):**
- `status_updated` (string), `progress_updated` (f64 seconds), `playback_finished` (null), `visualizer_updated` (active pitch array), `pedal_updated` (bool), `auto_paused` (null), `error_occurred` (string), `section_changed` (index), `playback_started` (null)
- `hotkey_bound_updated` / `hotkey_bound_save_updated` (string, the formatted combo) — update the Settings > Hotkey labels
- `hotkey_toggle_requested` / `hotkey_save_requested` (null) — fired by the real global hook; the frontend should react by calling `toggle_pause`/`save_playback` itself, matching the original's decoupled Signal-based design

**Not yet wired to `AppState`:** `ConfigManager` (so there's no backend-persisted `save_dir`/`midi_dir`/app-config yet — `save_playback` currently needs `AppState.save_dir` set through some other path; this is a real gap worth raising with the user or closing as part of this frontend work if a settings-persistence command is needed). `check_for_updates` is still `todo!()`.

## Suggested scope for a first pass

Given the size of the full original app, consider sequencing:
1. Theme/token system + app shell (sidebar, page routing, transport bar) — the foundation everything else sits on.
2. Playback tab (all three sub-tabs) + Track Selection dialog + Save/Load — the core "load a MIDI, configure it, play it" loop.
3. Visualizer tab (timeline + piano) wired to the playback events.
4. Translator tab.
5. Settings tab (Display/Files/Hotkey/System/Privacy).
6. Debug tab, License tab, Theme editor dialog — lowest priority, most effort-to-value skew; the Theme dialog in particular (live half-scale preview + inspect mode) is the single most complex piece of UI in the whole app and should be scoped as its own follow-up rather than attempted inline.

## Keyboard shortcuts to replicate

- `Ctrl+K` — toggle collapsed/expanded window mode (in-app, standard web `keydown` listener is fine for this one).
- **Playback Toggle** (default **F6**, no modifiers — not Ctrl+F6, verify against `HotkeyManager::new()`'s actual default in `humidi-tauri/src-tauri/src/managers/hotkey_manager.rs`) and **Save Playback** (default **Ctrl+S**) are OS-level global hotkeys, already implemented on the Rust side via `rdev` (works even when the window isn't focused) — the frontend does not need to (and should not) implement these as browser `keydown` listeners; it only needs to react to the `hotkey_toggle_requested`/`hotkey_save_requested` events and reflect current bindings via `hotkey_bound_updated`/`hotkey_bound_save_updated`.
