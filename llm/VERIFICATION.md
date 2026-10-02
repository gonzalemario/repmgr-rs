# repmgr-rs verification checklist

Living checklist of concrete scenarios repmgr-rs should survive, derived from real repmgr's
historical failure modes (`~/repmgr-5.5.0/HISTORY`) and from the `repmgr-*-review` skills'
findings. Check off `[x]` only when a real test actually covers the scenario — never from
reading code alone.

## Primary suitability check (`primary check` — pre-registration validation)

Scope note: these 6 scenarios need no metadata schema — they're pure Postgres server-state /
GUC introspection. Checks that write to a repmgr-rs metadata table (duplicate-primary guard,
duplicate-node-id guard, schema bootstrap) are deferred below until that schema is designed.

- [ ] accepts a properly configured primary
  Trigger: fresh Postgres, wal_level=replica, max_wal_senders>=2, hot_standby=on,
  archive_mode=on, archive_command set, not in recovery, supported PG major version
  Expected: check passes, exit 0
  Source: doc/quickstart.xml (PostgreSQL configuration section); repmgr-action-primary.c do_primary_register()
  Status: not implemented

- [ ] rejects a standby (in recovery) as primary
  Trigger: target Postgres is in recovery (pg_is_in_recovery() = true)
  Expected: check fails with a message naming the actual problem, not a generic error
  Source: repmgr-action-primary.c do_primary_register(), RECTYPE_STANDBY branch
  Status: not implemented

- [ ] rejects wal_level=minimal
  Trigger: postgresql.conf sets wal_level=minimal
  Expected: check fails, names wal_level and the required value (replica or logical)
  Source: doc/quickstart.xml — repmgr-rs improvement over upstream, which never checks this
  Status: not implemented

- [ ] rejects insufficient max_wal_senders
  Trigger: max_wal_senders=0 or 1
  Expected: check fails, names max_wal_senders and the required minimum
  Source: doc/quickstart.xml note — pg_basebackup alone needs 2 free WAL senders
  Status: not implemented
  Open question: is the minimum "2" flat, or "2 + number of standbys planned"? Your call.

- [ ] rejects archive_mode=off / empty archive_command
  Trigger: archive_mode=off (Postgres default) or archive_command unset
  Expected: check fails naming the missing setting
  Source: doc/quickstart.xml
  Status: not implemented
  Open question: hard requirement or advisory warning? Upstream repmgr doesn't check this at
  all — deciding to enforce it is a deliberate repmgr-rs improvement, but "enforce" vs "warn"
  is your call.

- [ ] rejects anything other than PostgreSQL 18
  Trigger: target server is not PG 18 (current single-version target — see Cross-cutting)
  Expected: check fails clearly instead of failing confusingly later
  Source: repmgr-action-primary.c check_server_version()
  Status: not implemented

- [ ] connection failure surfaces a clear error, never a panic
  Trigger: Postgres unreachable (wrong host/port, container down, auth failure)
  Expected: check fails with the real connection error, exits non-zero, no panic/unwrap
  Source: general Rust-safety expectation; current src/dbconnector.rs already has the shape
  of this (error_connecting) but doesn't yet turn it into a check result
  Status: not implemented

## Deferred — needs a metadata-schema decision first

These map to upstream's create_repmgr_extension() / get_primary_node_id() / get_node_record():

- [ ] schema/registry bootstrap equivalent (repmgr-rs now has a pgrx extension in `pg/`, but
  so far it only holds shared-memory node state, with no tables. The metadata table design is
  still undecided)
- [ ] rejects registering a second active primary when one is already registered
- [ ] rejects re-registering an existing node_id without an explicit --force-equivalent
  Source: repmgr-action-primary.c do_primary_register(), lines checking current_primary_id
  and get_node_record()
  Status: blocked on metadata schema design (see repmgr-topology-review)

## Extension shared-memory state (`pg/` crate)

Scope note: `pg/src/lib.rs` currently has `RepmgrNode` in a `PgLwLock` and exposes only
`get_local_node_id()`. Scenarios for state that doesn't exist yet are deferred below. Reference
for the upstream behaviour: `llm/EXTENSION_FUNCTIONS.md`.

- [ ] get_local_node_id returns the "unset" value before anything sets it
  Trigger: fresh Postgres with `repmgr` preloaded; call `get_local_node_id()` before any setter
  Expected: returns -1 (upstream `UNKNOWN_NODE_ID`)
  Source: repmgr.c:194 (shmem init), repmgr.h:92
  Status: implemented untested. `test_get_local_node_id` in `pg/src/tests.rs` exists, but isn't
  confirmed passing yet: the crate didn't compile until the `pub use tests::pg_test;` fix.

- [ ] loading without shared_preload_libraries fails clearly
  Trigger: `CREATE EXTENSION repmgr` (or `LOAD 'repmgr'`) when `repmgr` is not in
  `shared_preload_libraries`
  Expected: a clear error naming `shared_preload_libraries`, and no crash or half-initialised
  shared memory. Upstream returns NULL from every function in this case (for example `repmgr.c:223-224`);
  repmgr-rs's `_PG_init` errors instead, which is a deliberate divergence to confirm.
  Source: repmgr.c `_PG_init` (:118-145), repmgr-client.c:4543-4561 `check_shared_library`
  Status: implemented untested

- [ ] node ID range matches the config's ID range
  Trigger: set or return a node ID above 32767
  Expected: either accepted end to end, or rejected at config validation. Silent truncation must
  never happen. Upstream uses 32-bit `INT` (minimum 1, `MIN_NODE_ID`, repmgr.h:93), and
  `RepmgrNode.node_id` is `i16`.
  Source: repmgr--5.5.sql:5 (`node_id INTEGER`), configdata.c:40-48
  Status: not implemented (no setter yet). Open question: i16 on purpose, or i32 to match
  upstream? Your call.

### Deferred: needs the matching state or daemon to exist first

- [ ] upstream state is cleared on promotion
  Trigger: a standby that has recorded an upstream (ID + last-seen) is promoted without a
  Postgres restart; then query its upstream state
  Expected: no upstream / "never seen". Upstream repmgr leaves the old values in shared memory
  until a restart, and only hides them because callers wrap them in `pg_is_in_recovery()`.
  repmgr-rs should clear them at promotion instead of relying on every reader to check.
  Source: repmgr.c:182-202, :352-353; dbutils.c:5718-5727, :6064-6069
  Status: deferred until upstream tracking exists

- [ ] failover result reaches a sibling whose daemon is down
  Trigger: the election winner announces the new primary while a sibling's daemon is down but
  its Postgres is up; the daemon restarts afterwards
  Expected: the sibling ends up following the correct new primary
  Source: repmgrd-physical.c:3881 `notify_follow_primary` mailbox, :3893 polling
  Status: deferred until failover exists

- [ ] pause state survives restarts
  Trigger: pause, then restart the daemon, then restart Postgres
  Expected: still paused after each step (upstream persists it to `pg_stat/repmgrd_state.txt`)
  Source: repmgr.c:52, :229-275, :650-700
  Status: deferred until pause/control exists

- [ ] daemon-alive check isn't fooled by a stale or reused PID
  Trigger: the daemon crashes, and its PID gets reused by another process
  Expected: status reports "not running"
  Source: repmgr.c:620-647 (`kill(pid, 0)`), repmgr-client.c:4567
  Status: deferred until a daemon registry exists

## Cross-cutting

- [x] target version: PostgreSQL 18 only (corrected 2026-10-02; an earlier note said PG 19,
  which was wrong). Supporting older versions may come later, but it's out of current scope, so
  there's no version matrix to test today. Version incompatibility still broke real repmgr
  repeatedly, so keep version-specific assumptions isolated in the code, which makes adding
  versions later a contained change. The Docker harness can keep PG_VERSION/PG_VERSIONS as a
  knob, but its default and the version-check scenario above should track PG 18.
