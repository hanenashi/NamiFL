# Roadmap

## 0 — Foundation (complete)

Workspace, read-only discovery, decision model, export draft planning, serial-bound
ADB argument construction, developer harness, documentation and CI.

## 1 — Cull viewport prototype (implemented; real-shoot evaluation pending)

- [x] Desktop viewport using egui/eframe.
- [x] Worker-decoded oriented JPEGs, bounded thumbnails and next-image prefetch.
- [x] Keyboard Keep/Reject/Unreviewed tagging, multiselect, zoom, pan and loupe.
- [x] Stable path-based group identity, metadata fingerprints and versioned sessions.
- [x] Explicit RAW-only groups and ambiguous variants.
- [ ] Evaluate actual camera JPEGs and compare interaction/performance with choseek1.
- [ ] Decide color-management requirements and original-pixel handling above the preview cap.

Use the same photo set as choseek1 to compare first-image latency, navigation,
memory consumption, keyboard behavior, orientation and fit/100% viewing.
Do not assume Rust alone makes the workflow faster.

## 2 — First integrated prototype: Cull → DxO → Transport

- Complete a review, close/reopen, and restore decisions (available in milestone 1).
- Develop selected photographs externally in DxO and export JPEGs separately.
- Open the exported folder as an independent Transport selection.
- Discover ADB, select a device, and show unauthorized/offline states.
- Preview destination names and resolve remote/within-selection conflicts.
- Skip verified identical destinations; do not rename local originals.
- Copy, verify remote SHA-256, request a media scan, and show separate outcomes.
- Persist per-file results and allow safe retry after a disconnect or restart.

Acceptance criteria:

- No source file is modified, renamed or deleted in this prototype.
- Every device-specific command is bound to the selected serial.
- Navigation remains responsive while decoding, hashing or transferring.
- Unicode, spaces, apostrophes, duplicate names, changed sources, a full phone,
  disconnects and cancellation have explicit tested outcomes.
- A media-scan failure cannot be reported as verified gallery availability.
- Retry does not silently overwrite unrelated phone files or create duplicate exports.
- Document measurements on a representative 1,000-photo folder and real phone;
  agree performance thresholds after establishing the baseline.

## 3 — Usable replacement

Polish shortcuts and preferences, import compatible legacy settings deliberately,
test Windows packaging, choose a license, document ADB provenance/distribution,
and run the app on real shoots before retiring either legacy utility.

RAW deletion remains manual. Any future local JPEG removal or explicit renaming
requires a separate design with preview, recovery and source revalidation.

## Deferred

RAW rendering/development, DxO automation, image editing, catalogs, recursive
phone browsing, dynamic plugins, multi-device transfers, automatic RAW deletion,
and non-Windows desktop packaging.
