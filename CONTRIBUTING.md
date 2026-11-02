# Development

Use stable Rust; `rust-toolchain.toml` requests rustfmt and Clippy. The current
workspace has no external crate dependencies. Commit `Cargo.lock` for reproducible
application builds.

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p namifl -- --help
```

Start with `docs/architecture.md`, `docs/roadmap.md` and `docs/safety.md`.
Do not introduce a GUI dependency before testing the photo viewport candidate.
Keep domain models free from UI image handles, widgets and framework events.
Use small behavior-focused tests for grouping, planning and failure handling;
future ADB tests should use a fake process boundary before a real device.

Do not test deletion/overwrite against a real photo library. Fixtures should be
synthetic and contain no private photographs. Keep the existing Python apps as
behavioral references during migration; copying implementation requires resolving
licensing first.
