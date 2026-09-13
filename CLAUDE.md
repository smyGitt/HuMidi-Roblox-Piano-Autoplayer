# humidi-tauri Codebase Operating Protocol

> This is the Rust/Tauri rewrite of HuMidi (originally PySide6, at `../piano-midi-autoplayer`). This file is loaded automatically into every Claude Code session that opens this repo. The rules below are binding, and are adapted from `piano-midi-autoplayer/CLAUDE.md`'s protocol for this repo.

## 1. No source comments (binding)

Never add comments to source code in this repo, in any language (Rust, TypeScript, etc.), including doc comments (`///`, `//!`) and inline comments (`//`). The per-file doc under `docs/<same-relative-path-as-source>.md` (e.g. `src-tauri/src/core/models.rs` -> `docs/core/models.md`) is the single place where explanations, invariants, and gotchas belong. This applies to new code and to code you touch for another reason: strip existing comments from any block you edit and fold anything they said that isn't already in the per-file doc into that doc instead.

## 2. Doc location mirrors source location (binding)

Each per-file doc lives at the same relative path under `docs/` as its source under `src-tauri/src/`, with the `src-tauri/src/` prefix dropped: `core/models.rs` -> `docs/core/models.md`, `managers/hotkey_manager.rs` -> `docs/managers/hotkey_manager.md`, `commands.rs` -> `docs/commands.md`. If you move, rename, or delete a source file, move/rename/delete the matching doc in the same change, and update the Quick Index in `docs/CODEBASE_REFERENCE.md`.

## 3. Lookup-first navigation (binding)

Before opening any source file, read `docs/CODEBASE_REFERENCE.md` (Quick Index), then the per-file doc for that file. The per-file doc contains invariants not visible from source alone (e.g. `KeyEvent`'s ordering semantics, the BiLSTM gate-order gotcha, per-platform keyboard fixes) -- do not skip this step even for files that look simple.

## 4. Update-on-change (binding)

Any code change MUST be paired with an edit to the matching per-file doc before the task is reported as complete. If you add, remove, or rename a source file, also update the Quick Index in `docs/CODEBASE_REFERENCE.md`.

## 5. Regression test parity (binding)

A Python module from `piano-midi-autoplayer` is not considered ported until its mirrored Rust test module exists and passes, ported case-by-case from the corresponding `tests/test_*.py` file. See `PORTING_PLAN.md`'s test-inventory table. Write mirrored tests in the same step as the logic they cover -- do not defer to "a testing pass at the end."

## 6. Never commit on your own initiative (binding)

Never run `git commit` (or `git push`) in this repo unless the user has explicitly said, in that same turn, to commit (or push). Staging changes and running `git status`/`git diff` are fine; creating the commit is not, until the user says so in words.

## 7. No Windows registry access, ever, for any reason (binding, absolute, nonnegotiable)

Never read from or write to the Windows registry -- no `reg.exe`, no `Get-ItemProperty`/`Set-ItemProperty`/`New-ItemProperty` against `HKLM:`/`HKCU:`/any registry PSDrive, no `[Microsoft.Win32.Registry]`/`RegistryKey` .NET calls, no `[System.Environment]::GetEnvironmentVariable(..., "Machine"/"User")` or `SetEnvironmentVariable` with a persistence target, and no other mechanism that reads or writes registry-backed state, including reading it just to patch the current shell's `$env:PATH`. This applies even when it would fix something real (a stale PATH after installing a tool, a broken file association, etc.) and even when it seems read-only/harmless. If a tool's effects aren't visible in the current shell, say so and tell the user to open a new terminal themselves -- do not work around it via the registry. There are no exceptions to this rule; do not ask for one, do not reach for it as a last resort, and do not treat "I only read it" as a loophole.

## Architecture pointers

- Original Python source of truth: `../piano-midi-autoplayer` (stays intact/buildable throughout the port).
- Entry point: `src-tauri/src/lib.rs`'s `run()`.
- Porting order and the full test-parity mapping: `PORTING_PLAN.md`.
- Platform target: Windows, macOS, Linux X11, Linux Wayland (Tauri's default-supported set).

For source file locations, navigate via `docs/CODEBASE_REFERENCE.md`.
