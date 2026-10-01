use pgrx::prelude::*;

::pgrx::pg_module_magic!(name, version);

use pgrx::{pg_shmem_init, PGRXSharedMemory, PgLwLock, PgSharedMemoryInitialization};

#[derive(Copy, Clone)]
pub struct RepmgrNode {
    node_id: i16,
    name: [u8; 32]
}

impl Default for RepmgrNode {
    fn default() -> Self {
        Self { node_id: -1, name: [1; 32] }
    }
}

unsafe impl PGRXSharedMemory for RepmgrNode {}
static NODE: PgLwLock<RepmgrNode> = unsafe { PgLwLock::new(c"repmgr_node") };

#[pg_guard]
pub extern "C-unwind" fn _PG_init() {
    if unsafe { !pg_sys::process_shared_preload_libraries_in_progress } {
        pgrx::error!("repmgr must be added to shared_preload_libraries");
    }
    pg_shmem_init!(NODE);
}

#[pg_extern]
fn get_local_node_id() -> i16 {
    let node = NODE.share();
    node.node_id
}

/// This module is required by `cargo pgrx test` invocations.
/// It must be visible at the root of your extension crate.
#[cfg(any(test, feature = "pg_test"))]
mod tests;

#[cfg(test)]
pub use tests::*;
