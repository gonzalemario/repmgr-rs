# repmgr-rs

A Rust reimplementation of repmgr (PostgreSQL replication manager). It's an independent design,
not wire- or schema-compatible with upstream repmgr.

## Layout

- `cli/`: the `repmgr` CLI (`main.rs`, `dbconnector.rs`, sample `repmgr.conf`).
- `pg/`: a Postgres extension built with pgrx `=0.19.3` (crate name `repmgr`, default feature
  `pg18`). It currently holds node state in shared memory (`PgLwLock<RepmgrNode>`) and exposes
  `get_local_node_id()`.
  - All `#[pg_test]` tests live in `pg/src/tests.rs`, including the `pg_test` module (`setup`,
    `postgresql_conf_options` with `shared_preload_libraries = 'repmgr'`). `lib.rs` declares
    `mod tests;` and re-exports with `pub use tests::pg_test;`, not `mod pg_test;`, which would
    look for a missing `src/pg_test.rs`.
  - Run tests from `pg/` with `cargo pgrx test pg18`.
- `.claude` is a symlink to `llm/claude`, so the skills and this file load from there.
- `llm/EXTENSION_FUNCTIONS.md`: a reference for the 18 SQL functions in upstream repmgr's
  extension. It has a quick list, node ID semantics, the C implementations, callers, and what
  each one means for repmgr-rs.
- `llm/VERIFICATION.md`: the running list of test scenarios (`repmgr-verification-planner`).

## Upstream reference

- The repmgr 5.5.0 C source is at `~/repmgr-5.5.0` (that's in mario's home). Use the `repmgr-*`
  skills to work with it.
- Upstream node ID: a 32-bit `INT` of 1 or more, `-1` meaning unset. `get_local_node_id()`
  returns NULL when the library isn't preloaded. The repmgr-rs extension uses `i16` for now.

## Working with Mario

- Mario writes the implementation code. Don't edit his source files unless he asks for it.
- When he says "just show me", read the files he named and answer in chat. Don't run side
  experiments such as copying the crate to a scratch dir or test builds unless he asks for them.
- Keep answers short and direct.
