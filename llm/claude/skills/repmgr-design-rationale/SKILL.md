---
name: repmgr-design-rationale
description: Investigate why upstream repmgr made a specific architectural or implementation decision, and assess whether the underlying constraint that forced it actually applies to repmgr-rs's independent design. Use when Mario asks "why does repmgr do X", or before repmgr-rs builds (or deliberately skips) a mechanism that upstream repmgr has, to check whether the reason for it is structural or just inherited from upstream's specific situation.
---

# repmgr design-rationale investigation

This is a different job from the `repmgr-*-review` skills: those compare Mario's *existing* Rust
code against repmgr's C source, file:line to file:line. This one usually runs **before there's
any repmgr-rs code for the subsystem at all** — the deliverable is understanding a decision well
enough to say whether repmgr-rs needs an equivalent, a different mechanism for the same underlying
need, or nothing.

## Method

1. **Find the actual mechanism, don't infer it from a name or from documentation prose.** Grep
   and read the real C — struct definitions, the functions that touch them, the SQL they're
   exposed through (`repmgr--5.5.sql`'s function signatures, in particular volatility/strictness
   markers, are often more informative than the C comments). A past investigation in this
   project found the "shared memory instead of a table" decision (`repmgr.c:64-220`) only makes
   sense once you check that its SQL functions are plain `LANGUAGE C STRICT` with no restriction
   to non-recovery sessions — that's the tell, not something stated outright anywhere.

2. **Trace the actual forcing function.** Distinguish three categories, explicitly:
   - **Structurally forced** — an OS-level or Postgres-level rule leaves no alternative (e.g.
     hot-standby's blanket rejection of table writes, or `PG_MODULE_MAGIC`/backend-only headers
     meaning a Postgres extension can't be part of a general CLI binary).
   - **Historical/organic** — grew that way because of project history, not because it has to be
     that way (e.g. `repmgr` and `repmgrd` being separate binaries is largely SysV-daemon
     convention plus repmgrd being added as a bolt-on feature later; nothing forces that split).
   - **Deliberate tradeoff** — a real choice was available and they picked one for stated (or
     inferable) reasons; name the alternative that was passed up and why.

3. **Ask whether the forcing function actually applies to repmgr-rs's situation**, not "would we
   need this if we copied their architecture." Known standing facts about repmgr-rs to reason
   from (check [[repmgr_rs_architecture]] and [[repmgr_rs_priority_failure_modes]] for the
   current state of these, they can change):
   - independent design, not wire/schema-compatible with real repmgr
   - Mario writes all repmgr-rs `src/` code himself; this skill investigates and recommends, it
     never writes implementation code
   - no equivalent of a loaded Postgres C extension planned — daemon-side state is an ordinary
     process, not code embedded in the Postgres backend, so backend-only constraints (hot-standby
     write rules, `shared_preload_libraries`, `PG_MODULE_MAGIC`) may simply not apply
   - built on tokio + tokio-postgres (async I/O available for free, where upstream hand-rolls
     busy-wait loops over blocking libpq)
   - targets PostgreSQL 19 only (confirmed 2026-09-25) — no multi-version compatibility matrix
     to design around today

4. **Give a plain verdict**, not just an explanation: does repmgr-rs need an equivalent
   mechanism, a different mechanism serving the same underlying need, or can the whole problem be
   dropped because the constraint that created it doesn't exist in this architecture? Say which,
   and why, in the last paragraph — don't leave it as an open question when the source evidence
   actually answers it.

## Whose "why" to ask for

Mario joined repmgr's upstream team after its core architecture (config parsing, registry-in-
Postgres, election/voting design) was already set — he's not the original architect of those, so
don't ask him to justify decisions that predate him; investigate those from source. He *does* have
first-hand operational experience living with these decisions and authored the pgbackupapi
integration himself. Prefer asking him for real-world texture ("did this actually bite in
practice") over guessing, but don't assume he has original-design intent to offer on everything.

## Output shape

Structured explanation, grounded in file:line citations on the upstream side, ending with the
verdict from step 4. No Rust code. If the investigation surfaces a concrete scenario worth
tracking, name it as a candidate for `repmgr-verification-planner`'s `VERIFICATION.md` rather than
leaving it implicit.
