# Development

Use stable Rust; `rust-toolchain.toml` requests rustfmt and Clippy. The current
desktop uses egui/eframe, image, native dialogs and per-user session storage.
Commit `Cargo.lock` for reproducible application builds. On Debian/Ubuntu, install
`libfontconfig1-dev`, `libxkbcommon-dev`, `libwayland-dev` and `libdbus-1-dev` for
desktop builds; CI installs these explicitly.

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p namifl -- --help
```

Start with `docs/architecture.md`, `docs/roadmap.md` and `docs/safety.md`.
The desktop defaults to the culling workspace; use `inspect` for headless checks.
Set `NAMIFL_STATE_DIR` to an isolated directory during UI testing.
Keep domain models free from UI image handles, widgets and framework events.
Use small behavior-focused tests for grouping, planning and failure handling;
future ADB tests should use a fake process boundary before a real device.

Do not test deletion/overwrite against a real photo library. Fixtures should be
synthetic and contain no private photographs. Keep the existing Python apps as
behavioral references during migration; copying implementation requires resolving
licensing first.
