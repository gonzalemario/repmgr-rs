# repmgr extension SQL functions — reference

Inventory of every SQL function created by `CREATE EXTENSION repmgr` in repmgr 5.5.0, what
backs it, who calls it, and what coordination problem it solves. Source root: `~/repmgr-5.5.0/`.

Scope: functions only. Tables/views (`nodes`, `events`, `monitoring_history`, `voting_term`,
`show_nodes`, `replication_status`) are covered by `repmgr-topology-review`.

## Quick list

**Node identity**
1. `set_local_node_id(int)`: stores this node's ID. Only the first call takes effect. It also
   restores the saved pause state from disk.
2. `get_local_node_id()`: returns this node's ID. It returns NULL when the library isn't
   preloaded, which is how the CLI checks that repmgr is loaded.

**Upstream and liveness**

3. `set_upstream_node_id(int)`: stores which node this node replicates from. It errors if you
   pass the node's own ID.
4. `get_upstream_node_id()`: returns the stored upstream node ID.
5. `set_upstream_last_seen(int)`: records that the upstream was seen just now, and stores its ID.
6. `get_upstream_last_seen()`: returns the seconds since the upstream was last seen, or -1 if it
   never was.
7. `standby_set_last_updated()`: stores the current time as when this standby last sent
   monitoring data.
8. `standby_get_last_updated()`: returns that time. Only the `replication_status` view uses it.

**Failover signalling**

9. `notify_follow_primary(int)`: tells a node to follow a new primary. The election winner calls
   it on each other node's database. Passing -2 means "rerun the election".
10. `get_new_primary()`: returns the new primary's ID if a notification has arrived, otherwise -1.
    Each repmgrd checks it about once a second.
11. `reset_voting_status()`: clears the vote state and any pending notification.

**repmgrd control**

12. `set_repmgrd_pid(int, text)`: stores repmgrd's process ID and pidfile path. Passing NULL
    clears them.
13. `get_repmgrd_pid()`: returns the stored repmgrd process ID.
14. `get_repmgrd_pidfile()`: returns the stored pidfile path. repmgr itself never calls it.
15. `repmgrd_is_running()`: checks whether the stored repmgrd process is still alive.
16. `repmgrd_pause(bool)`: pauses or unpauses repmgrd and saves that state to
    `pg_stat/repmgrd_state.txt`, so it survives a restart.
17. `repmgrd_is_paused()`: returns whether repmgrd is paused.

**Misc**

18. `get_wal_receiver_pid()`: returns the process ID of the node's WAL receiver, which is 0 or
    less when it isn't running. During failover, repmgrd uses it to check whether other standbys
    are still connected to the primary.

### Node ID values

The ID behind `set_local_node_id` / `get_local_node_id` is the `node_id` from `repmgr.conf`: a
32-bit integer, 1 or more (`MIN_NODE_ID`, `repmgr.h:93`), unique across the cluster because it is
the primary key of `repmgr.nodes`. It has no usable default: the config default is `-1`
(`UNKNOWN_NODE_ID`, `configdata.c:45`), and a missing `node_id` fails validation
(`configfile.c:386-388`). What `get_local_node_id()` returns:

- `NULL`: the library isn't in `shared_preload_libraries`.
- `-1`: the library is loaded, but repmgrd hasn't called `set_local_node_id()` yet
  (`repmgr.c:194`).
- `1` or more: the real node ID.

## The mechanism behind all 18

- Declared in `repmgr--5.5.sql:77-167`, all `LANGUAGE C`, all `STRICT` except `set_repmgrd_pid`
  (`CALLED ON NULL INPUT`, `:144-147`). No `REVOKE`/`GRANT`/`SECURITY DEFINER` anywhere in the
  script — any role that can connect can call any of them, including the setters.
- Implemented in `repmgr.c` (`PG_FUNCTION_INFO_V1` list at `:95-112`). Every function except
  `get_wal_receiver_pid` reads or writes a single shared-memory struct, `repmgrdSharedState`
  (`repmgr.c:64-79`), guarded by one named LWLock (tranche `"repmgrd"`).
- Shared memory only exists if `repmgr` is in `shared_preload_libraries` (`_PG_init`,
  `repmgr.c:118-145`). Without it `shared_state == NULL` and the functions return NULL (or
  `-1` / `UNKNOWN_NODE_ID` for a few).
- The struct is initialised in `repmgr_shmem_startup` (`repmgr.c:167-210`): node ids `-1`,
  pid `-1`, `upstream_last_seen = POSTGRES_EPOCH_JDATE` (a magic "never set" value),
  `voting_status = VS_NO_VOTE`.
- **Lifetime:** the state survives a repmgrd restart but is lost on a Postgres restart. The one
  exception is the pause flag, which is also written to `pg_stat/repmgrd_state.txt` as
  `"<node_id>:<0|1>"` (`repmgr.c:52`, `:650-700`) and read back on the next `set_local_node_id`
  (`:229-275`).
- **Why shared memory and not a table:** these functions do no table writes, so they work on a
  hot standby. A table-backed design could not record state on standbys.
- All C-side callers go through thin wrappers in `dbutils.c`. The wrapper names are listed below.

## Function inventory

Columns: SQL signature → C impl (`repmgr.c`) → shared-state field → `dbutils.c` wrapper → real
callers.

### Node identity

| SQL | C impl | Field | Wrapper | Callers |
|---|---|---|---|---|
| `set_local_node_id(INT) → void` | `:217` | `local_node_id` (set **once only**, first call wins), also restores `repmgrd_paused` from state file | `repmgrd_set_local_node_id` `:2131` | repmgrd on startup and every reconnect: `repmgrd.c:490`, `repmgrd-physical.c:534,2229,2780,3525,4109,4945` |
| `get_local_node_id() → int` | `:287` | `local_node_id` | `repmgrd_get_local_node_id` `:2160`; `repmgrd_check_local_node_id` `:2185` | repmgrd reconnect paths (compares stored id vs config); **CLI `check_shared_library`** `repmgr-client.c:4552` uses NULL-ness as the "is the library preloaded?" probe |

Notes:
- `set_local_node_id` ignores later calls once set. A node id change needs a Postgres restart.
- `check_shared_library` exists because parsing `shared_preload_libraries` is unreliable and may
  not be readable by the repmgr user (`repmgr-client.c:4543-4550`).

### Upstream tracking / liveness

| SQL | C impl | Field | Wrapper | Callers |
|---|---|---|---|---|
| `set_upstream_last_seen(INT) → void` | `:338` | `upstream_last_seen = now()` **and** `upstream_node_id` | `set_upstream_last_seen` `:6026` | repmgrd monitor loop on each successful upstream check: `repmgrd-physical.c:1566,2474` |
| `get_upstream_last_seen() → int` | `:361` | seconds since `upstream_last_seen`; `-1` if never set | `get_upstream_last_seen` `:6050`; inline in `get_replication_info` `:5714-5728` | `repmgr service status` across all nodes (`repmgr-action-service.c:213`); `get_replication_info` callers in repmgrd and `standby`/`node` actions |
| `get_upstream_node_id() → int` | `:395` | `upstream_node_id` | `repmgrd_get_upstream_node_id` `:2391`; inline in `get_replication_info` | primary-side repmgrd asks the **witness** who it thinks its upstream is: `repmgrd-physical.c:5329` |
| `set_upstream_node_id(INT) → void` | `:410` | `upstream_node_id`; errors if equal to `local_node_id` | `repmgrd_set_upstream_node_id` `:2416` | primary repmgrd sets `NO_UPSTREAM_NODE`: `repmgrd-physical.c:356` |
| `standby_set_last_updated() → timestamptz` | `:304` | `last_updated = now()` | inline in `add_monitoring_record` `dbutils.c:5180` | repmgrd, right after it async-sends a `monitoring_history` insert to the primary |
| `standby_get_last_updated() → timestamptz` | `:321` | `last_updated` | none in C | only used in the `repmgr.replication_status` view (`repmgr--5.5.sql:187`) for `communication_time_lag` when queried on a standby |

Notes:
- `get_replication_info` (`dbutils.c:5657`) masks both upstream functions to `-1` when the node is
  not in recovery, except on a witness, which always reports them.
- `get_upstream_last_seen` returns elapsed seconds as `int` and truncates `secs` through a
  `uint32` cast (`repmgr.c:388-391`).

### Failover signalling (inter-node message passing)

| SQL | C impl | Field | Wrapper | Callers |
|---|---|---|---|---|
| `notify_follow_primary(INT) → void` | `:446` | `candidate_node_id = arg`, `follow_new_primary = true`; no-op if `local_node_id` unset | `notify_follow_primary` `:5389` | **called on a remote node's connection**: the election winner tells each sibling to follow it (`repmgrd-physical.c:3881`); manual `standby promote` tells the witness (`repmgr-action-standby.c:8740`). Arg `-2` (`ELECTION_RERUN_NOTIFICATION`) means "rerun the election" |
| `get_new_primary() → int` | `:489` | `candidate_node_id` if `follow_new_primary`, else `-1` | `get_new_primary` `:5417` | local repmgrd polls its own node once per second: `wait_primary_notification` `repmgrd-physical.c:3893` (bounded by `primary_notification_timeout`) and inside the reconnect sleep loop `:5599` |
| `reset_voting_status() → void` | `:511` | `voting_status = VS_NO_VOTE`, `candidate_node_id = -1`, `follow_new_primary = false` | `reset_voting_status` `:5457` | `reset_node_voting_status` `repmgrd-physical.c:4889`, called at monitor start and after each failover outcome (`:355,1368,2338,4734,4755,4778`) |

Notes:
- This is the important part. The extension works as a **mailbox**: the winner writes into each
  sibling's shared memory over an ordinary libpq connection, and each sibling's repmgrd reads its
  own mailbox. Nothing is pushed to the daemon itself. The receiver only notices the message
  through polling, which adds up to one poll interval of latency.
- `voting_status` and `current_electoral_term` live in the struct, but only
  `reset_voting_status` touches them through SQL. Nothing sets them, and
  `current_electoral_term` is never read. The real term counter is the `repmgr.voting_term` table.
- The mailbox only works if the sibling's Postgres is up **and** repmgr is preloaded there. If the
  sibling's repmgrd is down, the message sits until reset. The witness note at
  `repmgr-action-standby.c:8730-8735` says this outright: "if repmgrd is not running … this will have
  no effect."

### Daemon registry / control

| SQL | C impl | Field | Wrapper | Callers |
|---|---|---|---|---|
| `set_repmgrd_pid(INT, TEXT) → void` | `:579` | `repmgrd_pid`, `repmgrd_pidfile` (NULL pid → `-1`, clears the pidfile) | `repmgrd_set_pid` `:2249` | repmgrd at startup/reconnect (`repmgrd.c:528`, `repmgrd-physical.c:535,…`); clears it on shutdown (`repmgrd.c:1078`) |
| `get_repmgrd_pid() → int` | `:540` | `repmgrd_pid` | `repmgrd_get_pid` `:2290` | `repmgr daemon start/stop` (`repmgr-action-daemon.c:72,207`), `service status` (`repmgr-action-service.c:175`), `standby switchover` (`repmgr-action-standby.c:4836`), repmgrd checking siblings (`repmgrd-physical.c:4509`); `is_repmgrd_running` (`repmgr-client.c:4567`) then calls `kill(pid,0)` **from the client host** |
| `get_repmgrd_pidfile() → text` | `:559` | `repmgrd_pidfile` | none in C | unused by repmgr itself; exposed for operators |
| `repmgrd_is_running() → bool` | `:620` | `kill(repmgrd_pid, 0)` **inside the Postgres backend** | `repmgrd_is_running` `:2314`; inline in `get_repmgrd_status` `:6152` | `service status`, `switchover`, `node check --repmgrd` (`repmgr-action-node.c:2166`) |
| `repmgrd_pause(BOOL) → void` | `:650` | `repmgrd_paused`, persisted to `pg_stat/repmgrd_state.txt` | `repmgrd_pause` `:2362` | `repmgr service pause/unpause` (`repmgr-action-service.c:450`); `switchover` pauses all daemons, then unpauses them (`repmgr-action-standby.c:4951,5702`) |
| `repmgrd_is_paused() → bool` | `:703` | `repmgrd_paused` | `repmgrd_is_paused` `:2338` | repmgrd checks before acting on failure (`repmgrd-physical.c:1133,1732,1967,2062`); `service status`, `switchover` |

Notes:
- `repmgrd_is_running` uses `kill()` from the backend, so it only works because repmgrd runs on
  the same host as its Postgres. `is_repmgrd_running` in `repmgr-client.c` does a second
  `kill()` from wherever the **CLI** runs, which is only right when the CLI is on the same host too.
- PID reuse is not handled. A stale pid that now belongs to another process reports "running".
- Pause is the only state that survives a Postgres restart.

### Misc

| SQL | C impl | Field | Wrapper | Callers |
|---|---|---|---|---|
| `get_wal_receiver_pid() → int` | `:719` | reads `WalRcv->pid` directly; not repmgr state | `get_wal_receiver_pid` `dbutils.c:1827` | repmgrd, during failover, checks whether **sibling** standbys still have a live WAL receiver before declaring the primary dead (`repmgrd-physical.c:2933`, `sibling_nodes_disconnect_timeout`) |

Note: it still returns NULL when `shared_state` is NULL (`:724`), even though it never reads the
shared state. It needs the library loaded only for that reason.

## What these functions do between them

1. **Daemon state on a standby.** It's readable over SQL by the CLI and by other nodes' daemons,
   even though standbys can't take table writes (identity, upstream, last-seen, pid).
2. **A mailbox between nodes for failover results** (`notify_follow_primary` → `get_new_primary`).
   This is the only channel the election winner uses to tell the other nodes. There's no direct
   daemon-to-daemon connection.
3. **CLI → daemon control** without IPC to the daemon process (pause/unpause, running checks).
   The CLI only ever talks to Postgres.
4. **A capability probe** ("is repmgr preloaded here?") via a NULL return.
5. **Backend internals** (`WalRcv->pid`) without needing the stats views.

## Implications for repmgr-rs (no Postgres extension planned)

All five jobs come from one fact: Postgres is the only endpoint every repmgr component can reach.
If repmgr-rs daemons have their own listener (gRPC/HTTP/whatever), most of this goes away.

| Need | Upstream mechanism | repmgr-rs options |
|---|---|---|
| Per-node daemon state readable by others | shmem via SQL | the daemon serves its own state directly; nothing needs to live inside Postgres |
| Failover result propagation | `notify_follow_primary` mailbox + 1 s polling | direct daemon-to-daemon message (push, ack-able). **Open design question:** upstream's mailbox still delivers if the sibling *daemon* is down but its Postgres is up. Direct messaging loses that, so a restarted daemon must derive the new primary itself |
| CLI → daemon pause / status | shmem + `pg_stat/repmgrd_state.txt` | a control socket or API on the daemon. Pause must still survive restarts (upstream persists it). Decide where it's stored and whether it should survive a *Postgres* restart, a daemon restart, or both |
| "Is daemon alive" | pid + `kill(pid,0)` on the same host | a daemon health endpoint. This also avoids upstream's PID-reuse and same-host assumptions |
| Last-seen-upstream / staleness | `upstream_last_seen` in shmem | held in daemon memory and reported over its API. Ties into the topology-staleness question in `repmgr-topology-review` |
| Sibling WAL-receiver check during failover | `get_wal_receiver_pid()` per sibling | `pg_stat_wal_receiver` over SQL (check which columns are visible to a non-superuser role on PG 19), or ask the sibling daemon |
| Library-loaded probe | NULL from `get_local_node_id` | not needed |

### Verification candidates (for `VERIFICATION.md`)

- Winner announces the new primary while a sibling's daemon is down but its Postgres is up. The
  sibling's daemon restarts after the announcement. Does it end up following the right primary?
- Pause, then restart the daemon, then restart Postgres. Is it still paused after each step?
- A stale pid or pid reuse after a daemon crash: does the status check report "running" falsely?
- Sibling-WAL-receiver check when a sibling is reachable only over SQL but its daemon is down.
