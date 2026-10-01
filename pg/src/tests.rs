// This module is required by `cargo pgrx test` invocations.
// It must be visible at the root of your extension crate.
#[pgrx::pg_schema]
mod tests {
    use pgrx::prelude::*;

    #[pg_test]
    fn test_get_local_node_id() {
        assert_eq!(-1, crate::get_local_node_id());
    }
}

#[cfg(test)]
pub mod pg_test {
    pub fn setup(_options: Vec<&str>) {
        // perform one-off initialization when the pg_test framework starts
    }

    #[must_use]
    pub fn postgresql_conf_options() -> Vec<&'static str> {
        vec!["shared_preload_libraries = 'repmgr'"]
    }
}
