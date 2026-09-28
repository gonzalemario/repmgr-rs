---
name: repmgr-source-lookup
description: Fast, factual lookup of "what does real repmgr 5.5.0 do here" against the C source at ~/repmgr-5.5.0, with file:line citations. Use for quick, narrow questions about repmgr's existing behavior — not for full subsystem reviews (use the repmgr-*-review skills for those).
---

# repmgr source lookup

This is the cheap, narrow counterpart to the `repmgr-*-review` skills. Use it when the question is a specific factual lookup ("what's the default for X", "does repmgr validate Y", "what function handles Z") rather than a full review of Mario's own code.

Source root: `~/repmgr-5.5.0/`. Read access there is scoped to `~/repmgr*` paths only — use `Read` or `Bash cat/grep` directly against that tree; do not attempt to read outside it.

## How to answer

1. Grep first, read second. Use `grep -n` for symbol/keyword hits across the tree before opening whole files — several files here are huge (`repmgr-action-standby.c` ~259KB, `repmgrd-physical.c` ~164KB, `dbutils.c` ~144KB) and reading them in full wastes context for a narrow question.
2. Useful anchors already known from prior investigation (reuse before re-deriving):
   - Config grammar + schema table: `configfile-scan.l`, `configdata.c`, `configfile.c` (`parse_configuration_item`, `CONF_scanstr`)
   - Defaults and limits: `repmgr.h` (`DEFAULT_*` macros), `configfile.h` (`t_configuration_options` struct)
   - Cluster registry schema: `repmgr--5.5.sql` (always read the latest-versioned `repmgr--X.Y.sql`, not the incremental `--A--B.sql` migration diffs, for current shape)
   - Failover/election engine: `repmgrd-physical.c` (search function names: `do_election`, `do_primary_failover`, `do_upstream_standby_failover`, `promote_self`, `follow_new_primary`, `check_primary_child_nodes`), `voting.h`
   - Node identity independent of registry: `controldata.c` (binary `pg_control` reader)
   - Historical bugs/behavior rationale: `HISTORY` — grep by keyword, cite the line and version it landed in
   - CLI subcommand structure: `repmgr-action-{cluster,node,primary,standby,witness,service,daemon}.c`, dispatched from `repmgr-client.c`
3. Always answer with a direct file:line citation, quoting the minimal relevant snippet — not a paraphrase you can't point back to.
4. If the honest answer is "this needs the full subsystem review, not a quick lookup," say so and point at the matching `repmgr-*-review` skill instead of giving a shallow answer to a deep question.
5. This skill never touches `/home/mario/rust/repmgr-rs` source — it's a one-directional lookup against the reference implementation only.
