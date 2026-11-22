# Tospu

A tiny, private desktop companion written entirely in Rust (Tauri 2 + Leptos/WASM). A few times a day a cute sprite hops into the corner of your screen with one of your **big goals**, a **quote**, something you're **grateful** for, or something to be **proud** of—then falls asleep and disappears.

<!-- Screenshot placeholder: add a screenshot of the popup and Settings window here -->

## Highlights

- **Private by design.**
  - **Quiet mode** (`⌘⇧Y` on macOS, `Ctrl+Shift+Y` elsewhere, or from the menu bar) shows only `"Keep going."` until you click to reveal the real message.
  - **Privacy blur** (on by default) blurs message text in the Settings window until you hover or focus a row.
  - **100% local.** All messages and settings stay in plain JSON files on your machine—no accounts, no telemetry, no network calls.
- **Never in your way.** The popup never steals keyboard focus, clicks pass through everywhere outside the sprite and speech bubble, and quiet hours keep it silent overnight.
- **Easy to manage.** Open **Settings…** from the menu bar to manage messages, schedule, character packs, launch at login, and privacy options.
- **Swappable characters.** Drop a folder of sprite sheets plus a `manifest.json` into your sprites folder. See [docs/creating-a-sprite-pack.md](docs/creating-a-sprite-pack.md).

## Prerequisites

- Rust stable (edition 2021) + Xcode Command Line Tools on macOS
- WebAssembly target, `trunk`, and `tauri-cli`:

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
cargo install tauri-cli --version "^2" --locked
```

No Node.js or npm is required.

## Run and build

```bash
cargo tauri dev     # run in development (serves UI on http://localhost:1430)
cargo tauri build   # build Tospu.app / .dmg into target/release/bundle
```

## Where your data lives

Tospu stores `settings.json`, `content.json`, and user `sprites/` in your OS application data directory:

| OS | Path |
|---|---|
| macOS | `~/Library/Application Support/io.github.worty76.tospu/` |
| Linux | `~/.local/share/io.github.worty76.tospu/` |
| Windows | `%APPDATA%\io.github.worty76.tospu\` |

On first launch, if legacy data exists from the earlier Yub build (`dev.yub.companion`), Tospu copies it over once without deleting the original folder. If `content.json` or `settings.json` fails to parse, Tospu never overwrites it—fix the file and click **Reload files** in **Settings → General**.

## Architecture

```
crates/tospu-model   serde types & pure logic (native + wasm32): settings, content v2, manifest, anim
crates/tospu-core    native core: atomic writes, settings/content stores, pack scan, scheduler, picker
src-tauri            Tauri 2 shell: popup & settings windows, tray, global shortcut, autostart, IPC
ui                   Leptos 0.8 CSR app (wasm32, built with trunk): ?view=popup and ?view=settings
sprites/             bundled character packs (mochi, mochi-bounce, mochi-fluffy)
tools/               sprite-sheet slicer (Python)
```

## Checks & tests

```bash
trunk build --config ui/Trunk.toml
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p tospu-ui --target wasm32-unknown-unknown -- -D warnings
cargo test --workspace
```
