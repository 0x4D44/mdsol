# Project Context: Solitaire (mdsol)

## Overview
This is a native Windows **Klondike Solitaire** implementation written in **Rust**. It avoids high-level cross-platform GUI frameworks (like winit, druid, or bevy) in favor of direct **Win32 API** usage via the `windows` crate. This results in a lightweight, single-executable application with deep OS integration (GDI rendering, native menus, resource embedding).

## Architecture

### 1. Core Logic (`src/engine.rs`)
*   **Pure Rust:** No `windows` dependencies.
*   **State:** `GameState` struct manages the board (stock, waste, foundations, tableaus).
*   **Features:**
    *   Deck generation and shuffling (using `BCryptGenRandom` via `engine.rs` helper, though the logic itself is agnostic).
    *   Move validation (`can_place_on_tableau`, `can_place_on_foundation`).
    *   Solvability checking (interfaces with `src/solver.rs`).
    *   Score and move tracking.

### 2. Application Layer (`src/main.rs`)
*   **Win32 Entry Point:** `WinMain` (via `#[windows_subsystem]`) sets up the window class and message loop.
*   **Event Handling:** A standard `wndproc` handles `WM_PAINT`, `WM_LBUTTONDOWN`, `WM_COMMAND`, etc.
*   **Rendering:**
    *   **GDI (Graphics Device Interface):** Uses `Double Buffering` (`CreateDIBSection`) to prevent flickering.
    *   **Assets:** Loads card sprites from an embedded resource (`IDB_CARDS`) decoded via **WIC** (Windows Imaging Component).
    *   **High DPI:** Manually calculates scaling in `CardMetrics` based on window client size.
*   **State Management:** `WindowState` struct (boxed and stored in `GWLP_USERDATA`) holds the game state, GDI resources (brushes, DCs), and UI state (drag contexts, animations).

### 3. Build & Tooling (`xtask/`)
*   **Pattern:** Uses the "xtask" pattern (workspace member) for development tasks.
*   **Asset Pipeline:**
    *   Downloads open-source vector playing cards (e.g., Byron Knoll's).
    *   Rasterizes SVGs using `usvg` / `resvg`.
    *   Packs them into a single sprite sheet (`res/cards.png`).
    *   Updates `res/app.rc` to include the new sprite sheet.
*   **Command:** `cargo run -p xtask -- gen-cards`

## Key Files
*   `src/main.rs`: The "View" and "Controller". Handles Windows messages, painting, and input.
*   `src/engine.rs`: The "Model". Game rules and state manipulation.
*   `xtask/src/main.rs`: Asset generation tool.
*   `res/app.rc`: Resource script defining the icon, menu, accelerators, and embedded PNG.
*   `res/cards.png`: The sprite sheet used at runtime (13 columns x 4 rows).

## Build & Run

### Prerequisites
*   Windows 10/11 x64.
*   Rust (stable MSVC).
*   Visual Studio Build Tools (for `rc.exe` and linker).

### Commands
*   **Run Dev:** `cargo run`
*   **Build Release:** `cargo build --release`
*   **Generate Assets:** `cargo run -p xtask -- gen-cards`
*   **Create Installer:** `iscc installers/Solitaire.iss` (Requires Inno Setup)

## Conventions
*   **Style:** Standard Rust (`rustfmt`).
*   **Win32 Interop:**
    *   Use `windows` crate types (`HWND`, `HDC`, `PCWSTR`).
    *   Resource IDs are defined in `src/constants.rs` (implied) or inline constants to match `resource.h`.
    *   Strings are typically converted to UTF-16 (`Vec<u16>`) for Win32 APIs (`to_wide` helper).
*   **Safety:** `unsafe` blocks are pervasive due to FFI but are generally encapsulated in wrappers or grouped within Win32 API call sites.

## Game Features
*   **Modes:** Draw 1 / Draw 3.
*   **Undo/Redo:** Unlimited history stack.
*   **Victory:** Detects win condition; features "Classic" (bouncing cards) and "Modern" animations.
*   **Persistence:** Saves window position and size to Registry.
