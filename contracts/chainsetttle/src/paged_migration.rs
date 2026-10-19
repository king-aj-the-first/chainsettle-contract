//! Paged migration system for large storage upgrades.
//!
//! When upgrading contract storage schema with tens of thousands of shipments,
//! processing everything in a single transaction would exceed the ledger's
//! instruction limit. This module provides a chunked, resumable migration
//! pattern that allows an admin to incrementally migrate storage over multiple
//! invocations, tracking progress in a persistent cursor.
//!
//! # Usage
//!
//! 1. Admin calls `start_migration(env, migration_id, total_items)` to initialize.
//! 2. Admin repeatedly calls `migrate_chunk(env, migration_id, chunk_size)` until
//!    `get_migration_progress(env, migration_id)` returns `is_complete = true`.
//! 3. Each `migrate_chunk` call processes up to `chunk_size` items and updates
//!    the persistent cursor, so the migration can be paused/resumed at any time.
//!
//! # Example Migration: V1 → V2 Schema
//!
//! ```rust
//! // In your contract upgrade function:
//! pub fn migrate_storage_v1_to_v2(env: Env, chunk_size: u32) -> MigrationProgress {
//!     let migration_id = String::from_str(&env, "v1_to_v2");
//!     
//!     // First call: start migration
//!     if !migration_exists(&env, &migration_id) {
//!         let all_shipments = storage::get_all_shipments(&env);
//!         start_migration(&env, &migration_id, all_shipments.len());
//!     }
//!     
//!     // Process next chunk
//!     migrate_chunk(&env, &migration_id, chunk_size, |env, item_index| {
//!         let all_shipments = storage::get_all_shipments(env);
//!         let shipment_id = all_shipments.get(item_index).unwrap();
//!         
//!         // Read V1 data
//!         if let Some(v1_shipment) = env.storage().persistent()
//!             .get::<DataKey, ShipmentV1>(&DataKey::V1Shipment(shipment_id.clone())) {
//!             
//!             // Transform to V2
//!             let v2_shipment = transform_v1_to_v2(v1_shipment);
//!             
//!             // Write V2 data
//!             env.storage().persistent()
//!                 .set(&DataKey::V2Shipment(shipment_id.clone()), &v2_shipment);
//!             
//!             // Remove old V1 key to reclaim storage rent
//!             env.storage().persistent().remove(&DataKey::V1Shipment(shipment_id));
//!         }
//!     });
//!     
//!     get_migration_progress(&env, &migration_id)
//! }
//! ```

use soroban_sdk::{contracttype, Env, String};

/// Migration progress tracking structure.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub struct MigrationProgress {
    /// Unique identifier for this migration (e.g. "v1_to_v2_shipments").
    pub migration_id: String,
    /// Total number of items to migrate.
    pub total_items: u32,
    /// Number of items successfully migrated so far.
    pub items_migrated: u32,
    /// Whether migration is complete.
    pub is_complete: bool,
    /// Ledger at which migration was started.
    pub started_at_ledger: u32,
    /// Ledger at which last chunk was processed (None if never started).
    pub last_chunk_ledger: Option<u32>,
}

/// Migration state key for persistent storage.
#[contracttype]
#[derive(Clone)]
enum MigrationKey {
    Progress(String),
}

/// Initialize a new migration. Idempotent — safe to call multiple times.
///
/// # Arguments
/// * `migration_id` - Unique identifier for this migration
/// * `total_items` - Total number of items that need to be migrated
///
/// # Panics
/// Panics if a different migration with the same ID exists and is incomplete.
pub fn start_migration(env: &Env, migration_id: &String, total_items: u32) {
    if let Some(existing) = get_migration_progress(env, migration_id) {
        if existing.is_complete {
            // Already done, no-op
            return;
        }
        if existing.total_items != total_items {
            panic!("migration already in progress with different total_items");
        }
        // Same migration, already started, continue
        return;
    }

    let progress = MigrationProgress {
        migration_id: migration_id.clone(),
        total_items,
        items_migrated: 0,
        is_complete: false,
        started_at_ledger: env.ledger().sequence(),
        last_chunk_ledger: None,
    };

    env.storage()
        .persistent()
        .set(&MigrationKey::Progress(migration_id.clone()), &progress);
}

/// Check if a migration exists (started or completed).
pub fn migration_exists(env: &Env, migration_id: &String) -> bool {
    env.storage()
        .persistent()
        .has(&MigrationKey::Progress(migration_id.clone()))
}

/// Get current migration progress. Returns None if migration not started.
pub fn get_migration_progress(env: &Env, migration_id: &String) -> Option<MigrationProgress> {
    env.storage()
        .persistent()
        .get(&MigrationKey::Progress(migration_id.clone()))
}

/// Process the next chunk of items in the migration.
///
/// # Arguments
/// * `migration_id` - Unique identifier for the migration
/// * `chunk_size` - Maximum number of items to process in this invocation
/// * `migrate_fn` - Callback function that processes a single item by index
///
/// # Returns
/// Updated `MigrationProgress` after processing this chunk.
///
/// # Panics
/// Panics if migration doesn't exist or is already complete.
pub fn migrate_chunk<F>(
    env: &Env,
    migration_id: &String,
    chunk_size: u32,
    mut migrate_fn: F,
) -> MigrationProgress
where
    F: FnMut(&Env, u32),
{
    let mut progress = get_migration_progress(env, migration_id)
        .unwrap_or_else(|| panic!("migration not started"));

    if progress.is_complete {
        panic!("migration already complete");
    }

    let start_index = progress.items_migrated;
    let end_index = (start_index + chunk_size).min(progress.total_items);

    // Process items in this chunk
    for i in start_index..end_index {
        migrate_fn(env, i);
    }

    // Update progress
    progress.items_migrated = end_index;
    progress.is_complete = progress.items_migrated >= progress.total_items;
    progress.last_chunk_ledger = Some(env.ledger().sequence());

    // Persist updated progress
    env.storage()
        .persistent()
        .set(&MigrationKey::Progress(migration_id.clone()), &progress);

    progress
}

/// Abort an in-progress migration and remove its state.
/// Typically used when a migration needs to be restarted from scratch.
///
/// # Arguments
/// * `migration_id` - Unique identifier for the migration to abort
///
/// # Panics
/// Panics if migration doesn't exist.
pub fn abort_migration(env: &Env, migration_id: &String) {
    if !migration_exists(env, migration_id) {
        panic!("migration does not exist");
    }

    env.storage()
        .persistent()
        .remove(&MigrationKey::Progress(migration_id.clone()));
}

/// Remove a completed migration's progress record to reclaim storage.
/// Only succeeds if migration is complete.
///
/// # Arguments
/// * `migration_id` - Unique identifier for the completed migration
///
/// # Panics
/// Panics if migration doesn't exist or is not complete.
pub fn cleanup_migration(env: &Env, migration_id: &String) {
    let progress = get_migration_progress(env, migration_id)
        .unwrap_or_else(|| panic!("migration does not exist"));

    if !progress.is_complete {
        panic!("cannot cleanup incomplete migration");
    }

    env.storage()
        .persistent()
        .remove(&MigrationKey::Progress(migration_id.clone()));
}

#[cfg(test)]
mod test_paged_migration {
    use super::*;
    use soroban_sdk::{testutils::Ledger, Env, String};

    #[test]
    fn test_migration_lifecycle() {
        let env = Env::default();
        env.ledger().with_mut(|li| li.sequence_number = 100);

        let migration_id = String::from_str(&env, "test_migration");

        // Start migration
        start_migration(&env, &migration_id, 100);
        let progress = get_migration_progress(&env, &migration_id).unwrap();
        assert_eq!(progress.total_items, 100);
        assert_eq!(progress.items_migrated, 0);
        assert!(!progress.is_complete);

        // Process first chunk
        let mut processed = soroban_sdk::Vec::new(&env);
        migrate_chunk(&env, &migration_id, 30, |_env, idx| {
            processed.push_back(idx);
        });
        assert_eq!(processed.len(), 30);

        let progress = get_migration_progress(&env, &migration_id).unwrap();
        assert_eq!(progress.items_migrated, 30);
        assert!(!progress.is_complete);

        // Process remaining chunks
        migrate_chunk(&env, &migration_id, 50, |_env, _idx| {});
        let progress = get_migration_progress(&env, &migration_id).unwrap();
        assert_eq!(progress.items_migrated, 80);
        assert!(!progress.is_complete);

        migrate_chunk(&env, &migration_id, 50, |_env, _idx| {});
        let progress = get_migration_progress(&env, &migration_id).unwrap();
        assert_eq!(progress.items_migrated, 100);
        assert!(progress.is_complete);

        // Cleanup
        cleanup_migration(&env, &migration_id);
        assert!(!migration_exists(&env, &migration_id));
    }

    #[test]
    #[should_panic(expected = "migration already complete")]
    fn test_cannot_process_complete_migration() {
        let env = Env::default();
        let migration_id = String::from_str(&env, "test_migration");

        start_migration(&env, &migration_id, 10);
        migrate_chunk(&env, &migration_id, 100, |_env, _idx| {});

        // This should panic
        migrate_chunk(&env, &migration_id, 10, |_env, _idx| {});
    }

    #[test]
    #[should_panic(expected = "cannot cleanup incomplete migration")]
    fn test_cannot_cleanup_incomplete_migration() {
        let env = Env::default();
        let migration_id = String::from_str(&env, "test_migration");

        start_migration(&env, &migration_id, 100);
        migrate_chunk(&env, &migration_id, 30, |_env, _idx| {});

        // This should panic
        cleanup_migration(&env, &migration_id);
    }

    #[test]
    fn test_idempotent_start() {
        let env = Env::default();
        let migration_id = String::from_str(&env, "test_migration");

        start_migration(&env, &migration_id, 100);
        start_migration(&env, &migration_id, 100);

        let progress = get_migration_progress(&env, &migration_id).unwrap();
        assert_eq!(progress.total_items, 100);
        assert_eq!(progress.items_migrated, 0);
    }

    #[test]
    fn test_abort_migration() {
        let env = Env::default();
        let migration_id = String::from_str(&env, "test_migration");

        start_migration(&env, &migration_id, 100);
        migrate_chunk(&env, &migration_id, 30, |_env, _idx| {});

        abort_migration(&env, &migration_id);
        assert!(!migration_exists(&env, &migration_id));
    }
}
