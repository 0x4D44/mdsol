# Klondike Solitaire (Win32/Rust)

Native Windows Klondike Solitaire implemented with Rust and Win32 APIs via the `windows` crate. Ships as a single EXE with embedded resources.

## Build

Prerequisites (Windows 10/11 x64):

- Rust toolchain with MSVC target: `rustup toolchain install stable-x86_64-pc-windows-msvc`
- Visual Studio Build Tools with Windows 10/11 SDK (for `rc.exe` and link libs)
- `cargo` on PATH

Build and run:

```
cargo build --release
target\x86_64-pc-windows-msvc\release\klondike.exe
```

Notes:

- Double-buffered GDI rendering is enabled. If no card PNG is embedded, a placeholder card is drawn.
- Manifest, menu, accelerators, and version info are embedded. Icon and card assets are optional and can be added later.

## Cards: Download + Pack

We provide an `xtask` tool that downloads Byron Knoll’s Public Domain vector playing cards and packs a 13×4 PNG sprite sheet.

Usage (Windows, requires internet):

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

## Roadmap

- M1: Skeleton app/resources (done)
- M2: Rendering path (WIC decode + back buffer)
- M3: Engine + interaction
- M4: Scoring/options/persistence
- M5: Polish/QA
