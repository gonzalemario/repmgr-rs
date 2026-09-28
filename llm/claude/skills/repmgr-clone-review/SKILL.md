---
name: repmgr-clone-review
description: Review repmgr-rs's standby provisioning/cloning logic (creating a new replica from a backup source) against real repmgr's clone methods, especially the pgbackupapi integration. Use when the user is working on standby clone/create-replica, backup-source integration, or provisioning workflows in repmgr-rs.
---

# repmgr clone/provisioning review

You are reviewing **Mario's own Rust code** (he writes it; you review it — never write or edit his implementation files unless he explicitly asks). Unlike the other `repmgr-*-review` skills, **Mario personally designed and authored this subsystem upstream** (the pgbackupapi clone mode) — the rest of repmgr's architecture was already locked in before he joined. Ask him directly about intent/edge cases here; don't infer his own past reasoning from the C code alone the way you would for a subsystem he only maintained.

## Ground truth: how real repmgr provisions a new standby

Source: `~/repmgr-5.5.0/pgbackupapi.c`, `pgbackupapi.h`, `repmgr-action-standby.c` (the `standby clone` command — huge file, grep for `pg_backupapi`/`barman`/`pg_basebackup` rather than reading it whole), `configfile.h`/`configdata.c` (`pg_backupapi_*`, `barman_*`, `use_replication_slots`, `pg_basebackup_options`, `restore_command`, `tablespace_mapping` settings).

- **Multiple clone methods exist as alternatives**, selected by which config is present: `pg_basebackup` (direct, via libpq), `rsync`/`ssh` (older, direct filesystem copy), `barman` (via `barman-cli`/ssh invocation), and **pgbackupapi** — Mario's addition — which talks to Barman's `pg-backup-api` HTTP service instead of shelling out to barman-cli directly.
- **pgbackupapi flow is async/poll-based, not a single blocking call**: `create_new_task()` POSTs `{operation_type, backup_id, remote_ssh_command, destination_directory}` to `http://<host>:7480/servers/<node_name>/operations` and gets back an `operation_id`; `get_status_of_operation()` then GETs `.../operations/<operation_id>` repeatedly until the recovery operation completes. Any Rust equivalent needs this same submit-then-poll shape, including a real timeout/give-up condition on the polling loop (check `repmgr-action-standby.c` for how the C code bounds this — don't assume it polls forever).
- **Known-sharp-edge in the C implementation worth naming explicitly, not silently copying**: `MAX_BUFFER_LENGTH` is `72` bytes (`pgbackupapi.h`), and `receive_operation_id`/`receive_operation_status` `strncpy` JSON field values into fixed buffers of that size. A `backup_id` or `operation_id` from a real pg-backup-api deployment (often a UUID or timestamp-based string) can plausibly exceed that, silently truncating. Also `json_tokener_parse(content)` results are used (`json_object_object_get(root, ...)`) without checking `root` for NULL first — a malformed/empty HTTP response body would be a null-pointer dereference in C. These are exactly the kind of thing to ask Mario about directly: was this a known/accepted limitation, or has it actually bitten in production?
- Error surface today is thin: `curl_easy_perform` return (`CURLcode`) is the only signal bubbled up per call; `CURLOPT_FAILONERROR` treats HTTP >=400 as a curl error, but there's no structured distinction between "network unreachable," "API returned 4xx," "API returned malformed JSON," and "operation itself failed" once you're inside `operation_status`.

## What to check in his code

1. Is the submit → poll → complete/fail flow modeled as an explicit state machine (e.g. an enum of operation states) rather than a blocking loop, so timeouts/cancellation/retries are first-class?
2. Are backup/operation IDs treated as unbounded strings (`String`, not fixed-size buffers) — Rust naturally avoids the C truncation bug, but confirm nothing reintroduces a length assumption when serializing/deserializing.
3. Is a malformed or unexpected JSON response (missing field, wrong type, empty body) a typed, recoverable error, or does it panic? This is the direct Rust-safety upgrade over the C null-deref risk.
4. Does polling have bounded retries/backoff and a clear "gave up" outcome distinguishable from "backup API says it failed"?
5. How does this clone method plug into the rest of standby creation (data directory setup, replication slot creation, `restore_command`/`recovery.conf`-equivalent generation) — is that boundary clean, or does pgbackupapi-specific logic leak into generic clone code?

Since Mario authored this originally, prefer questions like "did the 72-byte buffer ever actually cause a real issue, or was it always in practice bounded?" over asserting it's a bug. Deliver findings as: the concrete mechanic in the C version, what his Rust version does (or doesn't yet do), and — where relevant — a direct question about the original design's intent rather than a guess.
