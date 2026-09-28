---
name: repmgr-topology-review
description: Review repmgr-rs's node registry / cluster topology data model against real repmgr's schema and node-identity decisions. Use when the user is working on the node registry, upstream/topology relationships, node identity, or cluster state representation in repmgr-rs.
---

# repmgr topology review

You are reviewing **Mario's own Rust code** (he writes it; you review it — never write or edit his implementation files unless he explicitly asks). repmgr-rs is an **independent design**, not schema-compatible with real repmgr — the goal here is informed divergence, not replication. Ground findings in file:line references on both sides.

## Ground truth: how real repmgr represents cluster topology

Source: `~/repmgr-5.5.0/repmgr--5.5.sql`, `dbutils.c`, `dbutils.h`, `controldata.c`.

- **Topology lives inside Postgres itself**, as tables installed by `CREATE EXTENSION repmgr` on the primary: `repmgr.nodes` (node_id PK, upstream_node_id self-FK, active bool, node_name, type CHECK IN ('primary','standby','witness','bdr'), location, priority, conninfo, repluser, slot_name, config_file), `repmgr.events` (append-only audit log: node_id, event, successful, timestamp, details), `repmgr.monitoring_history` (per-standby lag samples: LSNs, replication_lag, apply_lag), `repmgr.voting_term` (a single counter row guarding election idempotency). There is deliberately **no separate coordination service** — the primary's own catalog is the single source of truth.
- **This is also the root of most topology bugs**: when the primary is unreachable, every standby's view of `repmgr.nodes` is whatever it last streamed — potentially stale, and *not itself replicated consistently during a partition*. Any topology model that assumes "the registry reflects live reality" needs an explicit staleness/last-seen concept.
- `upstream_node_id` makes the topology a **tree, not a flat primary+standbys list** — cascading replication (standby-of-standby) is a first-class case, not an edge case.
- `type` is a closed enum including `witness` — a node that holds a registry entry and votes but never has a physical replica. Any code that assumes "every node has a data directory / replication state" will break on witness nodes (this was in fact repmgr's single most-patched area per HISTORY — witness handling).
- `active` is a soft-delete/exclusion flag, not physical presence — a node can exist in the registry but be excluded from election/consensus.
- `controldata.c` reads the **binary `pg_control` file directly**, without a running Postgres, to get `system_identifier` and current timeline — used to verify a node's identity/lineage independent of what the registry *claims*. This is repmgr's answer to "is this actually the node I think it is, or a stale/forked clone?" — a node-identity check that doesn't trust the registry alone.

## What to check in his code

1. Does the topology model support cascading (tree) replication, or does it assume one primary + flat standby list? If flat is intentional for now, that's fine — just confirm it's a conscious scope cut, not an oversight.
2. Is there any notion of "how stale is this topology view" (last successful refresh time, last-seen-primary timestamp), or is the registry treated as always-current?
3. Does the witness node type get modeled as a genuinely different kind of node (no data directory, no replication lag), or is it bolted onto the same struct as physical standbys with fields that don't apply?
4. Is node identity ever cross-checked against something outside the registry itself (e.g. a `pg_control`-equivalent read), or is the registry the only source of truth for "which node is this"? Flag this as a design question, not a required feature yet.
5. `active`/soft-exclusion: does the model distinguish "not currently reachable" from "administratively excluded"?

Deliver findings as: what real repmgr's schema encodes, what his data model currently encodes, and the concrete cluster scenario (e.g. cascading standby, witness-only node, partitioned primary) where the gap would surface. Since this is an independent design, prefer questions ("did you mean to scope this to single-level topology for now?") over "you're missing X" — divergence from repmgr may be the right call.
