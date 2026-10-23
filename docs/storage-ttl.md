# Storage TTL and archival

How Soroban storage lifetimes work in this contract, which keys
`extend_shipment_ttl` keeps alive, and what archival means for old shipments.
Source of truth: `contracts/chainsetttle/src/lib.rs` and
`contracts/chainsetttle/src/constants.rs`.

## TTL constants

| Constant | Value | Meaning |
| --- | --- | --- |
| `TTL_INITIAL_LEDGERS` | 100,000 (~1 day at 5s/ledger) | TTL given to freshly written persistent entries via `extend_ttl(TTL_INITIAL_LEDGERS, TTL_MAX_LEDGERS)` |
| `TTL_MAX_LEDGERS` | 6,300,000 (~1 year) | Ceiling every persistent entry is topped up to |
| `RECOVERY_THRESHOLD_LEDGERS` | 12,614,400 (~2 years) | Age after which emergency recovery is allowed (not a storage TTL) |

Soroban evicts a persistent entry once its TTL expires, so any shipment that
is idle for long enough loses its satellite keys first and eventually the
`DataKey::Shipment` entry itself. Reads then panic with `"shipment not found"`.

## Which entries are bumped on write

Every state-changing write sets its key and immediately calls
`extend_ttl(TTL_INITIAL_LEDGERS, TTL_MAX_LEDGERS)`. Reads that go through
`get_shipment` / `get_shipment_summary` also bump the **instance** TTL only;
they do not extend persistent shipment keys. The one exception is
`extend_shipment_ttl` below, which is the supported keep-alive.

## `extend_shipment_ttl`

```rust
pub fn extend_shipment_ttl(env: Env, shipment_id: String)
```

- **Permissionless**: anyone may call it for any existing shipment. It has no
  side effects on shipment state.
- Panics with `"shipment not found"` if the shipment does not exist.
- Bumps instance TTL, then tops the `DataKey::Shipment` entry up to
  `TTL_MAX_LEDGERS` (threshold and max both `TTL_MAX_LEDGERS`, so it always
  refills to the maximum), then bumps every related key listed below.
- Publishes a `shipment_ttl_extended` event keyed by shipment ID.

Call it periodically (e.g. from an off-chain cron or the frontend) for every
shipment that must stay callable, especially long-running shipments with
far-future deadlines.

## Keys covered by `extend_shipment_ttl`

The exact list lives in `extend_shipment_related_keys_ttl`. Each key is only
bumped `if` it exists (`has` check first); per-milestone keys loop over all
`shipment.milestones`. Shipment-level keys:

- `DataKey`: `Shipment` (handled directly, always topped to max),
  `CancelPolicy`, `SupplierCollateral`, `PendingRecovery`,
  `ConfirmationDelegate`
- `DataKeyExt`: `ShipmentFeeBps`, `MilestoneSplits`,
  `MilestoneTimestampDeadlines`, `BackupArbiter`,
  `ShipmentConfirmationCooldown`, `DisputeDecayStartLedger`,
  `ShipmentFeeOverride`, `ArbiterPanel`, `ShipmentPaused`,
  `ShipmentPauseRequest`, `ShipmentPausedAt`, `ArchivedShipment`
- `DataKeyExt2`: `CoBuyer`, `ComplianceHold`, `ShipmentMediator`,
  `ShipmentJurisdiction`, `ShipmentObservers`
- `DataKeyExt3`: `ShipmentMetadataKeys` plus each
  `ShipmentMetadata(shipment_id, key)`, `QualityGrades`,
  `MilestoneQuantities`, `RetainageBps`, `RetainageBalance`,
  `WarrantyConfig`, `WarrantyBalance`, `WarrantyEndsAt`, `WarrantyClaim`,
  `IncrementalCollateral`, `IncrementalCollateralBase`,
  `MilestoneMergeProposal`, `MilestoneSplitProposal`, `ShipmentInspector`,
  `InspectedMilestones`, `ProofSubmitters`, `RequireDualAttestation`
- `DataKeyExt4`: `ShipmentFeesPaid`, `ShipmentHadDispute`,
  `PendingExpiryExtension`, `TotalExpiryExtended`, `NewMilestoneProposal`

Per-milestone keys (`(shipment_id, index)`), plus per-purpose
`OracleAttestations(shipment_id, index, purpose)` when the shipment has an
oracle purpose:

- `DataKey`: `ProofSubmittedAt`, `AdvanceRequest`,
  `MilestoneProofWhitelist`, `SubmittedProofType`, `DisputeContestedPercent`,
  `EvidenceCount`
- `DataKeyExt`: `MilestoneInvoiceHash`, `AmendmentLog`, `ExtensionRequest`,
  `MilestoneDeadline`, `DisputeOpenedAt`, `EffectiveDisputeBond`,
  `DisputeVotes`, `MilestonePayees`, `ProofSubmitter`, `MilestoneNotes`,
  `DisputeEvidence`
- `DataKeyExt2`: `DeadlineWarningFired`, `JointConfirmation`,
  `DisputeResolvedAtLedger`, `DisputeAppealed`, `MediationProposal`,
  `DisputeAppealOriginal`, `DisputeResolvedApprove`,
  `DisputeResolutionReason`, `ExtensionRequestCount`
- `DataKeyExt3`: `RefundClaimableAtLedger`, `MilestoneGrade`, `PendingGrade`,
  `GradeDispute`, `DeliveredQuantity`, `PartialQtyReleased`, `DisputeRaisedBy`,
  `InspectionReport`, `PendingDualProof`, `ConditionBreachReports`
- `DataKeyExt4`: `SubstituteProposal`, `MilestoneSupplier`, `DisputeSplitBps`,
  `PendingSplit`, `PanelEscalated`

Global/singleton keys (admin config, indexes, token totals) are **not**
covered; they are rewritten on every use and carry their own TTLs.

## Archival

Admin-gated compaction for finished shipments:

- `set_archive_threshold(env, admin, ledgers)` / `get_archive_threshold(env)`:
  minimum shipment age for archival. `0` (default) means archival is disabled.
- `archive_shipment(env, caller, shipment_id)`: anyone authenticated may call
  it once the conditions hold. It panics with `"archival is disabled"`,
  `"shipment already archived"`,
  `"only completed or cancelled shipments can be archived"`,
  `"shipment not old enough to archive"`, or
  `"warranty holdback still in escrow"`. On success it writes a compact
  `ArchivedShipment` record (`id`, `buyer`, `supplier`, `status`,
  `total_amount`, `released_amount`, `completed_at`), deletes the live
  `DataKey::Shipment` entry and the `CancelPolicy` satellite, and publishes
  `shipment_archived`.
- `get_archived_shipment(env, shipment_id) -> ArchivedShipment`: read-only
  accessor for the compact record; panics with
  `"archived shipment not found"` when absent.

There is **no restore entry point**: archival is one-way, and the full
milestone/audit state is not recoverable afterwards. Keep a shipment alive
with `extend_shipment_ttl` instead of relying on post-archival reads, and
export anything you need (via the `get_*` queries in `docs/queries.md`)
before archiving.
