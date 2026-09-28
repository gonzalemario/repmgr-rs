---
name: repmgr-verification-planner
description: Maintain a living checklist of concrete test/verification scenarios for repmgr-rs, derived from real repmgr's historical failure modes and from gaps surfaced by the other repmgr-*-review skills. Use when the user wants to plan what to test/verify next, after implementing or reviewing a subsystem.
---

# repmgr verification planning

Purpose: turn "repmgr has a legacy of not working in some cases" into a concrete, prioritized, living list of scenarios repmgr-rs should be able to survive — and track which are covered. This is planning, not implementation: you do not write test code here unless asked; you name scenarios precisely enough that Mario (or a later session) can write the test.

## Where scenarios come from

1. **The historical failure catalog** (grep `~/repmgr-5.5.0/HISTORY` for patterns like `fix|bug|crash|hang|prevent|race` combined with `failover|witness|election|split.brain|timeline|config|pars`). Each fixed historical bug is evidence of a scenario real operators hit in production — treat the HISTORY file as a scenario source, not just changelog trivia.
2. **Findings from `repmgr-config-review`, `repmgr-topology-review`, `repmgr-failover-review`** — each of those skills should end its review by naming candidate scenarios; pull those in here rather than re-deriving them.
3. **Direct source reading** when a specific mechanism needs a scenario derived from how it actually works (e.g. read `do_election()` in `repmgrd-physical.c` to write a precise "split vote with an even number of siblings and no witness" scenario, rather than a vague "test elections").

## Maintain `VERIFICATION.md` at the repo root

Keep one living file, grouped by subsystem, each scenario as:

```
- [ ] <short scenario name>
  Trigger: <exact condition/inputs>
  Expected: <what correct behavior looks like>
  Source: <HISTORY line / repmgr C function / review finding this came from>
  Status: not implemented / implemented untested / covered by <test name>
```

Check off (`[x]`) only when Mario confirms a scenario is actually covered by a real test — never mark done based on inference from reading code.

## Session behavior

- When invoked, first read the current `VERIFICATION.md` (create it if absent) and the current state of `src/` to know what's actually implemented so far — don't propose scenarios for subsystems that don't exist yet; instead note them as "deferred until X exists."
- Prioritize ruthlessly: surface the 3-5 highest-value untested scenarios given current implementation state, not the full catalog at once. Explain *why* each one is high value (usually: "this is exactly the shape of bug that broke real repmgr in production, per HISTORY").
- When a new subsystem lands, ask whether to run the matching `repmgr-*-review` skill first — its findings are the richest scenario source.
- Never implement fixes or write test code from this skill without being asked; your output is the checklist plus a short rationale, not a patch.
