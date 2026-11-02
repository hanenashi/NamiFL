# Architecture

## Product boundary

NamiFL has two future workspaces: Cull and Transport. DxO remains external; there
is no editor, RAW developer, catalog, phone file manager, or automatic cleanup. Confirmed cleanup can remove reviewed RAWs.
Windows is the first desktop target. The core is platform-independent.

## Present code

`apps/namifl` composes an egui/eframe desktop UI with `namifl-core`.
`namifl-adb` is an independent adapter stub; no ADB processes run yet.
The desktop uses the Glow renderer, native folder dialogs and system font fallback
for filenames. There are no empty service crates.

`desktop.rs` owns selection, filtering, viewport interaction and worker messages.
`media.rs` has a single decoder worker, at most 24 queued requests and two queued
results. Visible thumbnails are requested lazily. Full previews take priority;
one next-image preview is prefetched when the current image is small enough.
Texture storage is bounded to 64 entries and a 160 MiB budget, except that the
active preview is pinned and may exceed the budget alongside a small number of
thumbnail uploads. Oversized adjacent previews are declined rather than evicting
the active image repeatedly. There is no disk image cache.

`session.rs` owns a separate I/O worker for folder selection/scanning, metadata
fingerprinting, restoration and saves. Grouping is published once directory
discovery/fingerprinting finishes; JPEG decoding does not block opening the list.
Session filenames are SHA-256 digests of the canonical folder's native path bytes.
Records use exact stems and sorted source-path/size/mtime fingerprints. This is
stable local identity across restarts, not a portable catalog ID or content hash.
All source variants participate in the fingerprint; changed membership resets the
decision. Saves use a flushed/synced temporary file and atomic replacement. A
failed save is visible, blocks folder switching, and prevents a normal close
until retried. Corrupt/future-version sessions block loading and are not replaced.

The core groups regular files by exact stem within one folder. It preserves all
JPEG and RAW variants, including orphans, rather than choosing a last-seen file.
Extensions are case-insensitive; stems are case-sensitive. Non-UTF-8 stems are
reported as unsupported. Review decisions default to Unreviewed. Explicit confirmed cleanup removes reviewed RAWs and rejected JPEGs, retaining keeper JPEGs and all unreviewed groups. Metadata fingerprints cannot detect same-size changes
that retain the modification time and are unsuitable for transfer verification.

Export planning accepts an explicit JPEG selection from the developed export
folder. It returns unchanged source paths plus destination names. Current checks
are draft checks, not a guarantee of valid Windows filenames or remote availability.

## Intended boundaries

| Module | Responsibility |
| --- | --- |
| Domain/discovery | File records, stable asset identity, groups, decisions, ambiguity |
| Media | JPEG decode, EXIF orientation, thumbnails, bounded cache and preview prefetch |
| Operations | Pure plans, destination policy, conflicts and source validation |
| Jobs | Background execution, cancellation, structured events and durable journal |
| Android | ADB resolution, serial-bound calls, remote checks, push and verification |
| Persistence | Versioned preferences and review sessions; atomic writes |
| Desktop | Cull/Transport views, keyboard navigation, plan previews and job status |

Media and session workers now live in the desktop application; extract them only
when reuse justifies a separate crate. Keep UI framework types out of core domain
objects and plans. egui/eframe is the viewport prototype dependency; real photo
workflow testing still decides whether it is suitable long-term.

## Intended flow

1. Scan a camera folder; publish groups incrementally.
2. Review JPEGs; save decisions without changing source files.
3. Work in DxO externally; export finished JPEGs to a separate folder.
4. Select that folder in Transport; draft destination names.
5. Select an authorized Android device and resolve conflicts.
6. Revalidate sources, execute transfers, verify destinations, and report media-scan results.

No inferred mapping from camera JPEGs to DxO filenames is required for the first
prototype. No transfer or cleanup starts merely because a photo is marked Keep.

## Legacy references

- choseek1 reviewed at `432f83523566faa36bd12a12149737947a7f6416`.
- kuro_commander reviewed at `99fdc63b1b2540a4bdae418c03518d8f3973ce77`.

Preserve useful interaction and conflict policies, not their UI coupling.
In particular, do not carry over deletion of keeper JPEGs, automatic local
renaming during transport, synchronous remote preflight on the UI thread, or
implicit ADB device selection.

## Culling refinements (0.2)

`workspace.rs` renders movable, resizable egui window panes, Settings and a modal
cleanup preview. Preferences save atomically via the I/O worker. Pane visibility,
thumbnail width, auto-advance and loupe levels persist; pane positions are local
to the running session. All panes can be restored from View.

JPEG decoding and pixel conversion are optimized even in development builds.
The decoder retains at most two recent decoded JPEGs, up to 32 MiB each, to reuse
preview/thumbnail work. Thumbnail requests leave four queue slots for previews.
No RAW decoding occurs.

Confirmed cleanup revalidates canonical folder scope and size/mtime for each
regular file, reports failures, reloads the folder and updates surviving review
fingerprints. Keeper decisions survive removal of their RAW companions.
