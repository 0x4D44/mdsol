# Solitaire (Win32/Rust)

Native Windows Solitaire (Klondike) implemented with Rust and Win32 APIs via the `windows` crate. Ships as a single ~3.5MB EXE with embedded resources, card sprites, menus, and dialogs.

## Features

- **Two Scoring Modes**: Standard (with optional timed penalty) and Vegas (with cumulative bank)
- **Two Victory Animations**: Classic Windows XP-style cascade and Modern physics-based bounce
- **Advanced Solver**: K+ compressed DFS algorithm determines deal solvability in ~120ms
- **Professional Graphics**: WIC-based PNG decoding, double-buffered GDI rendering with alpha blending
- **Full Undo/Redo**: Complete game state snapshots for unlimited undo
- **Settings Persistence**: Window position, game options, and Vegas bank saved to Windows Registry
- **Intelligent Hints**: Priority-based move suggestions (Ctrl+H)
- **Deterministic Replay**: BCrypt-based RNG with seed storage for "Deal Again" feature

## Documentation

📘 **[Comprehensive Architecture Documentation](2025.11.06%20-%20Comprehensive%20Architecture%20Documentation.md)** - Complete system design, data flow diagrams, module structure, and implementation details (1,636 lines, 64KB)

## Build

Prerequisites (Windows 10/11 x64):

- Rust toolchain (stable MSVC): `rustup toolchain install stable`
- Visual Studio Build Tools with Windows 10/11 SDK (for `rc.exe` and link libs)
- `cargo` on PATH

Build and run:

```
cargo build --release
target\\release\\mdsol.exe
```

Notes:

- Double-buffered GDI rendering is enabled. If no card PNG is embedded, a placeholder card is drawn.
- Manifest, menu, accelerators, and version info are embedded. Icon and card assets are optional and can be added later.

## Cards: Download + Pack

The repository already includes a pre-generated card sprite at `res/cards.png`, so builds embed the checked-in art. Run the optional `xtask` helper below only if you need to regenerate the sprites (for example, to use different artwork or dimensions).

Optional regeneration (requires internet):

```
cargo run -p xtask -- gen-cards --card-w 224 --card-h 312
```

This will:

- Download the deck from the `notpeter/Vector-Playing-Cards` mirror
- Rasterize SVGs to the requested size
- Generate `res/cards.png` (13 columns × 4 rows)
- Update `res/app.rc` to embed the PNG as `IDB_CARDS`

After that, rebuild:

```
cargo build --release
```

If you already have the SVGs locally:

```
cargo run -p xtask -- gen-cards --source path\to\cards-svg
```

The tool also writes a JSON map alongside the PNG for debugging (not used at runtime).

## Assets

- Cards: Place a CC0/PD card sprite sheet PNG at `res/cards.png` (e.g., Kenney playing cards). Then open `res/app.rc` and uncomment the line:

```
// IDB_CARDS RCDATA "res/cards.png"
```

to become

```
IDB_CARDS RCDATA "res/cards.png"
```

Rebuild to embed the PNG. The app will decode it via WIC and render a test card.

- Icon: Place a multi-size icon at `res/app.ico` and add an `ICON` entry in `res/app.rc` if desired.

### Licensing

- Byron Knoll’s “Vector Playing Cards” are Public Domain. The `notpeter/Vector-Playing-Cards` repo indicates “Public Domain/WTFPL” for images/outputs. See upstream for details.

## Licenses & Attributions

- Code: MIT License (see repository).
- Card art: Public Domain
  - Byron Knoll’s Vector Playing Cards (mirror: notpeter/Vector-Playing-Cards)
  - Kenney Playing Cards Pack (kenneyNL/playing-cards-pack)
- Tools used in asset generation (`xtask`): resvg, usvg, tiny-skia (see crates.io pages for licenses).

Attribution is shown in the About dialog. If you ship different assets, ensure you comply with their licenses and update attributions accordingly.

## Options

Open File → Options… to configure:

- Draw Mode: Draw 1 or Draw 3
- Victory Animation: Classic or Modern
- Scoring: Standard or Vegas
- Timed scoring: counts down -2 points every 10 seconds (Standard only)
- Vegas cumulative: keeps Vegas balance across hands
- Highlight hint target: draws a brief gold outline on the suggested destination

Notes:

- Timed scoring starts when you make your first move and pauses when the window is minimized; it resumes when restored.
- Vegas scores show as currency (e.g., -$52); Standard scores are whole numbers (with thousands separators).

## Keyboard Shortcuts

- New: F2
- Deal Again: Ctrl+N
- Undo: Ctrl+Z
- Redo: Ctrl+Y
- Hint: Ctrl+H
- Pause Timer: Ctrl+P / Esc (when no victory animation)
- Pause Timer: Ctrl+P

## Roadmap

- M1: Skeleton app/resources (done)
- M2: Rendering path (WIC decode + back buffer)
- M3: Engine + interaction
- M4: Scoring/options/persistence
- M5: Polish/QA

## Installer

We ship a simple [Inno Setup](https://jrsoftware.org/isinfo.php) script in `installers/` so you can produce a Windows installer like the stock Microsoft Solitaire experience.

1. Build the release binary:
   ```powershell
   cargo build --release
   ```

Note: The installer filename uses the crate version from `Cargo.toml` (e.g., `Solitaire-1.0.7-Setup.exe`) and lands in `installers/`.
2. Run Inno Setup's command line compiler (`iscc`) from the repository root:
   ```powershell
   iscc installers\Solitaire.iss
   ```

The compiled installer (`Solitaire-1.0.7-Setup.exe`) lands in `installers\`. Adjust the `MyAppVersion` constant at the top of `Solitaire.iss` when you bump the crate version.
## Test

- On Windows:
  - `cargo test` (runs engine/solver tests and builds the app)
  - `cargo clippy` (lint checks)
  - `cargo fmt --check` (format checks)
- In CI:
  - Windows tests run in `.github/workflows/windows-tests.yml`
  - Lint checks in `.github/workflows/lint.yml`
  - Linux checks for `xtask` run in `.github/workflows/xtask-check.yml`

## Architecture Overview

### Technology Stack

- **Language**: Rust 2021 Edition (MSVC toolchain)
- **Graphics**: GDI (double-buffered), WIC (PNG decoding), AlphaBlend
- **RNG**: BCrypt (cryptographically secure shuffling)
- **UI Framework**: Native Win32 via `windows-rs` v0.52
- **Build System**: Cargo + embed-resource (rc.exe integration)

### Module Structure

```
src/
├── main.rs       (~4,300 lines) - UI, rendering, Win32 integration
├── engine.rs     (~1,000 lines) - Game logic, move validation, scoring
├── solver.rs     (~650 lines)   - K+ compressed DFS solvability checker
└── constants.rs  (~56 lines)    - Shared resource IDs and constants
```

### Key Components

**Game Engine** (engine.rs):
- 52-card deck with BCrypt-based Fisher-Yates shuffle
- Move validation for foundations (same suit, ascending) and tableaus (alternating colors, descending)
- Two scoring modes with different rules and penalties
- Win condition detection and solvability integration

**Solver Algorithm** (solver.rs):
- K+ compressed stock/waste representation (~10x speedup)
- Iterative-deepening DFS with transposition table (~100x speedup)
- Auto-normalization (safe foundation moves)
- 120ms time budget per solve attempt
- Returns: Winnable, Unwinnable, or Timeout

**Rendering Pipeline** (main.rs):
- WIC decodes embedded PNG to 32bpp PBGRA (pre-multiplied alpha)
- Double-buffered off-screen composition
- Sprite sheet: 13×4 grid (2912×1248 px, 224×312 per card)
- AlphaBlend for transparency, single BitBlt per frame

**UI Interaction** (main.rs):
- Hit testing with cached tableau slots for performance
- Drag-and-drop with 4px threshold
- Double-click auto-move to foundations
- Hint system with 6-level priority (reveals hidden cards first)

### Performance Characteristics

- **Rendering**: 1-2ms per frame on modern hardware, 60 FPS achievable
- **Solver**: <10ms (simple), 20-50ms (medium), 100-120ms (hard deals)
- **Startup**: 100-200ms cold start (includes WIC PNG decode)
- **Memory**: 15-25MB total (14MB sprite sheet, 8MB backbuffer)

For detailed architecture documentation including data flow diagrams, subsystem analysis, and development guidelines, see [2025.11.06 - Comprehensive Architecture Documentation.md](2025.11.06%20-%20Comprehensive%20Architecture%20Documentation.md).

## Contributing

Before making changes:
1. Read [CLAUDE.md](CLAUDE.md) for AI assistant guidance
2. Review [Architecture Documentation](2025.11.06%20-%20Comprehensive%20Architecture%20Documentation.md) for system design
3. Ensure resource IDs in `constants.rs` match `res/app.rc` exactly
4. Run `cargo test`, `cargo clippy`, and `cargo fmt` before committing

## License

MIT License - see repository for details.

Card artwork is Public Domain (Byron Knoll's Vector Playing Cards).
