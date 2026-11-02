# Culling prototype verification

Verified on Windows x64 with stable Rust 1.99.1 on 2026-11-02.

## Automated checks

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo test --workspace --locked` — 10 tests
- `cargo build --workspace --locked`

Tests cover grouping without dropping orphan/alternate files, conservative export
planning, serial-bound ADB arguments, review restoration against all source
fingerprints, changed-file reset, atomic session replacement on Windows,
preservation of corrupt/future sessions, EXIF rotation/thumbnail bounds, and
modifier handling for quick keyboard chords.

## Native UI checks

Used synthetic 1800×1200 JPEGs, one EXIF-rotated JPEG with a Japanese filename,
RAW placeholders, an ambiguous RAW pair, a RAW-only group and an unreadable JPEG.
Test sessions were isolated using `NAMIFL_STATE_DIR`.

Observed:

- Folder load, filmstrip thumbnails, dark theme and Japanese filenames render.
- K/D tagging advances and persists; close/reopen restores review decisions.
- Shift+Right selects a range; K applies to the whole range without advancing.
- Ctrl+A selects all groups; U resets the selection to Unreviewed.
- Keeper filtering works, and reclassifying a keeper keeps selection in that view.
- RAW-only groups show an explicit no-preview state; unreadable JPEGs show an error.
- Normal and maximized layouts display the review panel, filmstrip and save status.
- SHA-256 comparison of all 27 fixture files before/after review found no changes.

## Still to validate

This is not a performance or camera-compatibility certification. Actual camera
JPEGs, a representative 1,000-photo folder, high-resolution memory pressure,
visual loupe accuracy, real-shoot keyboard ergonomics and a choseek1 comparison
remain pending. ICC color management is not implemented. Linux is build/test CI
coverage, not a native desktop UI certification. No Android operation has been
implemented or tested on a phone.
