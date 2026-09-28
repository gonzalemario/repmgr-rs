---
name: repmgr-config-review
description: Review repmgr-rs's repmgr.conf parsing/validation code against real repmgr's config grammar and schema-driven validation model. Use when the user is working on config file parsing, the settings data model, or config validation/error reporting in repmgr-rs.
---

# repmgr config review

You are reviewing **Mario's own Rust code** in repmgr-rs (he writes it; you review it — never write or edit his implementation files in this skill unless he explicitly asks you to). Ground every finding in a concrete file:line reference on both sides: his Rust file and the corresponding repmgr C source.

## Ground truth: how real repmgr parses config

Source: `~/repmgr-5.5.0/configfile.c`, `configfile.h`, `configfile-scan.l`, `configdata.c`, `repmgr.conf.sample`.

- **The grammar is deliberately identical to PostgreSQL's own GUC config parser** (`postgresql.conf` syntax), not a custom format. Read `configfile-scan.l` — it's a flex lexer with these token rules:
  - Bare identifiers (`ID`, `UNQUOTED_STRING`): letters/digits/`-._:/ ` unquoted are valid for simple values.
  - Quoted strings: `'...'` with **backslash escapes** (`\n \t \r \b \f`, octal `\0`-`\7` sequences) and **doubled single-quote** (`''` → `'`) as an alternative escape — both must be collapsed (see `CONF_scanstr`).
  - `#` starts a comment to end-of-line, **including trailing comments after a value** on the same line — HISTORY records this was once broken and fixed (search "ignore comments after values").
  - Whitespace/blank lines are eaten; whitespace-handling itself was a historical bug class (HISTORY: "Fixes to whitespace handling when parsing config file").
  - `include`, `include_if_exists`, `include_dir` directives, each guarded against recursion (depth > 10 rejected, direct self-inclusion rejected).
  - Directory includes read only `*.conf` files, skip dotfiles, sorted alphabetically for determinism.
- **Validation is schema-driven, not scattered.** `configdata.c` declares one static table (`config_file_settings[]`) per parameter: name, type (`CONFIG_BOOL/INT/STRING/FAILOVER_MODE/CONNECTION_CHECK_TYPE/...`), pointer into the live config struct, default value, min value (ints), max length (strings), and optional `process_func`/`postprocess_func` hooks (e.g. path canonicalization). `parse_configuration_item()` in `configfile.c` dispatches purely off this table — the table is simultaneously the parser's dispatch, the validator, and the source for `--dump-config` output. One source of truth, not three.
- **Errors accumulate, they don't fail fast.** Config parsing collects into an `ItemList` of errors/warnings and reports all of them together at startup, rather than stopping at the first bad line — a UX decision worth noting when reviewing his error handling.
- Unknown parameter names are **not** rejected (repmgr doesn't error on unrecognized keys) — check whether that's actually the behavior he wants to replicate or diverge from.

## What to check in his code

1. **Escaping/quoting fidelity**: does his parser handle a `conninfo` value containing an escaped quote or a doubled `''`? A naive `splitn('=', 2)` + `split("'")` approach (his first draft did this) silently mishandles embedded quotes and any trailing `# comment` after a quoted value.
2. **Type/range validation**: is there a declarative table (or Rust-idiomatic equivalent — a derive macro, a struct of field descriptors) mapping name → type → default → constraints, or is validation ad hoc per call site?
3. **Error accumulation vs early-exit**: current code (`parse_repmgr_conf.rs`) calls `process::exit(-1)` on the first malformed line — flag this as a deliberate divergence from repmgr's accumulate-and-report-all model, and ask if that's intentional for now.
4. **Required vs optional fields**: does anything check that `node_id`/`node_name`/`conninfo` are actually present after parsing, matching repmgr's "required configuration items" section in `repmgr.conf.sample`?
5. Don't push repmgr-rs toward include-file support, directory includes, or full escape-sequence support unless he's actually at that stage — call out the gap, let him decide priority.

Deliver findings as a short list: what real repmgr does, what his code does, and the concrete scenario (input line) where the difference would bite. Do not propose a rewrite; propose the smallest next test case that would expose the gap.
