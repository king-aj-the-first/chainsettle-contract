//! Chain-of-custody handoff log per milestone.
//!
//! This module provides an immutable, append-only audit trail for tracking the
//! physical custody chain of goods/assets as they move through a shipment's
//! lifecycle. Each milestone can accumulate custody handoff events as different
//! parties (supplier, logistics, buyer, third-party inspectors) take possession.
//!
//! # Use Cases
//!
//! - **Multi-leg shipping**: Track handoffs from supplier → warehouse → freight forwarder → buyer.
//! - **Quality control checkpoints**: Record inspector sign-offs at various stages.
//! - **Compliance audit**: Provide an immutable, timestamped chain-of-custody for regulatory review.
//! - **Dispute evidence**: Custody log serves as on-chain proof of who held goods when.
//!
//! # Design
//!
//! - Each `CustodyHandoff` entry records: custodian address, handoff type (received/transferred),
//!   optional metadata hash (e.g. signed receipt IPFS CID), and ledger timestamp.
//! - Log is stored per (shipment_id, milestone_index) with a configurable size cap (default 50).
//! - Append-only — entries cannot be modified or deleted once recorded.
//! - Retrieval is paginated for gas efficiency when reading large logs.
//!
//! # Example Workflow
//!
//! ```rust
//! // Supplier ships goods to logistics partner
//! custody_log::record_handoff(
//!     &env,
//!     &shipment_id,
//!     milestone_index,
//!     &logistics_address,
//!     HandoffType::Received,
//!     Some(receipt_hash),
//! );
//!
//! // Logistics transfers to buyer
//! custody_log::record_handoff(
//!     &env,
//!     &shipment_id,
//!     milestone_index,
//!     &buyer_address,
//!     HandoffType::Received,
//!     Some(delivery_receipt_hash),
//! );
//!
//! // Buyer confirms receipt
//! custody_log::record_handoff(
//!     &env,
//!     &shipment_id,
//!     milestone_index,
//!     &buyer_address,
//!     HandoffType::Confirmed,
//!     None,
//! );
//! ```

use soroban_sdk::{contracttype, Address, BytesN, Env, String, Vec};

/// Type of custody handoff event.
#[contracttype]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HandoffType {
    /// Entity received possession of goods.
    Received,
    /// Entity transferred possession to another party.
    Transferred,
    /// Entity confirmed receipt and condition of goods.
    Confirmed,
    /// Inspector performed quality check.
    Inspected,
    /// Customs clearance or regulatory checkpoint.
    Cleared,
    /// Goods stored at a warehouse or facility.
    Stored,
    /// Goods departed from a location.
    Departed,
    /// Generic handoff event for extensibility.
    Other,
}

/// Single custody handoff entry in the chain-of-custody log.
#[contracttype]
#[derive(Clone, PartialEq, Debug)]
pub struct CustodyHandoff {
    /// Address of the custodian (party taking/transferring custody).
    pub custodian: Address,
    /// Type of handoff event.
    pub handoff_type: HandoffType,
    /// Optional off-chain evidence hash (e.g. IPFS CID of signed receipt, photo, etc.).
    pub evidence_hash: Option<BytesN<32>>,
    /// Ledger sequence at which this handoff was recorded.
    pub ledger: u32,
    /// Optional geographic location identifier (e.g. "LAX", "warehouse_42", etc.).
    pub location: Option<String>,
    /// Optional notes field for additional context (max 256 chars recommended).
    pub notes: Option<String>,
}

/// Storage key for custody logs.
#[contracttype]
#[derive(Clone)]
enum CustodyLogKey {
    /// Log entries for (shipment_id, milestone_index).
    Log(String, u32),
    /// Configuration: max entries per log (default 50).
    MaxEntries,
}

/// Default maximum number of custody handoff entries per milestone.
const DEFAULT_MAX_ENTRIES: u32 = 50;

/// Record a new custody handoff event.
///
/// # Arguments
/// * `shipment_id` - Shipment identifier
/// * `milestone_index` - Milestone index (0-based)
/// * `custodian` - Address of the party taking/transferring custody
/// * `handoff_type` - Type of custody event
/// * `evidence_hash` - Optional off-chain evidence hash (e.g. IPFS CID)
///
/// # Panics
/// Panics if the log has reached maximum capacity.
pub fn record_handoff(
    env: &Env,
    shipment_id: &String,
    milestone_index: u32,
    custodian: &Address,
    handoff_type: HandoffType,
    evidence_hash: Option<BytesN<32>>,
) {
    record_handoff_full(
        env,
        shipment_id,
        milestone_index,
        custodian,
        handoff_type,
        evidence_hash,
        None,
        None,
    );
}

/// Record a custody handoff with full metadata (location + notes).
///
/// # Arguments
/// * `shipment_id` - Shipment identifier
/// * `milestone_index` - Milestone index (0-based)
/// * `custodian` - Address of the party taking/transferring custody
/// * `handoff_type` - Type of custody event
/// * `evidence_hash` - Optional off-chain evidence hash
/// * `location` - Optional location identifier
/// * `notes` - Optional notes (max 256 chars recommended)
///
/// # Panics
/// Panics if the log has reached maximum capacity.
pub fn record_handoff_full(
    env: &Env,
    shipment_id: &String,
    milestone_index: u32,
    custodian: &Address,
    handoff_type: HandoffType,
    evidence_hash: Option<BytesN<32>>,
    location: Option<String>,
    notes: Option<String>,
) {
    custodian.require_auth();

    let mut log = get_custody_log(env, shipment_id, milestone_index);
    let max_entries = get_max_entries(env);

    if log.len() >= max_entries {
        panic!("custody log full");
    }

    let entry = CustodyHandoff {
        custodian: custodian.clone(),
        handoff_type,
        evidence_hash,
        ledger: env.ledger().sequence(),
        location,
        notes,
    };

    log.push_back(entry);

    let key = CustodyLogKey::Log(shipment_id.clone(), milestone_index);
    env.storage().persistent().set(&key, &log);

    // Emit event for off-chain indexing
    env.events().publish(
        (
            soroban_sdk::symbol_short!("custody"),
            shipment_id.clone(),
            milestone_index,
        ),
        (custodian.clone(), handoff_type, env.ledger().sequence()),
    );
}

/// Get the full custody log for a milestone.
///
/// # Returns
/// Ordered vector of custody handoff entries (chronological order).
pub fn get_custody_log(env: &Env, shipment_id: &String, milestone_index: u32) -> Vec<CustodyHandoff> {
    let key = CustodyLogKey::Log(shipment_id.clone(), milestone_index);
    env.storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| Vec::new(env))
}

/// Get a paginated slice of the custody log.
///
/// # Arguments
/// * `shipment_id` - Shipment identifier
/// * `milestone_index` - Milestone index (0-based)
/// * `offset` - Starting index (0-based)
/// * `limit` - Maximum number of entries to return
///
/// # Returns
/// Vector of custody handoff entries within the requested range.
pub fn get_custody_log_paginated(
    env: &Env,
    shipment_id: &String,
    milestone_index: u32,
    offset: u32,
    limit: u32,
) -> Vec<CustodyHandoff> {
    let full_log = get_custody_log(env, shipment_id, milestone_index);
    let mut result = Vec::new(env);

    let start = offset.min(full_log.len());
    let end = (offset + limit).min(full_log.len());

    for i in start..end {
        result.push_back(full_log.get_unchecked(i));
    }

    result
}

/// Get the number of custody handoff entries for a milestone.
pub fn get_custody_log_count(env: &Env, shipment_id: &String, milestone_index: u32) -> u32 {
    get_custody_log(env, shipment_id, milestone_index).len()
}

/// Get the most recent custody handoff for a milestone.
///
/// # Returns
/// The last custody handoff entry, or None if log is empty.
pub fn get_latest_handoff(
    env: &Env,
    shipment_id: &String,
    milestone_index: u32,
) -> Option<CustodyHandoff> {
    let log = get_custody_log(env, shipment_id, milestone_index);
    if log.is_empty() {
        None
    } else {
        Some(log.get_unchecked(log.len() - 1))
    }
}

/// Get the current custodian (party who last received custody).
///
/// # Returns
/// Address of the current custodian, or None if no handoffs recorded.
pub fn get_current_custodian(
    env: &Env,
    shipment_id: &String,
    milestone_index: u32,
) -> Option<Address> {
    get_latest_handoff(env, shipment_id, milestone_index).map(|h| h.custodian)
}

/// Check if a specific address appears in the custody chain.
///
/// # Returns
/// True if the address has ever taken custody for this milestone.
pub fn is_in_custody_chain(
    env: &Env,
    shipment_id: &String,
    milestone_index: u32,
    address: &Address,
) -> bool {
    let log = get_custody_log(env, shipment_id, milestone_index);
    for i in 0..log.len() {
        if log.get_unchecked(i).custodian == *address {
            return true;
        }
    }
    false
}

/// Get all handoffs by a specific custodian for a milestone.
///
/// # Returns
/// Vector of custody handoff entries where custodian matches the given address.
pub fn get_handoffs_by_custodian(
    env: &Env,
    shipment_id: &String,
    milestone_index: u32,
    custodian: &Address,
) -> Vec<CustodyHandoff> {
    let log = get_custody_log(env, shipment_id, milestone_index);
    let mut result = Vec::new(env);

    for i in 0..log.len() {
        let entry = log.get_unchecked(i);
        if entry.custodian == *custodian {
            result.push_back(entry);
        }
    }

    result
}

/// Get all handoffs of a specific type for a milestone.
///
/// # Returns
/// Vector of custody handoff entries matching the given handoff type.
pub fn get_handoffs_by_type(
    env: &Env,
    shipment_id: &String,
    milestone_index: u32,
    handoff_type: HandoffType,
) -> Vec<CustodyHandoff> {
    let log = get_custody_log(env, shipment_id, milestone_index);
    let mut result = Vec::new(env);

    for i in 0..log.len() {
        let entry = log.get_unchecked(i);
        if entry.handoff_type == handoff_type {
            result.push_back(entry);
        }
    }

    result
}

/// Verify custody chain continuity: checks that each transfer has a matching receive.
///
/// # Returns
/// True if the custody chain is properly formed (every Transferred has a subsequent Received).
pub fn verify_custody_chain(env: &Env, shipment_id: &String, milestone_index: u32) -> bool {
    let log = get_custody_log(env, shipment_id, milestone_index);
    let mut expecting_receive = false;

    for i in 0..log.len() {
        let entry = log.get_unchecked(i);
        match entry.handoff_type {
            HandoffType::Transferred => {
                if expecting_receive {
                    // Two transfers in a row without a receive
                    return false;
                }
                expecting_receive = true;
            }
            HandoffType::Received => {
                expecting_receive = false;
            }
            _ => {
                // Other event types don't affect transfer/receive pairing
            }
        }
    }

    // Chain is valid if we're not expecting a receive at the end
    !expecting_receive
}

/// Admin function to set maximum custody log entries per milestone.
///
/// # Arguments
/// * `max_entries` - New maximum (must be > 0 and <= 200)
///
/// # Panics
/// Panics if max_entries is invalid.
pub fn set_max_entries(env: &Env, max_entries: u32) {
    if max_entries == 0 || max_entries > 200 {
        panic!("max_entries must be between 1 and 200");
    }

    env.storage()
        .instance()
        .set(&CustodyLogKey::MaxEntries, &max_entries);
}

/// Get the configured maximum custody log entries per milestone.
pub fn get_max_entries(env: &Env) -> u32 {
    env.storage()
        .instance()
        .get(&CustodyLogKey::MaxEntries)
        .unwrap_or(DEFAULT_MAX_ENTRIES)
}

/// Clear custody log for a milestone (admin/emergency use only).
///
/// **WARNING**: This breaks immutability and should only be used in exceptional
/// circumstances (e.g. data corruption, migration, or legal order).
///
/// # Panics
/// Panics if called by non-admin (caller must ensure admin check before calling).
pub fn clear_custody_log(env: &Env, shipment_id: &String, milestone_index: u32) {
    let key = CustodyLogKey::Log(shipment_id.clone(), milestone_index);
    env.storage().persistent().remove(&key);

    env.events().publish(
        (
            soroban_sdk::symbol_short!("log_clear"),
            shipment_id.clone(),
        ),
        milestone_index,
    );
}

#[cfg(test)]
mod test_custody_log {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env, String};

    #[test]
    fn test_record_and_retrieve_handoff() {
        let env = Env::default();
        env.mock_all_auths();

        let shipment_id = String::from_str(&env, "SHIP001");
        let custodian = Address::generate(&env);

        record_handoff(
            &env,
            &shipment_id,
            0,
            &custodian,
            HandoffType::Received,
            None,
        );

        let log = get_custody_log(&env, &shipment_id, 0);
        assert_eq!(log.len(), 1);
        assert_eq!(log.get_unchecked(0).custodian, custodian);
        assert_eq!(log.get_unchecked(0).handoff_type, HandoffType::Received);
    }

    #[test]
    fn test_multiple_handoffs() {
        let env = Env::default();
        env.mock_all_auths();

        let shipment_id = String::from_str(&env, "SHIP001");
        let supplier = Address::generate(&env);
        let logistics = Address::generate(&env);
        let buyer = Address::generate(&env);

        record_handoff(
            &env,
            &shipment_id,
            0,
            &supplier,
            HandoffType::Transferred,
            None,
        );
        record_handoff(
            &env,
            &shipment_id,
            0,
            &logistics,
            HandoffType::Received,
            None,
        );
        record_handoff(
            &env,
            &shipment_id,
            0,
            &logistics,
            HandoffType::Transferred,
            None,
        );
        record_handoff(&env, &shipment_id, 0, &buyer, HandoffType::Received, None);

        let log = get_custody_log(&env, &shipment_id, 0);
        assert_eq!(log.len(), 4);

        let current = get_current_custodian(&env, &shipment_id, 0).unwrap();
        assert_eq!(current, buyer);
    }

    #[test]
    fn test_pagination() {
        let env = Env::default();
        env.mock_all_auths();

        let shipment_id = String::from_str(&env, "SHIP001");
        let custodian = Address::generate(&env);

        // Record 10 handoffs
        for _ in 0..10 {
            record_handoff(
                &env,
                &shipment_id,
                0,
                &custodian,
                HandoffType::Received,
                None,
            );
        }

        // Get first 5
        let page1 = get_custody_log_paginated(&env, &shipment_id, 0, 0, 5);
        assert_eq!(page1.len(), 5);

        // Get next 5
        let page2 = get_custody_log_paginated(&env, &shipment_id, 0, 5, 5);
        assert_eq!(page2.len(), 5);

        // Get beyond end
        let page3 = get_custody_log_paginated(&env, &shipment_id, 0, 10, 5);
        assert_eq!(page3.len(), 0);
    }

    #[test]
    fn test_verify_custody_chain() {
        let env = Env::default();
        env.mock_all_auths();

        let shipment_id = String::from_str(&env, "SHIP001");
        let supplier = Address::generate(&env);
        let logistics = Address::generate(&env);

        // Valid chain: Transferred → Received
        record_handoff(
            &env,
            &shipment_id,
            0,
            &supplier,
            HandoffType::Transferred,
            None,
        );
        record_handoff(
            &env,
            &shipment_id,
            0,
            &logistics,
            HandoffType::Received,
            None,
        );

        assert!(verify_custody_chain(&env, &shipment_id, 0));

        // Invalid chain: Transferred → Transferred (missing receive)
        let shipment_id2 = String::from_str(&env, "SHIP002");
        record_handoff(
            &env,
            &shipment_id2,
            0,
            &supplier,
            HandoffType::Transferred,
            None,
        );
        record_handoff(
            &env,
            &shipment_id2,
            0,
            &logistics,
            HandoffType::Transferred,
            None,
        );

        assert!(!verify_custody_chain(&env, &shipment_id2, 0));
    }

    #[test]
    fn test_filter_by_custodian() {
        let env = Env::default();
        env.mock_all_auths();

        let shipment_id = String::from_str(&env, "SHIP001");
        let addr1 = Address::generate(&env);
        let addr2 = Address::generate(&env);

        record_handoff(&env, &shipment_id, 0, &addr1, HandoffType::Received, None);
        record_handoff(&env, &shipment_id, 0, &addr2, HandoffType::Received, None);
        record_handoff(&env, &shipment_id, 0, &addr1, HandoffType::Transferred, None);

        let addr1_handoffs = get_handoffs_by_custodian(&env, &shipment_id, 0, &addr1);
        assert_eq!(addr1_handoffs.len(), 2);

        let addr2_handoffs = get_handoffs_by_custodian(&env, &shipment_id, 0, &addr2);
        assert_eq!(addr2_handoffs.len(), 1);
    }

    #[test]
    fn test_filter_by_type() {
        let env = Env::default();
        env.mock_all_auths();

        let shipment_id = String::from_str(&env, "SHIP001");
        let custodian = Address::generate(&env);

        record_handoff(
            &env,
            &shipment_id,
            0,
            &custodian,
            HandoffType::Received,
            None,
        );
        record_handoff(
            &env,
            &shipment_id,
            0,
            &custodian,
            HandoffType::Inspected,
            None,
        );
        record_handoff(
            &env,
            &shipment_id,
            0,
            &custodian,
            HandoffType::Received,
            None,
        );

        let received = get_handoffs_by_type(&env, &shipment_id, 0, HandoffType::Received);
        assert_eq!(received.len(), 2);

        let inspected = get_handoffs_by_type(&env, &shipment_id, 0, HandoffType::Inspected);
        assert_eq!(inspected.len(), 1);
    }

    #[test]
    #[should_panic(expected = "custody log full")]
    fn test_log_capacity_limit() {
        let env = Env::default();
        env.mock_all_auths();

        let shipment_id = String::from_str(&env, "SHIP001");
        let custodian = Address::generate(&env);

        set_max_entries(&env, 5);

        // Record 5 handoffs (should succeed)
        for _ in 0..5 {
            record_handoff(
                &env,
                &shipment_id,
                0,
                &custodian,
                HandoffType::Received,
                None,
            );
        }

        // 6th handoff should panic
        record_handoff(
            &env,
            &shipment_id,
            0,
            &custodian,
            HandoffType::Received,
            None,
        );
    }
}
