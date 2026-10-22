# Query and pagination API reference

Read-only reference for every read-only `pub fn` in `contracts/chainsetttle/src/lib.rs`.
None of these functions require authorization and none mutate shipment state
(`get_shipment` and a few siblings bump the *instance* TTL, which does not touch
shipment data). For the full API including state-changing calls, see the doc
comments on each `pub fn` in `lib.rs`.

## Cursor semantics

Paginated queries share one convention:

- `cursor: Option<u32>` is a **0-based index** into the underlying ID list.
  `None` (or `Some(0)`) starts at the head.
- `limit: u32` is **clamped** to a contract constant (see below); passing a
  larger value silently returns at most the cap, never an error.
- `list_shipments` returns `(Vec<String>, Option<u32>)`: the second element is
  the cursor for the next page, or `None` when the list is exhausted. A cursor
  at or past the end returns `(vec![], None)`.
- `get_shipments_by_arbiter` / `get_shipments_by_logistics` return only
  `Vec<String>` (via the shared `paginate_shipment_ids` helper): advance by
  passing `cursor + returned.len()` as the next cursor; a short/empty page
  means you are done.
- `get_dispute_history` takes `(address, cursor, limit)` with the same
  index-based rules; `limit` is clamped to `LIST_SHIPMENTS_MAX_PAGE`.
- `get_upcoming_deadlines(address, within_ledgers, limit)` is **not**
  cursor-based: it scans buyer/supplier indexes for milestones with an
  effective deadline in `(now, now + within_ledgers]`, sorted ascending and
  capped at `limit` (clamped to `LIST_SHIPMENTS_MAX_PAGE`). Zero
  `within_ledgers` or `limit` returns empty.
- `get_shipment_summaries(ids)` is batch-by-ID, not cursor-based: unknown IDs
  are silently skipped, and the batch is rejected when `ids.len()` exceeds
  `get_max_summary_batch` (`set_max_summary_batch` /
  `constants::DEFAULT_MAX_SUMMARY_BATCH` = 50 when unset).

Page-size caps (`contracts/chainsetttle/src/constants.rs`):

| Constant | Value | Gates |
| --- | --- | --- |
| `LIST_SHIPMENTS_MAX_PAGE` | 50 | `list_shipments`, `get_dispute_history`, `get_shipments_by_arbiter`, `get_shipments_by_logistics`, `get_upcoming_deadlines` |
| `LIST_SHIPMENTS_BY_TOKEN_MAX_PAGE` | 50 | reserved for token-filtered listing |
| `DEFAULT_MAX_SUMMARY_BATCH` | 50 | `get_shipment_summaries` (via `get_max_summary_batch`) |

Indexes behind the listing queries: `AllShipments` / `ShipmentsByStatus` (for
`list_shipments`), `BuyerShipments` / `SupplierShipments` (uncapped full-list
returns), `ArbiterShipments` / `LogisticsShipments` (paginated),
`JurisdictionShipments` (via `get_shipments_by_jurisdiction`, uncapped).
Archived shipments stay in the party indexes.

## Queries by domain

- **Shipment core:** `get_shipment`, `get_shipment_summary`,
  `get_shipment_summaries`, `get_milestone`, `get_completion_percentage`,
  `get_escrow_balance`, `get_shipment_count`, `get_shipment_metadata`,
  `get_shipment_metadata_keys`, `get_milestone_notes`, `get_shipment_fees_paid`,
  `get_shipment_mediator`, `get_mediation_proposal`, `get_shipment_jurisdiction`,
  `get_new_milestone_proposal`, `get_milestone_split_proposal`,
  `get_milestone_merge_proposal`, `get_pending_expiry_extension`,
  `get_total_expiry_extended`, `get_invoice_hash`, `get_amendment_log`,
  `get_milestone_deadline`, `get_proof_whitelist`, `get_milestone_proof_type`,
  `get_evidence_count`, `get_proof_submitter`
- **Shipment lists / indexes:** `list_shipments`, `get_shipments_by_supplier`,
  `get_shipments_by_buyer`, `get_shipments_by_arbiter`,
  `get_shipments_by_logistics`, `get_shipments_by_jurisdiction`,
  `get_upcoming_deadlines`, `find_shipments_by_metadata`
- **Disputes / arbiters / panels:** `get_active_disputes`, `get_dispute_votes`,
  `get_dispute_split`, `get_dispute_history`, `get_dispute_evidence`,
  `get_dispute_resolution_reason`, `get_arbiter_panel`, `get_arbiter_pool`,
  `get_arbiter_stats`, `get_panel_escalation`, `is_dispute_panel_escalated`,
  `is_arbiter_available`, `is_arbiter_slashed`, `get_appeal_window_ledgers`,
  `get_appeal_fee`, `get_appeal_fee_deposit`
- **Fees / treasury / economics:** `get_fee_config`, `get_treasury_revenue`,
  `get_fee_tier`, `get_fee_waiver`, `get_fee_waiver_proposal`,
  `is_fee_holiday_active`, `get_referral_fee_bps`, `get_conversion_rate`,
  `get_pending_payout`, `get_payout_currency_preference`,
  `get_payout_assignment`, `get_loser_pays_arbiter_fee`, `get_supplier_earnings`,
  `get_buyer_spend`, `estimate_shipment_costs`, `preview_milestone_payout`,
  `get_total_escrowed_value`, `get_tvl`, `get_tvl_cap`
- **Collateral / risk / limits:** `get_locked_collateral`,
  `get_incremental_collateral`, `get_collateral_slash_bps`,
  `is_collateral_slashed`, `get_collateral_token`, `get_depeg_guard`,
  `is_token_depegged`, `get_supplier_exposure`, `get_supplier_exposure_cap`,
  `get_buyer_spending_limit`, `get_buyer_spending_window_usage`,
  `get_buyer_open_disputes`, `get_buyer_dispute_cap`, `get_buyer_reliability`,
  `get_max_shipment_value`, `get_min_shipment_value`,
  `get_token_min_shipment_value`, `get_token_max_shipment_value`,
  `get_max_milestone_count`, `get_max_advance_percent`,
  `get_max_expiry_extension_ledgers`, `get_max_extension_requests`,
  `get_address_outflow_limit`, `get_shipment_creation_rate_limit`,
  `is_rate_limit_exempt`, `get_supplier_cancel_cooldown`
- **Compliance views:** `is_blacklisted`, `get_blacklist_appeal`,
  `is_whitelisted`, `get_approved_buyers`, `get_require_mutual_preapproval`,
  `is_on_compliance_hold`, `get_shipment_jurisdiction`,
  `get_shipments_by_jurisdiction`, `get_auto_blacklist_rule`
- **Reputation / ratings:** `get_reputation`, `get_supplier_tier`,
  `get_supplier_tier_config`, `get_supplier_recent_arbiters`,
  `get_rating_summary`, `get_rating_window_ledgers`, `is_fast_track_eligible`
- **Config / admin views:** `version`, `storage_schema_version`,
  `get_upgrade_proposal`, `get_pending_admin_actions`, `get_admin_log`,
  `get_admin_log_filtered`, `get_role_expiry`, `has_role`, `is_paused`,
  `is_operation_paused`, `is_emergency_frozen`, `get_emergency_freeze_proposal`,
  `get_emergency_unfreeze_proposal`, `get_freeze_supermajority_bps`,
  `get_confirmation_cooldown`, `get_auto_confirm_threshold`,
  `get_escalation_threshold`, `get_warning_lead_ledgers`,
  `get_dispute_filing_window`, `get_joint_confirmation_threshold`,
  `get_finality_delay_ledgers`, `get_max_dispute_bond_bps`,
  `get_max_overturned_before_slash`, `get_arbiter_stake`, `get_arbiter_slash_bps`,
  `get_arbiter_history_size`, `get_mediator_pool`, `get_oracle_group`,
  `get_oracle_attestation_count`, `get_inspection`, `get_allowed_tokens`,
  `get_max_allowed_tokens`, `get_buyer_allowed_tokens`,
  `get_proof_hash_length_bounds`, `get_proof_hash_required_prefix`,
  `get_refund_recipient`, `get_refund_sweep_window`, `get_archive_threshold`,
  `get_archived_shipment`, `is_shipment_paused`, `is_metadata_key_indexed`,
  `get_standing_order`, `get_vault_balance`, `get_warranty_balance`,
  `get_warranty_end_ledger`, `get_warranty_claim`, `get_retainage_balance`,
  `get_delivered_quantity`, `get_milestone_grade`, `get_quality_grades`,
  `get_milestone_payees`, `get_milestone_supplier`, `get_milestone_template`,
  `get_co_buyer`, `get_joint_confirmation_status`, `get_delegate`,
  `get_shipment_observers`, `get_confirmation_webhooks`, `get_nft_hook_enabled`,
  `get_shipment_risk`, `get_contract_stats`, `get_long_hold_rebate`,
  `get_clean_completion_rebate_bps`, `get_max_summary_batch`,
  `get_pause_min_notice_ledgers`, `get_pending_recovery`,
  `get_supplier_terms_profile`, `get_shipment_fee_override`
- **Auth-gated reads** (require the queried party's auth or admin, but change
  no state): `get_dispute_votes` is open; no read-only function requires auth.

## Complete alphabetical reference

| Function | Signature | Summary |
| --- | --- | --- |
<!-- count: 179 -->
| `estimate_shipment_costs` | `pub fn estimate_shipment_costs( env: Env, buyer: Address, token: Address, total_amount: i128, milestones: Vec<Milestone>, options: ShipmentOptions, ) -> ShipmentCostEstimate` | Estimates every fee and payout for a whole shipment before creation. |
| `find_shipments_by_metadata` | `pub fn find_shipments_by_metadata(env: Env, key: Symbol, value: String) -> Vec<String>` | #662: Shipment IDs whose metadata `key` currently equals `value`. |
| `get_active_disputes` | `pub fn get_active_disputes(env: Env) -> Vec<DisputeEntry>` |  |
| `get_address_outflow_limit` | `pub fn get_address_outflow_limit(env: Env, address: Address) -> (i128, u32, u32, i128)` | Read-only getter for per-address outflow config. |
| `get_admin_log` | `pub fn get_admin_log(env: Env) -> Vec<AuditEntry>` |  |
| `get_admin_log_filtered` | `pub fn get_admin_log_filtered( env: Env, action: Option<Symbol>, from_ledger: Option<u32>, to_ledger: Option<u32>, ) -> Vec<AuditEntry>` | Filtered view over the admin audit trail (Issue #419). All parameters are |
| `get_allowed_tokens` | `pub fn get_allowed_tokens(env: Env) -> Vec<Address>` | Returns the current token whitelist. An empty list means all tokens are |
| `get_amendment_log` | `pub fn get_amendment_log( env: Env, shipment_id: String, milestone_index: u32, ) -> Vec<AmendmentEntry>` | Returns the append-only amendment log for a milestone in chronological order. |
| `get_appeal_fee` | `pub fn get_appeal_fee(env: Env) -> i128` |  |
| `get_appeal_fee_deposit` | `pub fn get_appeal_fee_deposit( env: Env, shipment_id: String, milestone_index: u32, ) -> Option<(Address, i128)>` | Returns the escrowed appeal fee for a milestone as (appellant, amount), if any. |
| `get_appeal_window_ledgers` | `pub fn get_appeal_window_ledgers(env: Env) -> u32` |  |
| `get_approved_buyers` | `pub fn get_approved_buyers(env: Env, supplier: Address) -> Vec<Address>` | Returns the buyers `supplier` has pre-approved. Read-only. |
| `get_arbiter_history_size` | `pub fn get_arbiter_history_size(env: Env) -> u32` | Returns the configured per-supplier arbiter history size (0 = disabled). |
| `get_arbiter_panel` | `pub fn get_arbiter_panel(env: Env, shipment_id: String) -> Vec<Address>` | Returns the arbiter panel for a shipment, in the order supplied at creation. |
| `get_arbiter_pool` | `pub fn get_arbiter_pool(env: Env) -> Vec<Address>` | Return the current arbiter pool (read-only). |
| `get_arbiter_slash_bps` | `pub fn get_arbiter_slash_bps(env: Env) -> u32` | Read the configured arbiter stake-slash bps (0 = financial slash disabled). |
| `get_arbiter_stake` | `pub fn get_arbiter_stake(env: Env, arbiter: Address) -> i128` | Read the locked stake amount for `arbiter` (0 if none). |
| `get_arbiter_stats` | `pub fn get_arbiter_stats(env: Env, arbiter: Address) -> ArbiterStats` |  |
| `get_archive_threshold` | `pub fn get_archive_threshold(env: Env) -> u32` |  |
| `get_archived_shipment` | `pub fn get_archived_shipment(env: Env, shipment_id: String) -> ArchivedShipment` |  |
| `get_auto_blacklist_rule` | `pub fn get_auto_blacklist_rule(env: Env) -> AutoBlacklistRule` |  |
| `get_auto_confirm_threshold` | `pub fn get_auto_confirm_threshold(env: Env) -> u32` |  |
| `get_blacklist_appeal` | `pub fn get_blacklist_appeal(env: Env, address: Address) -> Option<BlacklistAppeal>` | Read the current appeal (if any) filed by `address`. Read-only. |
| `get_buyer_allowed_tokens` | `pub fn get_buyer_allowed_tokens(env: Env, buyer: Address) -> Vec<Address>` | Returns the buyer-specific allowlist, or an empty list when no override is set. |
| `get_buyer_dispute_cap` | `pub fn get_buyer_dispute_cap(env: Env, buyer: Address) -> Option<u32>` | Returns the buyer's configured concurrent-dispute cap (None = uncapped). |
| `get_buyer_open_disputes` | `pub fn get_buyer_open_disputes(env: Env, buyer: Address) -> u32` | Returns how many disputes raised by `buyer` are currently open. |
| `get_buyer_reliability` | `pub fn get_buyer_reliability(env: Env, buyer: Address) -> BuyerReliability` | Get buyer reliability score. Read-only. |
| `get_buyer_spend` | `pub fn get_buyer_spend(env: Env, buyer: Address, token: Address) -> i128` | Cumulative gross amount released from escrow toward settlement on |
| `get_buyer_spending_limit` | `pub fn get_buyer_spending_limit(env: Env, buyer: Address) -> Option<(i128, u32)>` | Returns the buyer's configured (limit, window_ledgers), if any. |
| `get_buyer_spending_window_usage` | `pub fn get_buyer_spending_window_usage(env: Env, buyer: Address) -> i128` | Returns the buyer's committed amount in the current (unexpired) rolling window. |
| `get_clean_completion_rebate_bps` | `pub fn get_clean_completion_rebate_bps(env: Env) -> u32` | Read the configured clean-completion rebate bps (0 = disabled). |
| `get_co_buyer` | `pub fn get_co_buyer(env: Env, shipment_id: String) -> Option<Address>` | Returns the designated co-buyer for a shipment, if any. |
| `get_collateral_slash_bps` | `pub fn get_collateral_slash_bps(env: Env, shipment_id: String) -> u32` | Basis points of the shipment's remaining supplier collateral forfeited to |
| `get_collateral_token` | `pub fn get_collateral_token(env: Env, shipment_id: String) -> Address` | Read the collateral token address for a shipment (or shipment token if unset). |
| `get_completion_percentage` | `pub fn get_completion_percentage(env: Env, shipment_id: String) -> u32` | Returns how far a shipment has settled, as a whole-number percentage in `[0, 100]`. |
| `get_confirmation_cooldown` | `pub fn get_confirmation_cooldown(env: Env, shipment_id: String) -> u32` |  |
| `get_confirmation_webhooks` | `pub fn get_confirmation_webhooks(env: Env) -> Vec<Address>` | Returns the current confirmation webhook allowlist. Empty by default. |
| `get_contract_stats` | `pub fn get_contract_stats(env: Env) -> ContractStats` |  |
| `get_conversion_rate` | `pub fn get_conversion_rate(env: Env, from_token: Address, to_token: Address) -> Option<u32>` | Read the registered conversion rate (basis points) for a token pair, |
| `get_delegate` | `pub fn get_delegate(env: Env, shipment_id: String) -> Option<DelegateConfig>` | Returns the current confirmation delegate for a shipment, if one is registered. |
| `get_delivered_quantity` | `pub fn get_delivered_quantity(env: Env, shipment_id: String, milestone_index: u32) -> (u32, u32)` | Returns `(delivered, expected)` quantity for a milestone (expected 0 = not quantity-based). |
| `get_depeg_guard` | `pub fn get_depeg_guard(env: Env, token: Address) -> Option<DepegGuardConfig>` | Read the depeg guard config for a token. |
| `get_dispute_evidence` | `pub fn get_dispute_evidence( env: Env, shipment_id: String, milestone_index: u32, ) -> Vec<DisputeEvidence>` | Read-only getter returning all evidence entries for a disputed milestone, |
| `get_dispute_filing_window` | `pub fn get_dispute_filing_window(env: Env) -> u32` | Returns the configured dispute filing window after a missed deadline. |
| `get_dispute_history` | `pub fn get_dispute_history( env: Env, address: Address, cursor: Option<u32>, limit: u32, ) -> Vec<DisputeRecord>` | Paginated dispute history for `address` (as buyer or supplier). |
| `get_dispute_resolution_reason` | `pub fn get_dispute_resolution_reason( env: Env, shipment_id: String, milestone_index: u32, ) -> Option<ResolutionReason>` | Returns the reason recorded for the most recent resolution of a dispute. |
| `get_dispute_split` | `pub fn get_dispute_split(env: Env, shipment_id: String, milestone_index: u32) -> Option<u32>` | Supplier share (bps) of the latest split resolution on this milestone. |
| `get_dispute_votes` | `pub fn get_dispute_votes(env: Env, shipment_id: String, milestone_index: u32) -> Vec<DisputeVote>` | Returns the current votes for a panel dispute identified by shipment and milestone. |
| `get_emergency_freeze_proposal` | `pub fn get_emergency_freeze_proposal( env: Env, proposal_id: u64, ) -> Option<EmergencyFreezeProposal>` | Returns a pending emergency-freeze activation proposal, or None. |
| `get_emergency_unfreeze_proposal` | `pub fn get_emergency_unfreeze_proposal( env: Env, proposal_id: u64, ) -> Option<EmergencyFreezeProposal>` | Returns a pending emergency-unfreeze proposal, or None. |
| `get_escalation_threshold` | `pub fn get_escalation_threshold(env: Env) -> u32` |  |
| `get_escrow_balance` | `pub fn get_escrow_balance(env: Env, shipment_id: String) -> i128` |  |
| `get_evidence_count` | `pub fn get_evidence_count(env: Env, shipment_id: String, milestone_index: u32) -> u32` | Returns the current evidence count for a given milestone. |
| `get_fee_config` | `pub fn get_fee_config(env: Env) -> Option<FeeConfig>` |  |
| `get_fee_tier` | `pub fn get_fee_tier(env: Env, address: Address) -> u32` | Returns the effective fee bps for `address` based on lifetime volume. |
| `get_fee_waiver` | `pub fn get_fee_waiver(env: Env, partner: Address) -> Option<(u32, u64)>` | Returns the active fee waiver for `partner` as (waiver_bps, expires_at), |
| `get_fee_waiver_proposal` | `pub fn get_fee_waiver_proposal(env: Env, proposal_id: u64) -> Option<FeeWaiverProposal>` | Returns a pending fee-waiver proposal, or None if it never existed or |
| `get_finality_delay_ledgers` | `pub fn get_finality_delay_ledgers(env: Env) -> u32` |  |
| `get_freeze_supermajority_bps` | `pub fn get_freeze_supermajority_bps(env: Env) -> u32` | Read the configured emergency-freeze supermajority (basis points). |
| `get_incremental_collateral` | `pub fn get_incremental_collateral(env: Env, shipment_id: String) -> bool` | Whether incremental collateral release is enabled for a shipment. |
| `get_inspection` | `pub fn get_inspection( env: Env, shipment_id: String, milestone_index: u32, ) -> Option<InspectionRecord>` | Returns the inspection record for a milestone, if any. |
| `get_invoice_hash` | `pub fn get_invoice_hash( env: Env, shipment_id: String, milestone_index: u32, ) -> Option<BytesN<32>>` | Returns the invoice hash for a milestone, or None if not set. |
| `get_joint_confirmation_status` | `pub fn get_joint_confirmation_status( env: Env, shipment_id: String, milestone_index: u32, ) -> JointConfirmationStatus` | Returns the partial joint-confirmation progress for a milestone. |
| `get_joint_confirmation_threshold` | `pub fn get_joint_confirmation_threshold(env: Env) -> i128` |  |
| `get_locked_collateral` | `pub fn get_locked_collateral(env: Env, shipment_id: String) -> i128` | Supplier collateral still locked for a shipment. |
| `get_long_hold_rebate` | `pub fn get_long_hold_rebate(env: Env) -> (u32, u32)` | Read the current long-hold escrow rebate configuration (read-only). |
| `get_loser_pays_arbiter_fee` | `pub fn get_loser_pays_arbiter_fee(env: Env, shipment_id: String) -> bool` | Whether loser-pays arbiter fee allocation is enabled for a shipment. |
| `get_max_advance_percent` | `pub fn get_max_advance_percent(env: Env) -> u32` |  |
| `get_max_allowed_tokens` | `pub fn get_max_allowed_tokens(env: Env) -> u32` | #387: Read the currently configured maximum allowed-token list size |
| `get_max_dispute_bond_bps` | `pub fn get_max_dispute_bond_bps(env: Env) -> u32` |  |
| `get_max_expiry_extension_ledgers` | `pub fn get_max_expiry_extension_ledgers(env: Env) -> u32` | Returns the configured maximum total expiry extension in ledgers |
| `get_max_extension_requests` | `pub fn get_max_extension_requests(env: Env) -> u32` | Read the configured max extension requests per milestone; `0` means no cap. |
| `get_max_milestone_count` | `pub fn get_max_milestone_count(env: Env) -> u32` | Read the currently configured max milestone count (falls back to |
| `get_max_overturned_before_slash` | `pub fn get_max_overturned_before_slash(env: Env) -> u32` | Read the configured overturned-resolution slash threshold (0 = disabled). |
| `get_max_shipment_value` | `pub fn get_max_shipment_value(env: Env) -> i128` |  |
| `get_max_summary_batch` | `pub fn get_max_summary_batch(env: Env) -> u32` | Read the admin-configured `get_shipment_summaries` batch cap |
| `get_mediation_proposal` | `pub fn get_mediation_proposal( env: Env, shipment_id: String, milestone_index: u32, ) -> Option<MediationProposal>` |  |
| `get_mediator_pool` | `pub fn get_mediator_pool(env: Env) -> Vec<Address>` |  |
| `get_milestone` | `pub fn get_milestone(env: Env, shipment_id: String, milestone_index: u32) -> Milestone` |  |
| `get_milestone_deadline` | `pub fn get_milestone_deadline(env: Env, shipment_id: String, milestone_index: u32) -> u32` | Returns the effective deadline ledger for a milestone (0 = not set). |
| `get_milestone_grade` | `pub fn get_milestone_grade(env: Env, shipment_id: String, milestone_index: u32) -> Option<u32>` | Grade recorded for a milestone by `confirm_milestone_graded` (None = not graded). |
| `get_milestone_merge_proposal` | `pub fn get_milestone_merge_proposal( env: Env, shipment_id: String, ) -> Option<MilestoneMergeProposal>` | Returns the pending milestone merge proposal for a shipment, if any. |
| `get_milestone_notes` | `pub fn get_milestone_notes( env: Env, shipment_id: String, milestone_index: u32, ) -> Vec<MilestoneNote>` |  |
| `get_milestone_payees` | `pub fn get_milestone_payees( env: Env, shipment_id: String, milestone_index: u32, ) -> Vec<MilestonePayee>` | Returns the configured payees for a milestone. |
| `get_milestone_proof_type` | `pub fn get_milestone_proof_type( env: Env, shipment_id: String, milestone_index: u32, ) -> Option<Symbol>` | Returns the submitted proof content type for a milestone, or None if not yet submitted. |
| `get_milestone_split_proposal` | `pub fn get_milestone_split_proposal( env: Env, shipment_id: String, ) -> Option<MilestoneSplitProposal>` | Returns the pending split proposal for a shipment, if any. |
| `get_milestone_supplier` | `pub fn get_milestone_supplier( env: Env, shipment_id: String, milestone_index: u32, ) -> Option<Address>` | Read the approved substitute supplier for a milestone, if any. |
| `get_milestone_template` | `pub fn get_milestone_template(env: Env, creator: Address, name: String) -> Vec<Milestone>` | Fetch a saved milestone template by (creator, name). Panics with |
| `get_min_shipment_value` | `pub fn get_min_shipment_value(env: Env) -> i128` |  |
| `get_new_milestone_proposal` | `pub fn get_new_milestone_proposal( env: Env, shipment_id: String, ) -> Option<NewMilestoneProposal>` | Pending new-milestone proposal for a shipment, if any. Read-only. |
| `get_nft_hook_enabled` | `pub fn get_nft_hook_enabled(env: Env) -> bool` | Returns true if the NFT mint hook event is currently enabled. |
| `get_oracle_attestation_count` | `pub fn get_oracle_attestation_count( env: Env, shipment_id: String, milestone_index: u32, ) -> u32` |  |
| `get_oracle_group` | `pub fn get_oracle_group(env: Env, purpose: Symbol) -> Option<(Vec<Address>, u32)>` |  |
| `get_panel_escalation` | `pub fn get_panel_escalation(env: Env, token: Address) -> Option<(i128, u32)>` | Returns `(threshold, panel_size)` for `token`, or `None` if disabled. |
| `get_pause_min_notice_ledgers` | `pub fn get_pause_min_notice_ledgers(env: Env) -> u32` | Returns the configured minimum pause notice in ledgers (0 = no minimum). |
| `get_payout_assignment` | `pub fn get_payout_assignment(env: Env, shipment_id: String) -> Option<PayoutAssignment>` | Active payout assignment for a shipment, if any. |
| `get_payout_currency_preference` | `pub fn get_payout_currency_preference(env: Env, supplier: Address) -> Option<Address>` | Read the supplier's configured payout currency preference, if any. |
| `get_pending_admin_actions` | `pub fn get_pending_admin_actions(env: Env, action_id: String) -> Vec<Address>` |  |
| `get_pending_expiry_extension` | `pub fn get_pending_expiry_extension(env: Env, shipment_id: String) -> Option<ExpiryExtensionProposal>` | Returns the pending expiry extension for a shipment, if any. |
| `get_pending_payout` | `pub fn get_pending_payout(env: Env, supplier: Address) -> i128` | Read-only getter for the supplier's accumulated pending payout balance. |
| `get_pending_recovery` | `pub fn get_pending_recovery(env: Env, shipment_id: String) -> Option<RecoveryProposal>` | Returns the pending recovery proposal for a shipment, if one exists. |
| `get_proof_hash_length_bounds` | `pub fn get_proof_hash_length_bounds(env: Env) -> (u32, u32)` | Returns the configured (min_len, max_len) proof-hash length bounds |
| `get_proof_hash_required_prefix` | `pub fn get_proof_hash_required_prefix(env: Env) -> String` | Returns the configured required proof-hash prefix (empty = none). |
| `get_proof_submitter` | `pub fn get_proof_submitter(env: Env, shipment_id: String, milestone_index: u32) -> Address` | Returns the address allowed to submit proof for `milestone_index` |
| `get_proof_whitelist` | `pub fn get_proof_whitelist(env: Env, shipment_id: String, milestone_index: u32) -> Vec<Symbol>` | Returns the proof content-type whitelist for a milestone. |
| `get_quality_grades` | `pub fn get_quality_grades(env: Env, shipment_id: String) -> Vec<u32>` | Payout basis points per grade index configured for the shipment (empty = none). |
| `get_rating_summary` | `pub fn get_rating_summary(env: Env, address: Address) -> (u32, u32)` | Returns `(count, average_x100)` for `address`, where `average_x100` is |
| `get_rating_window_ledgers` | `pub fn get_rating_window_ledgers(env: Env) -> u32` | Read the admin-configured rating window in ledgers (`0` = ratings disabled). |
| `get_referral_fee_bps` | `pub fn get_referral_fee_bps(env: Env) -> u32` |  |
| `get_refund_recipient` | `pub fn get_refund_recipient(env: Env, shipment_id: String) -> Option<Address>` | Return the configured refund recipient for a shipment, or None if not set. |
| `get_refund_sweep_window` | `pub fn get_refund_sweep_window(env: Env) -> u32` |  |
| `get_reputation` | `pub fn get_reputation(env: Env, supplier: Address) -> ReputationScore` |  |
| `get_require_mutual_preapproval` | `pub fn get_require_mutual_preapproval(env: Env) -> bool` | Returns true if the mutual buyer+supplier pre-approval gate is currently enabled. |
| `get_retainage_balance` | `pub fn get_retainage_balance(env: Env, shipment_id: String) -> i128` | Retainage withheld so far and not yet released to the supplier or refunded. |
| `get_role_expiry` | `pub fn get_role_expiry(env: Env, role: Role, address: Address) -> Option<u32>` | #633: Expiry ledger of a role grant, or None when there is no grant or |
| `get_shipment` | `pub fn get_shipment(env: Env, shipment_id: String) -> Shipment` |  |
| `get_shipment_count` | `pub fn get_shipment_count(env: Env, address: Address) -> u32` | Returns the total number of shipments associated with `address` as buyer or supplier. |
| `get_shipment_creation_rate_limit` | `pub fn get_shipment_creation_rate_limit(env: Env) -> Option<(u32, u32)>` | Returns the configured (max_shipments, window_ledgers), if any. |
| `get_shipment_fee_override` | `pub fn get_shipment_fee_override(env: Env, shipment_id: String) -> Option<u32>` |  |
| `get_shipment_fees_paid` | `pub fn get_shipment_fees_paid(env: Env, shipment_id: String) -> i128` | Cumulative platform fees collected for `shipment_id` (read-only). |
| `get_shipment_jurisdiction` | `pub fn get_shipment_jurisdiction(env: Env, shipment_id: String) -> Option<Symbol>` | #385: Returns the jurisdiction/compliance tag for a shipment, or None if untagged. |
| `get_shipment_mediator` | `pub fn get_shipment_mediator(env: Env, shipment_id: String) -> Option<Address>` |  |
| `get_shipment_metadata` | `pub fn get_shipment_metadata(env: Env, shipment_id: String, key: Symbol) -> Option<String>` | Read a single custom metadata value for a shipment. Read-only. |
| `get_shipment_metadata_keys` | `pub fn get_shipment_metadata_keys(env: Env, shipment_id: String) -> Vec<Symbol>` | List all custom metadata keys set on a shipment. Read-only. |
| `get_shipment_observers` | `pub fn get_shipment_observers(env: Env, shipment_id: String) -> Vec<Address>` | Return the list of read-only observers for a shipment. |
| `get_shipment_risk` | `pub fn get_shipment_risk(env: Env, shipment_id: String) -> ShipmentRisk` |  |
| `get_shipment_summaries` | `pub fn get_shipment_summaries(env: Env, ids: Vec<String>) -> Vec<ShipmentSummary>` | Batch variant of `get_shipment_summary`. Unknown IDs are silently |
| `get_shipment_summary` | `pub fn get_shipment_summary(env: Env, shipment_id: String) -> ShipmentSummary` | Read-only, no authorization required (matches `get_shipment`). Returns a |
| `get_shipments_by_arbiter` | `pub fn get_shipments_by_arbiter( env: Env, arbiter: Address, cursor: Option<u32>, limit: u32, ) -> Vec<String>` | #572: Paginated list of shipment IDs currently assigned to `arbiter`. |
| `get_shipments_by_buyer` | `pub fn get_shipments_by_buyer(env: Env, buyer: Address) -> Vec<String>` |  |
| `get_shipments_by_jurisdiction` | `pub fn get_shipments_by_jurisdiction(env: Env, jurisdiction: Symbol) -> Vec<String>` | #385: Returns all shipment IDs tagged with `jurisdiction`, for off-chain |
| `get_shipments_by_logistics` | `pub fn get_shipments_by_logistics( env: Env, logistics: Address, cursor: Option<u32>, limit: u32, ) -> Vec<String>` | #573: Paginated list of shipment IDs involving `logistics`. |
| `get_shipments_by_supplier` | `pub fn get_shipments_by_supplier(env: Env, supplier: Address) -> Vec<String>` |  |
| `get_standing_order` | `pub fn get_standing_order(env: Env, order_id: u64) -> StandingOrder` |  |
| `get_supplier_cancel_cooldown` | `pub fn get_supplier_cancel_cooldown( env: Env, supplier: Address, ) -> Option<(u32, u32, u32)>` | Returns the supplier's configured (max_cancellations, window_ledgers, |
| `get_supplier_earnings` | `pub fn get_supplier_earnings(env: Env, supplier: Address, token: Address) -> i128` | Cumulative post-fee amount actually paid to `supplier` in `token` across |
| `get_supplier_exposure` | `pub fn get_supplier_exposure(env: Env, supplier: Address) -> i128` | Returns the current aggregate locked escrow for `supplier` across all Active shipments. |
| `get_supplier_exposure_cap` | `pub fn get_supplier_exposure_cap(env: Env) -> i128` |  |
| `get_supplier_recent_arbiters` | `pub fn get_supplier_recent_arbiters(env: Env, supplier: Address) -> Vec<Address>` | Recent pool-assigned arbiters for `supplier`'s disputes, oldest first. |
| `get_supplier_terms_profile` | `pub fn get_supplier_terms_profile(env: Env, supplier: Address) -> Option<SupplierTermsProfile>` | Get the supplier's default terms profile if set. |
| `get_supplier_tier` | `pub fn get_supplier_tier(env: Env, supplier: Address) -> SupplierTier` | Derives a supplier's current tier from their reputation score against the |
| `get_supplier_tier_config` | `pub fn get_supplier_tier_config(env: Env) -> Option<SupplierTierConfig>` |  |
| `get_token_max_shipment_value` | `pub fn get_token_max_shipment_value(env: Env, token: Address) -> Option<i128>` | Returns the per-token maximum override, or `None` if the token falls |
| `get_token_min_shipment_value` | `pub fn get_token_min_shipment_value(env: Env, token: Address) -> Option<i128>` | Returns the per-token minimum override, or `None` if the token falls |
| `get_total_escrowed_value` | `pub fn get_total_escrowed_value(env: Env, token: Address) -> i128` |  |
| `get_total_expiry_extended` | `pub fn get_total_expiry_extended(env: Env, shipment_id: String) -> u32` | Returns the total ledgers a shipment's expiry has been extended by. |
| `get_treasury_revenue` | `pub fn get_treasury_revenue(env: Env, token: Address) -> i128` | Get treasury revenue collected for a specific token. Read-only. |
| `get_tvl` | `pub fn get_tvl(env: Env, token: Address) -> i128` | Read the current total escrowed value (TVL) for a token. |
| `get_tvl_cap` | `pub fn get_tvl_cap(env: Env, token: Address) -> i128` | Read the TVL cap for a token (0 = uncapped). |
| `get_upcoming_deadlines` | `pub fn get_upcoming_deadlines( env: Env, address: Address, within_ledgers: u32, limit: u32, ) -> Vec<(String, u32, u32)>` | #574: Upcoming milestone deadlines for `address` as buyer or supplier. |
| `get_upgrade_proposal` | `pub fn get_upgrade_proposal(env: Env, proposal_id: u64) -> Option<UpgradeProposal>` | Returns a pending upgrade proposal, or None if it doesn't exist (never |
| `get_vault_balance` | `pub fn get_vault_balance(env: Env, buyer: Address, token: Address) -> i128` | Read-only vault balance for (buyer, token). |
| `get_warning_lead_ledgers` | `pub fn get_warning_lead_ledgers(env: Env) -> u32` |  |
| `get_warranty_balance` | `pub fn get_warranty_balance(env: Env, shipment_id: String) -> i128` | Warranty holdback currently in escrow for a shipment. |
| `get_warranty_claim` | `pub fn get_warranty_claim(env: Env, shipment_id: String) -> Option<WarrantyClaim>` | Open warranty claim for a shipment, if any. |
| `get_warranty_end_ledger` | `pub fn get_warranty_end_ledger(env: Env, shipment_id: String) -> u32` | Ledger at which the warranty period ends (0 = warranty period not started). |
| `has_role` | `pub fn has_role(env: Env, role: Role, address: Address) -> bool` | Returns true if `address` holds `role` (or is the primary contract admin). |
| `is_arbiter_available` | `pub fn is_arbiter_available(env: Env, arbiter: Address) -> bool` | Whether `arbiter` is currently accepting new assignments (default true). |
| `is_arbiter_slashed` | `pub fn is_arbiter_slashed(env: Env, arbiter: Address) -> bool` | Whether the given arbiter is currently slashed. |
| `is_blacklisted` | `pub fn is_blacklisted(env: Env, address: Address) -> bool` |  |
| `is_collateral_slashed` | `pub fn is_collateral_slashed(env: Env, shipment_id: String, milestone_index: u32) -> bool` | Whether (shipment_id, milestone_index) has already been slashed for a |
| `is_consortium_shipment` | `pub fn is_consortium_shipment(env: Env, shipment_id: String) -> bool` | Returns true when this shipment was created with per-milestone consortium suppliers. |
| `is_dispute_panel_escalated` | `pub fn is_dispute_panel_escalated(env: Env, shipment_id: String, milestone_index: u32) -> bool` | Whether the open dispute on this milestone was escalated to a panel. |
| `is_emergency_frozen` | `pub fn is_emergency_frozen(env: Env) -> bool` | Whether the contract is currently under an emergency freeze. |
| `is_fast_track_eligible` | `pub fn is_fast_track_eligible(env: Env, supplier: Address) -> bool` | Whether a supplier currently qualifies for proof-review fast-track. |
| `is_fee_holiday_active` | `pub fn is_fee_holiday_active(env: Env) -> bool` | #413: Whether a fee holiday is currently active. Read-only. |
| `is_metadata_key_indexed` | `pub fn is_metadata_key_indexed(env: Env, key: Symbol) -> bool` | #662: Whether a metadata key is currently indexed. Read-only. |
| `is_on_compliance_hold` | `pub fn is_on_compliance_hold(env: Env, shipment_id: String) -> bool` | Read-only check for whether a shipment is currently on compliance hold. |
| `is_operation_paused` | `pub fn is_operation_paused(env: Env, op: Symbol) -> bool` | Returns whether a specific operation is paused. |
| `is_paused` | `pub fn is_paused(env: Env) -> bool` |  |
| `is_rate_limit_exempt` | `pub fn is_rate_limit_exempt(env: Env, supplier: Address) -> bool` | Whether `supplier` is on the rate-limit exemption list. |
| `is_shipment_paused` | `pub fn is_shipment_paused(env: Env, shipment_id: String) -> bool` | Whether a specific shipment is mutually-consent paused. |
| `is_token_depegged` | `pub fn is_token_depegged(env: Env, token: Address) -> bool` | Check whether a token is depegged based on its configured oracle guard. |
| `is_whitelisted` | `pub fn is_whitelisted(env: Env, address: Address) -> bool` | Returns true if `address` is on the supplier whitelist, or the whitelist is empty (open mode). |
| `list_milestone_templates` | `pub fn list_milestone_templates(env: Env, creator: Address) -> Vec<String>` | List the names of all milestone templates saved by `creator`. |
| `list_shipments` | `pub fn list_shipments( env: Env, cursor: Option<u32>, limit: u32, status_filter: Option<ShipmentStatus>, ) -> (Vec<String>, Option<u32>)` |  |
| `preview_milestone_payout` | `pub fn preview_milestone_payout( env: Env, shipment_id: String, milestone_index: u32, ) -> PayoutPreview` | Simulates exactly what `confirm_milestone` would pay out for |
| `storage_schema_version` | `pub fn storage_schema_version(env: Env) -> u32` | #571: Storage schema version from instance storage (set by `migrate`). |
| `version` | `pub fn version(_env: Env) -> (u32, u32, u32)` | #571: Semantic contract version `(major, minor, patch)` matching Cargo.toml. |
