# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Native Windows Solitaire implemented with Rust and Win32 APIs via the `windows` crate. Ships as a single EXE with embedded resources (menu, icons, card sprites, manifest).

**Key dependencies:**
- `windows` crate (v0.52) for Win32 API bindings
- BCrypt for cryptographically-secure RNG (deck shuffling)
- WIC (Windows Imaging Component) for PNG decoding
- GDI for double-buffered rendering

## Build Commands

**Prerequisites:** Windows 10/11 x64, Rust MSVC toolchain, Visual Studio Build Tools with Windows SDK (for `rc.exe`)

```powershell
# Build release binary
cargo build --release

# Run the application
target\release\mdsol.exe

# Run tests (engine and solver tests)
cargo test

# Run specific test
cargo test <test_name>

# Run tests with output
cargo test -- --nocapture

# Lint with Clippy
cargo clippy

# Format code
cargo fmt
```

## Development Workflow

**Resource files:** The Win32 resources are defined in `res/app.rc` and compiled during build via `build.rs` using the `embed-resource` crate. Changes to `.rc`, `.manifest`, `.ico`, or `cards.png` trigger a rebuild.

**Card sprite generation (optional):**
```powershell
# Regenerate res/cards.png from downloaded SVG cards
cargo run -p xtask -- gen-cards --card-w 224 --card-h 312

# Use existing SVG directory
cargo run -p xtask -- gen-cards --source path\to\cards-svg
```

**Installer:** Uses Inno Setup to create `Solitaire-1.0.7-Setup.exe`:
```powershell
cargo build --release
iscc installers\Solitaire.iss
```

## Architecture

### Module Structure

- **`main.rs`** (~4000 lines): Win32 window proc, GDI rendering, UI interactions, drag-and-drop, menu handling, animation, registry persistence, status bar, dialogs (About/Options/Help)
- **`engine.rs`**: Core game logic—`GameState`, `Card`, `Pile`, deck creation, shuffling (BCrypt), dealing, move validation, scoring (Standard/Vegas), undo/redo stacks, and solvability checks
- **`solver.rs`**: Iterative-deepening DFS solver for determining if a deal is winnable (used by hint system and "deal solvable" feature). Uses K+ compressed stock/waste representation
- **`constants.rs`**: Shared resource IDs (menu commands, dialog controls), company/product metadata
- **`build.rs`**: Invokes `embed-resource` to compile `res/app.rc` into the binary

### Key Data Structures

**`GameState`** (engine.rs:136):
- `stock`, `waste`, `foundations[4]`, `tableaus[7]`: card piles
- `draw_mode`: DrawOne or DrawThree
- `score`, `moves`, `rng_seed`
- `scoring_mode`: Standard or Vegas
- Undo/redo stacks for moves

**`WindowState`** (main.rs:506):
- Owns `GameState`, `BackBuffer`, `CardImage`, drag/animation state
- Tracks UI state: selected slots, hint targets, victory animations, timer
- Registry persistence: window bounds, options, Vegas bank balance

**`Solver`** (solver.rs):
- Represents cards as `u8` in [0, 51] (suit * 13 + rank)
- `State`: tableau piles (with `up_from` index), foundations (heights), K+ stock/waste
- Iterative deepening with transposition table and time budget (120ms default)
- Returns `SolveResult::{Winnable, Unwinnable, Timeout}`

### Rendering Pipeline (main.rs)

1. **WIC Decode** (startup): Load embedded PNG from resources (`IDB_CARDS`), convert to 32bpp PBGRA
2. **BackBuffer** (main.rs:1305): Off-screen DIB section for double-buffering
3. **WM_PAINT**: Render green felt background, card slots, cards (with AlphaBlend for transparency), animations, drag preview
4. **Victory Animations**: Classic (card cascade with physics) or Modern (card flip-bounce)

### Win32 Integration

- **Resources** (`res/app.rc`): Menu, accelerators, dialogs, version info, manifest, card PNG
- **Registry** (`HKEY_CURRENT_USER\Software\0x4D44 Software\Solitaire`): Persist window bounds, draw mode, scoring options, Vegas bank
- **Status Bar**: Shows score/time via `CreateStatusWindowW`
- **Dialogs**: About, Options, Keyboard Shortcuts (defined in `.rc`, handled via `DialogBoxParamW`)
- **Accelerators**: F2 (New), Ctrl+N (Deal Again), Ctrl+Z/Y (Undo/Redo), Ctrl+H (Hint), Ctrl+P (Pause)

## Testing

- **Unit tests** are in `engine.rs` and `solver.rs` (e.g., `#[cfg(test)] mod tests`)
- **CI workflows** (`.github/workflows/`):
  - `windows-tests.yml`: Run `cargo test` on Windows
  - `xtask-check.yml`: Verify `xtask` builds on Linux (cross-platform check)
  - `lint.yml`: Run `cargo clippy` and `cargo fmt --check`
  - `windows-build.yml`: Build release binary
  - `release.yml`: Build installer on tag push

## Scoring Modes

**Standard:**
- Waste to Tableau: +5
- Waste to Foundation: +10
- Tableau to Foundation: +10
- Foundation to Tableau: -15
- Recycle waste: -100 (except first pass in Draw Three)
- Timed: -2 points every 10 seconds (pauses when minimized)

**Vegas:**
- Cost: -$52 per hand
- Foundation: +$5 per card
- Cumulative mode: bank persists across hands (reset via Game → Reset Vegas Bank)

## Code Style

- **Rust edition 2021**, MSVC target
- **Release profile:** LTO enabled, `opt-level=3`, `codegen-units=1`, stripped
- **No `unsafe`** blocks in main code (rely on `windows` crate safe abstractions)
- **Naming:** `snake_case` for functions/vars, `PascalCase` for types, `SCREAMING_SNAKE_CASE` for consts
- **Win32 constants:** Defined in `constants.rs` to match `.rc` file (e.g., `IDM_FILE_NEW = 40001`)
- **Resource IDs:** Must stay in sync between `constants.rs` and `res/app.rc`

## Common Pitfalls

- **Resource ID mismatches:** If menu commands don't fire, check that `constants.rs` and `res/app.rc` agree
- **Card sprite layout:** 13 columns (Ace→King) × 4 rows (Spades, Hearts, Diamonds, Clubs); `sprite_index = suit.row() * 13 + rank.column()`
- **Registry keys:** Use the helpers in `main.rs` (`load_dword_value`, `save_dword_value`, etc.) to avoid raw Registry API errors
- **Undo/redo stacks:** Clear redo stack on any non-undo move; capture full state snapshots
- **Solver timeout:** 120ms budget per call (see `SOLVER_TIME_BUDGET_MS`); return `Timeout` if exceeded to avoid hanging UI

## Important Constants

- **Window defaults:** 640×480 minimum, centered on primary monitor
- **Timer IDs:** `GAME_TIMER_ID = 2` (timed scoring), `STATUS_TIMER_ID = 3` (status bar updates)
- **Solver budget:** 120ms (`SOLVER_TIME_BUDGET_MS`)
- **Card dimensions:** Default 224×312 pixels (configurable via `xtask gen-cards`)

## Versioning

Version is defined in `Cargo.toml` (e.g., `version = "1.0.7"`). When bumping version, also update `MyAppVersion` in `installers/Solitaire.iss` to match.
