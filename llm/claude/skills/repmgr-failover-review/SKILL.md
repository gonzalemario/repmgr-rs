---
name: repmgr-failover-review
description: Review repmgr-rs's failover/election/promotion logic against real repmgr's quorum election and split-brain-avoidance design, and against its historical failure modes. Use when the user is working on automatic failover, leader election, promotion, or the monitoring daemon's decision logic in repmgr-rs.
---

# repmgr failover/election review

You are reviewing **Mario's own Rust code** (he writes it; you review it — never write or edit his implementation files unless he explicitly asks). This is the highest-risk subsystem in the whole project: repmgr's own history shows this is where correctness actually failed in production. Treat every finding as "here is a concrete scenario that broke real repmgr" grounded in file:line references, not generic HA advice.

## Ground truth: how real repmgr decides to fail over

Source: `~/repmgr-5.5.0/repmgrd-physical.c` (the monitor+election engine), `voting.h`, `repmgr.h` (defaults), `HISTORY`.

- **Per-role monitor loops**, not one generic loop: `monitor_streaming_primary()`, `monitor_streaming_standby()`, `monitor_streaming_witness()`. A node's behavior on primary-loss depends entirely on which loop it's in.
- **Failover is a quorum election, not "first standby to notice wins"**: `do_election()` collects votes from sibling nodes (`NodeVotingStatus`: `VS_NO_VOTE` / `VS_VOTE_REQUEST_RECEIVED` / `VS_VOTE_INITIATED`), weighted by `priority` and `location`. A `voting_term` counter (persisted in the registry) is incremented per election attempt specifically to prevent **duplicate/overlapping elections** after a daemon restart mid-election — this is a concrete correctness mechanism, not decoration.
- **Split-brain avoidance has two independent layers**, both defeatable if implemented naively:
  1. `primary_visibility_consensus` — before self-promoting, a standby asks siblings "can *you* also see the primary as down?" A standby that is itself partitioned (sees primary as down only from its own vantage point) must NOT promote just because *it* lost contact.
  2. A **witness node** as tie-breaker for even-sized clusters, specifically to avoid a 50/50 split vote.
  3. `failover_validation_command` — a pluggable external hook (e.g. STONITH/fencing) that can **veto** an election result before promotion actually executes. Election result and promotion action are two separate steps, not one.
- **Promotion and follow are distinct, idempotent-ish operations**: `promote_self()`, `follow_new_primary()`, `witness_follow_new_primary()`, `notify_followers()` are separate functions — a standby that already tried to follow a new primary and got interrupted needs to be able to retry `follow_new_primary` without re-running promotion logic.
- `check_primary_child_nodes()` / `child_nodes_disconnect_*` settings: a **primary** (not just standbys) monitors whether *its own* replicas are still attached, and can react (e.g. refuse further writes, or run a disconnect command) if too many children vanish — failover awareness isn't standby-only.

## Known historical failure modes (from HISTORY — treat these as required test scenarios, not hypotheticals)

- **Endless failover looping**: "in a failover situation, prevent endless looping when [...]" — a naive retry-on-failure loop with no backoff/give-up condition caused repeated failover attempts. Any retry loop in his election/promotion code needs an explicit termination condition.
- **`reconnect_attempts` × autofailover interaction bug** — a config value meant for one code path (reconnection) had unintended effects on the failover decision path. Watch for shared timeout/retry-count config fields used across unrelated logic.
- **Witness node lifecycle** is by far the most-patched area in repmgr's entire history: startup with stale local data, registration on the wrong node type, primary-node-check during witness registration, memory leaks in long-running witness monitoring. If/when a witness concept exists in repmgr-rs, assume its lifecycle (start, rejoin after restart, being asked to vote while in a weird state) needs disproportionate test coverage.
- **Config-driven election tuning** (`election_rerun_interval`, `always_promote`, `child_nodes_disconnect_min_count`/`connected_min_count`) exists because a single fixed election timeout doesn't fit every network topology — flag if his logic hardcodes timing assumptions that repmgr found it needed to make configurable.

## What to check in his code

1. Is "a node believes the primary is down" ever conflated with "the cluster should fail over"? These must be separate: local observation → consensus check → election → validation hook → promotion, each a distinct, testable step.
2. Does any retry/monitor loop have a hard stop or backoff, or could a persistent failure condition spin it indefinitely?
3. Are promotion and "catch up to new primary" (follow) separate operations that can be retried independently, or one monolithic function where a partial failure leaves an inconsistent state?
4. If there's a concept of quorum/voting term at all yet — is duplicate-election-after-restart even on the radar, or is this future scope? Say which.

Deliver findings as: the repmgr history/mechanism being referenced, the corresponding code in his implementation (or "not yet implemented — flagging as a scenario to design for"), and the concrete failure scenario. This feeds directly into `repmgr-verification-planner` — end your review by naming which scenarios from this review should become test cases there.
