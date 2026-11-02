# NamiFL

A focused Rust photo workflow: **Cull → DxO → Transport**.

NamiFL is the planned successor to [choseek1](https://github.com/hanenashi/choseek1)
and [Kuro Commander](https://github.com/hanenashi/kuro_commander). It brings photo
selection and Android delivery into one application, with development in DxO in
between. The existing repositories remain independent and unchanged.

## Workflow

1. Review camera JPEGs alongside their RAW companions and mark keepers.
2. Edit and develop the chosen photographs in DxO outside NamiFL.
3. Export finished JPEGs from DxO into a separate folder.
4. Return to NamiFL to select those exports and send them to Android.
5. Delete RAWs manually when ready. NamiFL must not automatically delete RAWs.

Camera JPEGs and developed exports are distinct selections. A keeper decision is
not a deletion instruction, and export naming must not silently rename originals.

## Current status: culling prototype

Implemented:

- An egui/eframe desktop culling workspace with UI-independent core types.
- Read-only, non-recursive JPEG/RAW grouping with orphan and ambiguity reporting.
- Oriented JPEG thumbnails and previews, zoom, pan, fit/100% view and a loupe.
- Keyboard Keep/Reject/Unreviewed tagging, multiselect and decision filters.
- Background decoding with bounded queues and a bounded texture cache.
- Background, versioned review-session saves in a per-user directory.
- Pure JPEG export draft planning with destination suffixes and duplicate checks.
- An ADB argument builder that requires an explicit device serial.
- A read-only `inspect` command and Windows/Linux CI checks.

Not implemented: actual ADB execution, remote conflict resolution, transfer
verification, EXIF exposure tooltips or local deletion. The culling prototype
does not modify, rename, delete or transfer source photos. It writes only review
sessions. This is not yet a replacement for both existing apps.

## Run

Install stable Rust using [rustup](https://rustup.rs/). Windows development also
requires the Visual Studio C++ build tools and Windows SDK.

```powershell
cargo run -p namifl
cargo run -p namifl -- cull "C:\Photos\Shoot"
cargo run -p namifl -- --help
cargo run -p namifl -- inspect "C:\Photos\Shoot"
```

The desktop accepts a folder from the Open button, Ctrl+O, a dropped folder, or
the `cull` argument. It decodes JPEGs and records review decisions separately.
The `inspect` command reads directory entries only and writes nothing.
Neither mode follows symlinks, recurses, changes photos or contacts a phone.
JPEG extensions: `.jpg`, `.jpeg`; RAW: `.pef`, `.dng`, `.nef`, `.cr2`, `.cr3`,
`.arw`, `.orf`, `.rw2`, `.raf`. RAW recognition does not imply RAW decoding.

## Review controls

| Control | Action |
| --- | --- |
| Left / Right | Previous / next group in the current filter |
| K / D / U | Keep / Reject / Unreviewed for the selection |
| Space | Toggle the current selection between Keep and Unreviewed |
| Ctrl-click / Shift-click | Add/remove a selection / select a range |
| Ctrl+A | Select all groups in the current filter |
| Ctrl+S | Retry/save the review immediately; decisions also autosave |
| Wheel / left drag | Zoom around the pointer / pan |
| F / middle click / double click | Fit the preview |
| Hold right mouse button | Show a 100% loupe |

Single-photo tagging advances by default; disable **Advance after tagging** to
stay on the photo. RAW-only and ambiguous groups remain visible. Choose which
JPEG variant to preview in the Source files panel.

Sessions live in `%LOCALAPPDATA%\NamiFL\sessions` on Windows (the platform-local
data directory elsewhere). Set `NAMIFL_STATE_DIR` to use a separate session
directory, such as for testing. Decisions restore only when the canonical folder,
source paths, file sizes and modification times match. Renamed/moved/changed
groups return to Unreviewed. These metadata checks are not content verification.
The prototype does not watch folders for external changes; reopen the folder to
rescan after adding, removing or editing source files.

This is a viewport prototype, not a color-proofing tool: ICC-managed display,
real-shoot performance comparisons, configurable shortcuts and packaging are
still pending. Previews are capped at 8192 pixels on the longest edge; the UI
identifies capped previews. Such previews and their loupe are not original-pixel
100% views. JPEG decode allocation is limited to 256 MiB; oversized images can be
rejected with an explicit error.

## Project map

| Path | Purpose |
| --- | --- |
| `apps/namifl` | Desktop UI, image worker/cache, session I/O and inspection command |
| `crates/namifl-core` | Domain, discovery and pure export planning |
| `crates/namifl-adb` | Android transport boundary, currently argument construction only |
| `docs/architecture.md` | Module boundaries and data flow |
| `docs/roadmap.md` | Migration milestones and first-prototype acceptance criteria |
| `docs/safety.md` | File ownership, operation rules and failure handling |
| `docs/verification.md` | Verification performed and remaining validation |
| `CONTRIBUTING.md` | Local checks and development expectations |

No license has been selected yet. Public visibility is not a grant of an
open-source license. Resolve licensing before copying legacy implementation or
redistributing third-party binaries; no ADB binaries are included here.
