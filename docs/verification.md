# Culling prototype verification

Verified on Windows x64 with stable Rust 1.99.1 on 2026-11-02.

## Automated checks

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `cargo test --workspace --locked` — 18 tests; one manual timing test excluded by default
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

This is not a performance or camera-compatibility certification. A representative 1,000-photo folder, high-resolution memory pressure,
visual loupe accuracy, real-shoot keyboard ergonomics and a choseek1 comparison
remain pending. ICC color management is not implemented. Linux is build/test CI
coverage, not a native desktop UI certification. No Android operation has been
implemented or tested on a phone.

## Version 0.2 checks

Used the user-provided JPEGs in E:\TEST\100_1030 (28 JPEG+RAW groups).
The first JPEG is 1728x1152 and about 1.27 MB. No RAW decoder is used.
A five-iteration timing test of that JPEG measured preview decoding at 794.0 ms
before optimization and 28.2 ms afterward. Thumbnail generation changed from
799.1 ms to 37.6 ms, while increasing its output bound from 240x180 to 480x360.
These are decode/render timings on one file, not end-to-end UI latency.

Added tests cover keeper-JPEG cleanup policy, preservation of unreviewed files,
changed-source and out-of-folder skips, surviving keeper decisions, preference
persistence/validation, and loupe-level validation. Cleanup tests use disposable
synthetic folders, not the user's original photographs.

Native checks include movable panes, version/GitHub/settings display, and
middle-click switching the loupe from 100% to 200%. Pane positions currently reset
on restart; visibility and thumbnail/loupe preferences persist.

Final native checks confirmed that an ordinary vertical mouse wheel scrolls the
thumbnail strip horizontally and the size slider enlarges thumbnail cards.
Keeper/reject cards have green/red backgrounds. The cleanup preview for one
keeper and one rejected group listed one rejected JPEG and both RAWs, retaining
the keeper JPEG and excluding 26 unreviewed groups. It was cancelled; the test
folder still contains all 28 original JPEGs and 28 RAWs.

## Version 0.2.1 checks

Three regression tests cover boundary arrow-key no-ops, repeated last-card reveal
requests without overshoot/recentering, and strip/grid selection from pane shape.
Native verification used the currently available 13 groups in E:\TEST\100_1030.
Confirmed a three-column tall grid, a shallow review pane with all tagging controls
visible, movement by the slim grip, resizing back to a strip with the last photo
selected, and an extra Right key causing no movement in either layout. No source
files were changed during these checks. The app was left in the tall-grid layout.
