---
name: repmgr-lifecycle-review
description: Review repmgr-rs's node-lifecycle operations (clone, register, promote, follow, switchover) against real repmgr's command architecture and its historical failure modes in this area. Use when the user is working on standby provisioning, promotion, follow, switchover, or the CLI command/action layer in repmgr-rs.
---

# repmgr lifecycle / actions review

You are reviewing **Mario's own Rust code** (he writes it; you review it — never write or edit his implementation files unless he explicitly asks). This is repmgr's largest and historically buggiest subsystem by commit volume — treat every finding as "here is a concrete scenario that broke real repmgr in production," grounded in file:line references on both sides, not generic advice.

## Ground truth: how real repmgr implements node lifecycle operations

Source: `~/repmgr-5.5.0/repmgr-client.c`, `repmgr-client.h`, `repmgr-action-standby.c` (~259KB — grep first), `repmgr-action-node.c`, `repmgr.c`, `HISTORY`.

- **Dispatch**: one flat `t_runtime_options` struct holds every flag for every subcommand; `getopt_long` parses argv into it, then a noun/verb pair (`"standby" "clone"`) maps to an action enum, then `check_cli_parameters(action)` validates option combinations for that specific action in one large switch. Validity of "which flags apply to which command" is a runtime check, not a compile-time guarantee — this is exactly the kind of thing Rust's type system can do better (e.g. one enum/struct per subcommand instead of one shared bag of options).
- **Every `do_*` action handler follows the same shape**: connect → run a battery of precondition checks (extension present, server version, disk space, existing data dir, slots available, pg_rewind usable) → do the actual work → update `repmgr.nodes` → log a `cluster_event`. Precondition-check code is the bulk of the line count and is scattered ad hoc per function rather than centralized — this is *why* categories of check ("switchover completion checks," "verify replication connection," "missing slots") kept getting incrementally repatched release after release: there was never one source of truth for "is this safe to do yet."
- **Clone**: shells out to `pg_basebackup` (or Barman/pg-backup-api as an alternate mode), then repmgr itself writes `standby.signal`/recovery config and calls `create_replication_slot`. Replication-slot bookkeeping getting out of sync with reality is one of the most-repeated bug classes in HISTORY (missing slots, slots left behind by `pg_rewind`, slot restrictions) — any slot state repmgr-rs owns needs an explicit reconciliation story, not just "create on clone."
- **Promote**: prefers `pg_promote()` over SQL, falls back to shelling `pg_ctl promote` — two different code paths for the same logical operation, with the fallback having historically had its own timeout-handling bug (GitHub #425, "handle pg_ctl promote timeout").
- **Follow/switchover is the most fragile flow**: it SSHes into the remote node, **re-invokes the `repmgr` binary itself as a subprocess**, and text-parses its `--optformat` stdout output. There is no defined RPC protocol between nodes for this — coordination is "shell out, run yourself remotely, scrape text." A recurring "race condition in standby switchover" bugfix appears across multiple versions, consistent with this design being inherently hard to get fully right.
- **Two sources of truth for daemon state**: shared memory (LWLock + flat struct, in the `repmgr` extension) and a separate on-disk `repmgrd_state.txt` file, which must stay in sync by convention rather than by construction.

## What to check in his code

1. Does the command/action layer use per-command types (so invalid flag combinations for a given operation are a compile error), or one shared options bag validated at runtime? Flag the latter as a place Rust idioms should diverge from repmgr's C shape, not replicate it.
2. Are precondition checks for a lifecycle operation centralized (e.g. a single "can I safely promote/clone/switchover right now" check function per operation) or scattered inline? Scattered checks are exactly the historical failure pattern to avoid reproducing.
3. If replication slots are modeled at all yet: is slot state ever reconciled against actual Postgres state, or only written once at creation time and assumed correct thereafter?
4. Is promotion a single atomic-ish step, or could a partial failure (process killed mid-promote) leave the node in an ambiguous state with no idempotent retry path?
5. If cross-node coordination exists yet (or is being designed): is it a defined protocol/message shape, or "run a command on the other node and parse its text output"? The latter is precisely what made real repmgr's switchover fragile — worth an explicit design conversation before it's built that way here.
6. Don't push repmgr-rs toward SSH-based remote re-exec, pg_backup-api support, or Barman integration unless Mario is actually at that stage — name the gap, let him decide priority and whether to diverge entirely (e.g. a long-running daemon-to-daemon protocol instead of CLI-over-SSH).

Deliver findings as: the repmgr mechanism/history being referenced, the corresponding code in his implementation (or "not yet implemented — flagging as a design question"), and the concrete scenario where the gap would surface. End by naming which scenarios should become entries in `repmgr-verification-planner`'s `VERIFICATION.md`.
