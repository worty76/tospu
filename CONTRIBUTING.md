# Contributing to Tospu

Thanks for wanting to help make Tospu better! Whether you're drawing a cute new character, reporting a bug, or improving the Rust code, contributions are very welcome.

## Ways to contribute

- **Add a character (no coding needed):** Create a sprite pack and share it with everyone.
- **Report bugs or share ideas:** Open an issue describing what happened (include your OS and monitor setup if it's a window or display issue).
- **Improve the app:** Fix bugs, polish the UI, or help test on Windows and Linux.

---

## Contributing a character

1. Follow [docs/creating-a-sprite-pack.md](docs/creating-a-sprite-pack.md) to generate (or draw) and slice your character.
2. Keep characters **original** (no copyrighted anime, game, or brand characters) and set `"author"` and `"license"` (such as `"CC-BY-4.0"` or `"CC0-1.0"`) in `manifest.json`.
3. Place the pack folder in `sprites/<id>/`.
4. Test it locally with `cargo tauri dev` and make sure `cargo test --workspace` passes.
5. Open a pull request with a screenshot or short screen recording of your character!

---

## Contributing code

### Setup

Tospu is a pure-Rust workspace (Tauri 2 + Leptos/WASM). You'll need:

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
cargo install tauri-cli --version "^2" --locked
```

Run the app in development mode:

```bash
cargo tauri dev
```

### Guidelines to keep in mind

- **Keep the popup light and unobtrusive:** It must never steal keyboard focus, and clicks outside the sprite/bubble must pass through.
- **Protect user data:** Never overwrite or delete a user's `settings.json` or `content.json` if it fails to parse. All writes go through `tospu_core::fsutil::atomic_write`.
- **Keep it 100% Rust:** No Node.js, npm, or hand-written JavaScript.
- **Write tests** for pure logic in `crates/tospu-model` and `crates/tospu-core`.

### Before opening a PR

Run the same checks that CI runs:

```bash
trunk build --config ui/Trunk.toml
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo clippy -p tospu-ui --target wasm32-unknown-unknown -- -D warnings
cargo test --workspace
```

Keep pull requests small and focused, and include a brief note on what changed and how you tested it.
