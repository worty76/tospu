# Tospu

A tiny, gentle desktop companion for your Mac.

A few times a day (or on any schedule you pick), a cute character pops into the corner of your screen with one of your **big goals**, a **quote**, something you're **grateful** for, or something to be **proud** of—then waves goodbye and disappears.

<!-- Screenshot placeholder: add a screenshot of the popup and Settings window here -->

## Why Tospu?

- **Private by default.**
  - **Quiet mode** (`⌘⇧Y` on macOS, or from the menu bar) shows only `"Keep going."` until you click to reveal the real message—handy when sharing your screen or working around others.
  - **Privacy blur** keeps your messages blurred in the Settings window until you hover over them.
  - **100% offline.** Everything stays in plain files on your computer. No accounts, no tracking, no internet required.
- **Never interrupts your work.** Tospu never steals keyboard focus, clicks pass right through around the sprite and bubble, and quiet hours keep it asleep overnight.
- **Your pace, your screen.** Pop up at specific times each day or every N minutes, and choose whether Tospu follows your cursor or stays on a specific monitor.
- **Make it yours.** Add your own messages, switch characters anytime, or create your own character in minutes—your menu bar icon even changes to match your character.

## Install (macOS)

Download the latest app from [Releases](https://github.com/worty76/tospu/releases/latest) and drag **Tospu** into Applications.

Tospu ships for macOS today. **Windows and Linux versions are coming soon.**

Tospu is **not code-signed with an Apple Developer certificate yet**, so macOS blocks the first launch. To open it:

1. Double-click Tospu. When macOS blocks it, click **Done**.
2. Open **System Settings → Privacy & Security**, scroll to **Security**, and click **Open Anyway** next to "Tospu was blocked". Confirm with your password.

You only need to do this once. On macOS 14 and earlier you can right-click the app and choose **Open** instead.

If macOS says the app is "damaged", remove the download quarantine flag and open it again:

```bash
xattr -dr com.apple.quarantine /Applications/Tospu.app
```

If you would rather not bypass anything, you can build it from source yourself (see the end of this page).

## Using Tospu

Tospu lives quietly in your menu bar. Click its icon to:

- **Show one now** — call your companion whenever you want a quick boost.
- **Quiet mode** — hide private text behind `"Keep going."` (`⌘⇧Y`).
- **Character** — switch between installed companions.
- **Settings…** — manage your messages, schedule, character, startup, and screen preferences.

## Add or contribute a character

Every character is just a folder of images and a small config file—no coding needed.

Want to make your own (with ready-to-paste ChatGPT prompts) or share one with everyone? Check out [docs/creating-a-sprite-pack.md](docs/creating-a-sprite-pack.md).

## Where your data lives

Your messages, settings, and custom characters are stored locally on your machine:

- **macOS:** `~/Library/Application Support/io.github.worty76.tospu/`
- **Windows:** `%APPDATA%\io.github.worty76.tospu\`
- **Linux:** `~/.local/share/io.github.worty76.tospu/`

*(If you're upgrading from the earlier Yub version, Tospu copies your old data over automatically on first launch without deleting the original.)*

---

<details>
<summary><strong>Build from source (for developers)</strong></summary>

Tospu is written in Rust with Tauri 2 and Leptos (WASM)—no Node.js or npm needed.

### Prerequisites

Install Rust (stable), Xcode Command Line Tools (on macOS), and the build tools:

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
cargo install tauri-cli --version "^2" --locked
```

### Run & package

```bash
cargo tauri dev     # run in dev mode
cargo tauri build   # build Tospu.app / .dmg
```

### Run checks & tests

```bash
trunk build --config ui/Trunk.toml
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p tospu-ui --target wasm32-unknown-unknown -- -D warnings
cargo test --workspace
```

</details>
