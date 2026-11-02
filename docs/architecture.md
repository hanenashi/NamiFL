# Architecture

## Product boundary

NamiFL has two future workspaces: Cull and Transport. DxO remains external; there
is no editor, RAW developer, catalog, phone file manager, or automatic RAW cleanup.
Windows is the first desktop target. The core is platform-independent.

## Present code

`apps/namifl` depends on `namifl-core`. `namifl-adb` is an independent adapter stub.
There is no GUI dependency or process runner yet. Keep this initial graph small;
do not create empty crates for every anticipated service.

The core groups regular files by exact stem within one folder. It preserves all
JPEG and RAW variants, including orphans, rather than choosing a last-seen file.
Extensions are case-insensitive; stems are case-sensitive. Non-UTF-8 stems are
reported as unsupported. Review decisions default to Unreviewed. Group membership
is a discovery result, not durable identity: stable asset IDs and source
fingerprints are required before saved sessions and mutation support.

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

Add media/jobs/persistence modules when their first behavior is implemented. Keep
UI framework types out of domain objects and plans. The initial GUI candidate is
egui/eframe, subject to a viewport spike; this is not a committed dependency yet.

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
