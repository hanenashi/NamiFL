# Roadmap

## 0 — Foundation (this skeleton)

Workspace, read-only discovery, decision model, export draft planning, serial-bound
ADB argument construction, developer harness, documentation and CI.

## 1 — Cull viewport prototype

- Add a desktop viewport spike; evaluate egui/eframe on actual camera JPEGs.
- Decode oriented JPEGs on workers, with bounded thumbnails and adjacent-image prefetch.
- Add keyboard Keep/Reject/Unreviewed tagging, multiselect, zoom, pan and loupe.
- Add stable asset IDs, source fingerprints and versioned review-session persistence.
- Display RAW-only groups and ambiguous variants explicitly.

Use the same photo set as choseek1 to compare first-image latency, navigation,
memory consumption, keyboard behavior, orientation and fit/100% viewing.
Do not assume Rust alone makes the workflow faster.

## 2 — First integrated prototype: Cull → DxO → Transport

- Complete a review, close/reopen, and restore decisions.
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
