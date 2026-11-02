# File and transfer rules

These are requirements for future implementation; the skeleton only inspects
folders and builds draft data. It performs no transfers or file mutations.

## Ownership and selection

- Camera files and DxO exports are distinct inputs.
- Keep/Reject/Unreviewed states do not themselves trigger operations.
- RAW deletion is manual and outside the application workflow.
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

Store preferences/sessions in a per-user application directory with versioned
schemas and atomic replacement. A session must identify the actual source file,
not just its list index. Logs should avoid exposing personal paths unnecessarily.
Do not commit personal photos, sessions, settings, device serials or logs.
