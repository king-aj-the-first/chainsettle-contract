# Compliance controls

Every compliance lever the contract offers: what it does, who operates it,
and how it affects live shipments. Source of truth:
`contracts/chainsetttle/src/lib.rs`.

## Blacklist

`blacklist_address(env, admin, address, reason_hash)` (admin-only) flags an
address in `DataKey::Blacklisted`. `remove_from_blacklist(env, admin, address)`
clears it; `is_blacklisted(env, address) -> bool` is the read-only check.
Blacklisted suppliers cannot create or service new shipments, and the buyer of
an affected live shipment can exit via `cancel_for_blacklisted_supplier`
(buyer-authenticated; works like `cancel_shipment` for that shipment).

`set_auto_blacklist_rule(env, admin, max_cancelled, max_disputed)` arms an
automatic rule (`get_auto_blacklist_rule` reads it back): suppliers whose
cancelled or disputed counts exceed the thresholds are blacklisted without a
separate admin call. Either threshold set to `0` disables that leg.

## Blacklist appeals

A blacklisted address can contest its listing without admin help:

1. `appeal_blacklist(env, address, evidence_hash)` — the listed address itself
   authenticates and files an appeal carrying an off-chain evidence pointer.
   Panics with `"address is not blacklisted"` when there is nothing to appeal.
2. `get_blacklist_appeal(env, address) -> Option<BlacklistAppeal>` — read-only
   view of the pending/decided appeal.
3. `review_blacklist_appeal(env, admin, address, approve)` — decided by a
   `Role::ComplianceOfficer`, not a generic admin. Approving clears the
   blacklist entry; rejecting leaves it in place. The decision is recorded in
   the appeal record.

## Whitelist and pre-approval

- `add_to_whitelist` / `remove_from_whitelist` (admin-only) and
  `is_whitelisted` manage an allowlist checked at shipment creation when the
  whitelist gate is enabled.
- Mutual pre-approval is per-supplier: `add_approved_buyer` /
  `remove_approved_buyer` (supplier-authenticated) maintain the supplier's
  buyer allowlist, `get_approved_buyers` reads it, and
  `set_require_mutual_preapproval` (admin) /
  `get_require_mutual_preapproval` toggle whether creation requires the buyer
  to be pre-approved. When enabled, shipments with unapproved buyers are
  rejected at creation.

## KYC attestations

`DataKeyExt2::BuyerKycHash(Address)` and `SupplierKycHash(Address)` reserve
storage for 32-byte KYC attestation hashes per buyer and supplier. They are
storage keys only: no public setter/getter exists yet, so enforcement of KYC
happens off-chain until accessors are added. Do not assume on-chain KYC
gating from the presence of these keys.

## Jurisdiction tags

The buyer/supplier can tag a shipment at creation via
`ShipmentOptions.jurisdiction: Option<Symbol>` (e.g. `"US"`, `"EU_MIFID"`).
The contract stores it under `DataKeyExt2::ShipmentJurisdiction(shipment_id)`
and indexes it in `DataKeyExt2::JurisdictionShipments(tag)`:

- `get_shipment_jurisdiction(env, shipment_id) -> Option<Symbol>`
- `get_shipments_by_jurisdiction(env, jurisdiction) -> Vec<String>`

Tags are informational: they do not change settlement logic by themselves,
but off-chain compliance tooling and the `is_on_compliance_hold` gate below
can key off them.

## Compliance holds

`set_compliance_hold(env, admin, shipment_id, reason_hash)` (admin-only)
freezes a shipment in place; `clear_compliance_hold(env, admin, shipment_id)`
lifts it; `is_on_compliance_hold(env, shipment_id) -> bool` is the read-only
check. While a hold is set, `assert_shipment_not_on_hold` — called from
`cancel_shipment_internal` and the other state-changing paths — panics with
`"shipment is on compliance hold pending review"`, so funds cannot move until
the hold is cleared.

## Dispute appeals and finality

- `set_appeal_window_ledgers` / `get_appeal_window_ledgers`: ledgers after a
  resolution during which either party may call `appeal_dispute` (`0` =
  disabled). Only one appeal per milestone is allowed (`DisputeAppealed`
  guard); a second resolution is final.
- `set_appeal_fee` / `get_appeal_fee` / `get_appeal_fee_deposit`: the appeal
  fee, refunded if the appeal succeeds and paid to the appeal arbiter
  otherwise.
- `set_finality_delay_ledgers` / `get_finality_delay_ledgers`: delay before a
  resolution becomes final, giving compliance reviewers a window to intervene.

## Effect on active shipments

| Control | Operator | Effect while active |
| --- | --- | --- |
| Blacklist entry | admin / auto-rule | supplier blocked from new work; buyer may `cancel_for_blacklisted_supplier` |
| Blacklist appeal pending | listed address files; `ComplianceOfficer` reviews | listing stays enforced until approval |
| Whitelist gate / mutual pre-approval | admin / supplier | creation rejected for unlisted parties |
| Compliance hold | admin | all state changes on the shipment panic until cleared |
| Jurisdiction tag | set at creation | informational; feeds off-chain filtering |
| Appeal window / finality delay | admin-configured | resolutions stay contestable until the window lapses |
