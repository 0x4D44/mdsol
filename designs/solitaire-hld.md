# Solitaire (Win32/Rust) — High‑Level Design

This document proposes a native Windows Solitaire implemented in Rust using the `windows` crate (windows-rs) and classic Win32 APIs. The app ships as a single 64‑bit Windows executable with embedded resources and an authentic Win32 look and feel.

## Overview

- Native Win32 application written in Rust.
- Single EXE (no DLLs or frameworks beyond OS libraries).
- Classic Windows UI: menus, accelerators, status bar, icon, version info, manifest.
- Rendering via GDI with double buffering; PNG decoding via WIC; RNG via BCrypt.
- Per‑Monitor v2 DPI aware; flicker‑free painting; authentic green felt background.
- Solitaire rules with Draw 1/3, Standard/Vegas scoring, timer, undo/redo, auto‑move, auto‑complete.

## Goals and Non‑Goals

### Goals
- Authentic Windows UX with comctl32 visual styles (v6) and system fonts.
- Smooth, flicker‑free rendering on Win10/11 x64 using only OS libraries.
- Robust game engine with deterministic undo/redo and scoring variants.
- Self‑contained assets embedded in the executable.
- Clean modular architecture with clear platform separation.

### Non‑Goals
- Cross‑platform support (Windows‑only).
- GPU/DirectX rendering (we will use GDI + WIC only).
- Localization beyond English in v1.

## Target Environment

- OS: Windows 10/11 x64.
- Toolchain: Rust (stable), target `x86_64-pc-windows-msvc`.
- Runtime deps: system DLLs only (User32, Gdi32, Comctl32, Msimg32, Windowscodecs, Ole32, Bcrypt, Advapi32, Shell32).

## Tech Stack

- Language: Rust (edition 2021 or newer).
- Crates:
  - `windows` (windows-rs) with features for: Win32 UI, GDI, COM, WIC, BCrypt, Shell, Registry.
  - `embed-resource` for compiling `.rc` → `.res` and linking.
  - `anyhow` (errors) and `thiserror` (optional) for ergonomic error handling.
  - `bitflags` for UI/engine flags.
  - `once_cell` for singletons (e.g., WIC factory) if needed.
- Link libraries: User32, Gdi32, Comctl32, Msimg32, Windowscodecs, Ole32, Bcrypt, Advapi32.
- Subsystem: `#![windows_subsystem = "windows"]` in `main.rs`.

## Assets and Licensing

- Cards: CC0/Unlicense card sprite sheet (e.g., Kenney). Embed as PNG (RCDATA) within EXE.
- Icon: CC0/PD card‑themed icon with multiple sizes (16–256 px) embedded as `RT_ICON/GROUP_ICON`.
- Background: solid green felt via GDI; optional subtle noise later if desired (still embedded).
- Licenses: include asset license texts in `res/licenses/` and reference in About dialog.

## Functional Requirements

- Game: Classic Solitaire.
  - Layout: 7 tableau piles, 4 foundation piles, stock, waste.
  - Deal: standard 52‑card deck, shuffled with `BCryptGenRandom` seed.
  - Draw modes: Draw 1 or Draw 3 (toggleable).
  - Moves: drag/drop, double‑click/Right‑click auto‑move to foundation, single click to flip stock.
  - Auto‑complete: when no hidden cards remain in tableau and all moves are monotonically increasing to foundations.
- Scoring and timing:
  - Standard scoring (Windows classic) and Vegas scoring; optional timed scoring; Vegas cumulative option.
  - Timer start on first move; pause on minimize or when game over.
  - Status bar: score, time, moves, draw mode, Vegas flag.
- Undo/Redo: unlimited (bounded by reasonable memory), Ctrl+Z / Ctrl+Y.
- Menus & accelerators:
  - New (F2), Deal Again (Ctrl+N), Options, Undo/Redo, Hint (optional), Exit (Alt+F4).
  - Help → About with version, license attributions.
- Persistence: user options stored under `HKCU\Software\<Vendor>\Solitaire`.

## Non‑Functional Requirements

- Single EXE deliverable; no external assets.
- Flicker‑free rendering via off‑screen DIB and single blit per paint.
- DPI awareness: Per‑Monitor v2, dynamic layout scaling, crisp assets (nearest/alpha as appropriate).
- Startup time under 250 ms on typical hardware.
- Memory footprint under ~50 MB at steady state.

## Architecture

### Module Overview

- `app_main` — program entry, COM init, message loop, window class registration.
- `platform_win` — Win32 window proc, message dispatch, DPI handling, timers, cursors.
- `resources` — load icon/menu/accelerators; WIC loader for embedded PNG → 32bpp BGRA DIB.
- `render_gdi` — back buffer management (DIB section), sprite blits (AlphaBlend/BitBlt), layout/measure.
- `engine` — game state, rules, legal move checking, scoring, timer, auto‑complete, undo/redo stacks.
- `rng` — wrappers over `BCryptGenRandom` for shuffles and seeds.
- `ui` — commands, menu/accelerator routing, dialogs (About/Options), status bar control.
- `settings` — options persistence in registry and in‑memory cache.

### Data Model

- `Suit` (Clubs, Diamonds, Hearts, Spades)
- `Rank` (Ace..King)
- `Color` (Red/Black)
- `Card { suit: Suit, rank: Rank, face_up: bool, id: u8 }` — `id` fixed mapping 0..51 for sprite lookup.
- `PileKind` (Stock, Waste, Tableau(0..6), Foundation(0..3))
- `Pile { kind: PileKind, cards: Vec<Card> }`
- `Move` enum for atomic user/engine actions (flip, move stack, deal, recycle stock, auto‑complete step).
- `GameState { piles, score, moves, time_ms, options, rng_seed, undo: Vec<Move>, redo: Vec<Move> }`
- `Options { draw_three: bool, scoring: ScoringMode, timed: bool, vegas_cumulative: bool }`

### Core Flows

- Startup: init COM (MTA or STA as needed for WIC), load resources (icons/menus), decode cards PNG once, create back buffer sized to client rect, start idle timer.
- Deal/New Game: reset engine state, shuffle deck via RNG, populate piles according to rules, reset score/time.
- Input: mouse down → hit test → begin drag with `SetCapture`; mouse move → update drag; mouse up → drop if legal else snap back; double‑click/right‑click → auto‑move; keyboard via accelerators.
- Paint: respond to `WM_PAINT` by drawing background, piles, dragged stack atop all, status overlays; avoid `WM_ERASEBKGND` flicker.
- Resize/DPI: on `WM_SIZE` or `WM_DPICHANGED`, rebuild back buffer to new size/scale; recompute layout metrics.
- Quit: persist options to registry; release resources.

## Rendering Design

- Back buffer: 32‑bit DIB section (BGRA) created once per size; render all scene into memory DC; single `BitBlt` to screen in `WM_PAINT`.
- Cards: decoded from PNG sprite sheet via WIC into a 32‑bpp DIB or GDI+compatible HBITMAP; blitted via `AlphaBlend` (from Msimg32).
- Background: fill with solid brush (green felt RGB(0,128,0) variant) onto back buffer.
- Layout metrics derived from DPI: base card size scales with DPI (e.g., 1.0x → ~84×116 logical; actual sized by sprite & scaling).
- Spacing: horizontal/vertical offsets scale with DPI; overlapping fans for tableau face‑up and face‑down.
- Text: system UI font via `LOGFONT`/`CreateFontIndirectW`; GDI text for status bar if not using common control.
- Invalidations: precise `InvalidateRect` per region; minimize full‑window repaints.

## DPI and Visual Styles

- Manifest entries:
  - `dpiAware` Per‑Monitor v2.
  - Common Controls v6 dependency (comctl32 v6) for status bar and consistent theming.
- Handle `WM_DPICHANGED`: resize window to suggested rect; rebuild buffers; recompute layout.

## Win32 UI and Resources

- Menu (`IDR_MAINMENU`): File (New, Deal Again, Options, Exit), Game (Draw 1/3, Scoring, Auto‑complete), Edit (Undo, Redo), Help (About).
- Accelerators (`IDR_ACCEL`): `F2`, `Ctrl+N`, `Ctrl+Z`, `Ctrl+Y`, `Esc`.
- Status bar: created via `CreateStatusWindowW` or `STATUSCLASSNAME` with panes for Score | Time | Moves | Mode.
- Icon: multi‑size group icon (`IDI_APP`).
- Version info: `VS_VERSION_INFO` block.
- Embedded PNG: `IDB_CARDS` as `RCDATA`.
- Manifest: `res/app.manifest` compiled into binary.

### Example `app.rc` (illustrative)

```rc
#include <windows.h>

IDI_APP           ICON            "res/app.ico"
IDR_MAINMENU      MENU            "res/menu.rc"
IDR_ACCEL         ACCELERATORS    "res/accel.rc"
CREATEPROCESS_MANIFEST_RESOURCE_ID RT_MANIFEST "res/app.manifest"
VS_VERSION_INFO   VERSIONINFO     "res/version.rc"
IDB_CARDS         RCDATA          "res/cards.png"
```

## Engine Details

- Legal moves:
  - Tableau → Tableau: descending rank, alternating color; stacks allowed if all face‑up and ordered.
  - Tableau → Foundation: ascending rank, same suit, starting at Ace.
  - Waste → Tableau/Foundation: as above.
  - Stock recycle: when stock empty, move waste (face‑down) back to stock; rules vary with Draw 1/3 and Vegas scoring penalties.
- Scoring:
  - Standard: mirror classic Windows scoring (move to foundation +10, move from foundation −15, turn tableau card +5, time penalty if enabled; values tunable to match references).
  - Vegas: −$52 buy‑in; +$5 per card to foundation; cumulative option persists across games until reset.
- Timer: `SetTimer` at 1s tick for time/score updates; shorter tick (e.g., 16 ms) only when animating.
- Undo/Redo: store inverse of `Move` to enable precise reversal; clear redo after new user action.
- Auto‑complete: compute topologically safe sequence to foundations; animate with short timer.

## Resource Loading and WIC Pipeline

- Use `FindResourceW`/`LoadResource`/`LockResource` to get PNG bytes (RCDATA).
- Create WIC stream over memory (`IWICStream::InitializeFromMemory`), decode to `IWICBitmapSource`.
- Convert to 32‑bpp premultiplied BGRA via `IWICFormatConverter`.
- Copy pixels to DIB or create `HBITMAP` via `CreateDIBSection` and manual copy; cache surfaces.

## Error Handling and Diagnostics

- Convert `HRESULT`/Win32 errors to `anyhow::Error` with context.
- In debug builds, log to `OutputDebugStringW`; optional rotating file log in `%LOCALAPPDATA%` for troubleshooting.
- Fail‑fast on unrecoverable resource issues with a message box.

## Persistence

- Registry path: `HKCU\Software\<Vendor>\Solitaire`.
- Values: `DrawThree` (DWORD), `ScoringMode` (DWORD), `Timed` (DWORD), `VegasCumulative` (DWORD), `WindowPlacement` (BINARY), `HighScore`/`BestTime` (optional).

## Build and Packaging

- `Cargo.toml`:
  - `windows` crate with feature gates for used APIs.
  - `build = "build.rs"` to invoke `embed-resource`.
- `build.rs`:
  - Calls `embed_resource::compile("app.rc")`.
- Output: single `solitaire.exe` with embedded resources.

### Illustrative `Cargo.toml` snippets

```toml
[package]
name = "solitaire"
version = "0.1.0"
edition = "2021"
build = "build.rs"

[dependencies]
windows = { version = "*", features = [
  "Win32_Foundation",
  "Win32_Graphics_Gdi",
  "Win32_UI_WindowsAndMessaging",
  "Win32_UI_Controls",
  "Win32_UI_Shell",
  "Win32_System_Com",
  "Win32_Security_Cryptography",
  "Win32_Storage_FileSystem",
  "Win32_System_LibraryLoader",
  "Win32_Graphics_GdiPlus",
  "Win32_Graphics_Dwm",
  "Win32_Media",
  "Win32_Globalization",
  "Win32_System_Registry",
  "Win32_Graphics_Imaging", # WIC
] }
anyhow = "*"
bitflags = "*"
once_cell = "*"
thiserror = "*"

[build-dependencies]
embed-resource = "*"
```

## Testing Strategy

- Engine unit tests: move legality, scoring, shuffles, undo/redo reversibility, auto‑complete safety.
- Serialization tests: ensure options load/save round‑trip correctly.
- Rendering smoke tests: headless back‑buffer drawing to DIB; pixel sanity (basic checksum assertions in tests where feasible).
- Manual QA: DPI scaling on 100%/150%/200%; window resize; minimize/restore; drag/drop; accelerators; Vegas cumulative roll‑over.

## Risks and Mitigations

- Flicker/tearing: eliminate `WM_ERASEBKGND`; always draw into back buffer; single blit.
- DPI blurriness: render at device pixels; avoid GDI stretching of text/icons; scale layout not bitmaps when possible.
- Input edge cases: robust capture handling; cancel drag on `Esc`, `WM_CANCELMODE`, or window deactivation.
- Performance on 4K: cache scaled card surfaces if needed; avoid per‑frame WIC conversions.
- Resource integrity: validate PNG decode; show message box with guidance if assets missing (dev builds only).

## Telemetry and Privacy

- No network or telemetry. All state stored locally in HKCU.

## Open Questions

- Sounds: add simple `PlaySound` events (deal, win) with toggle? Default silent.
- Exact Windows classic scoring constants: match legacy precisely vs. approximate? (decide during implementation/testing).
- Vegas cumulative persistence: reset on user action only, or per session? (default: persist until user resets).
- Card back design: select a CC0 pattern and embed.

## Milestones

1) Skeleton app and resources
- Cargo project, `build.rs`, `app.rc`, icon, manifest, version info, menu/accelerators.
- Window creation, message loop, status bar, About dialog stub.

2) Rendering path
- WIC decode of embedded PNG; back buffer; draw one test card; DPI handling; flicker‑free paint.

3) Engine + interaction
- Game state and rules; hit‑testing; drag/drop; stock/waste; double‑click and right‑click auto‑move.

4) Scoring, options, persistence
- Standard/Vegas scoring, timer, undo/redo; Options dialog; registry persistence; status bar updates.

5) Polish and QA
- Auto‑complete animation; high‑DPI icon sizes; final About text and licenses; manual QA; performance profiling.

## Acceptance Criteria

- Ships as a single `solitaire.exe` that runs on a clean Windows 10/11 x64 install.
- Visual correctness at 100–200% DPI without flicker or blurry text.
- Functional parity with classic Solitaire: legal moves enforced, scoring modes work, undo/redo stable, auto‑complete completes.
- Options persist across runs; Vegas cumulative behaves per spec; About dialog lists version and licenses.


