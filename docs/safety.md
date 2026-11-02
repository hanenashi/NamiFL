# File and transfer rules

These rules apply to the current culling prototype and future transport work.
The desktop reads JPEGs and source metadata and writes separate per-user review
sessions. The inspect command writes nothing. Only explicit confirmed desktop cleanup deletes source photos. Neither mode renames files, executes ADB, or performs transfers.

## Ownership and selection

- Camera files and DxO exports are distinct inputs.
- Keep/Reject/Unreviewed states do not themselves trigger operations.
- Confirmed permanent cleanup removes rejected JPEGs and reviewed RAWs, retaining
  keeper JPEGs and all unreviewed groups. It is folder-wide, not selection/filter-wide.
- Validate folder scope, regular-file type, size and mtime before each removal;
  skip changed targets and report partial failures. Deletion is not transactional.
- Export suffixes affect destination names, not source names or RAW companions.
- Never silently discard alternate JPEGs/RAWs or resolve ambiguous groups.

## Plan before executing

Show sources, destinations, skip/overwrite/rename choices and verification policy.
The executor must revalidate source fingerprints and destination state; a draft
plan is not a filesystem transaction. Changes after preflight must surface as
conflicts. Do not treat case folding as a complete Unicode collision solution.

## Android execution

- Bind calls to one explicit device serial; distinguish offline and unauthorized.
- Resolve a known ADB executable; launch arguments directly, without a host shell.
- Remote shell operations require separate Android-shell quoting and path validation.
- Use bounded timeouts and cancellable process handling, including an active push.
- Plan remote staging/finalization and partial-file handling before enabling overwrite.
- Verify transferred content before marking success; a successful push alone is insufficient.
- Journal per-file results for retry; do not claim rollback across local and remote storage.
- Report transfer verification and media indexing separately.
- Do not shut down a shared ADB server merely because NamiFL exits.

## Persistence and recovery

Review sessions currently use versioned schemas, atomic replacement and
path/size/mtime fingerprints in a per-user application directory. These are not
content hashes and do not follow files through renames/moves. Normal close waits
for pending saves; save failures require retry. Force-killing the process can lose
unsaved decisions. Concurrent reviews of the same folder in separate instances
are not supported: the last saved snapshot wins.
Preferences now persist separately; pane positions are session-local. Logs should avoid exposing personal paths unnecessarily.
Do not commit personal photos, sessions, settings, device serials or logs.
