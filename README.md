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

## Current status: skeleton

Implemented:

- A dependency-free Cargo workspace with UI-independent core types.
- Read-only, non-recursive JPEG/RAW grouping with orphan and ambiguity reporting.
- Unreviewed/Keep/Reject decision types; no filesystem side effects.
- Pure JPEG export draft planning with destination suffixes and duplicate checks.
- An ADB argument builder that requires an explicit device serial.
- A read-only `inspect` command and Windows/Linux CI checks.

Not implemented: desktop GUI, image decoding, saved review sessions, actual ADB
execution, remote conflict resolution, transfer verification, or deletion.
This is not yet a replacement for either existing app.

## Run

Install stable Rust using [rustup](https://rustup.rs/). Windows development also
requires the Visual Studio C++ build tools and Windows SDK.

```powershell
cargo run -p namifl -- --help
cargo run -p namifl -- inspect "C:\Photos\Shoot"
```

The command reads directory entries only. It does not read image contents, follow
symlinks, recurse, write settings, rename files, delete files, or contact a phone.
JPEG extensions: `.jpg`, `.jpeg`; RAW: `.pef`, `.dng`, `.nef`, `.cr2`, `.cr3`,
`.arw`, `.orf`, `.rw2`, `.raf`. RAW recognition does not imply RAW decoding.

## Project map

| Path | Purpose |
| --- | --- |
| `apps/namifl` | Read-only developer harness; future desktop composition root |
| `crates/namifl-core` | Domain, discovery and pure export planning |
| `crates/namifl-adb` | Android transport boundary, currently argument construction only |
| `docs/architecture.md` | Module boundaries and data flow |
| `docs/roadmap.md` | Migration milestones and first-prototype acceptance criteria |
| `docs/safety.md` | File ownership, operation rules and failure handling |
| `CONTRIBUTING.md` | Local checks and development expectations |

No license has been selected yet. Public visibility is not a grant of an
open-source license. Resolve licensing before copying legacy implementation or
redistributing third-party binaries; no ADB binaries are included here.
