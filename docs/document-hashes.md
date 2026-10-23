# Trade-document uniqueness registry

Prevents the same trade document from financing two shipments: a
bill-of-lading or invoice hash registered on one active shipment cannot be
attached on another until the owning shipment ends. Source of truth:
`contracts/chainsetttle/src/lib.rs` (#617).

## Storage

- `DataKeyExt4::DocHashOwner(BytesN<32>) -> String`: global registry mapping
  each registered document hash to its owning `shipment_id`. Invoice and BOL
  hashes share this namespace, so a collision across types is also rejected.
- `DataKeyExt4::ShipmentBolHash(shipment_id, milestone_index)`: the
  per-milestone bill-of-lading hash, stored the same way `CancelPolicy` is
  kept outside the `Shipment` struct (the `Shipment`/`ShipmentOptions` types
  are at their field limits).
- Invoice hashes keep their existing home,
  `DataKeyExt::MilestoneInvoiceHash(shipment_id, milestone_index)`.

Registry entries are written with
`extend_ttl(TTL_INITIAL_LEDGERS, TTL_MAX_LEDGERS)` and are bumped by
`extend_shipment_ttl` alongside the milestone keys (see
`docs/storage-ttl.md`).

## Functions

- `attach_invoice_hash(env, caller, shipment_id, milestone_index, invoice_hash)`:
  supplier-only (`require_supplier_auth`, shipment must be `Active`, index
  valid, proof already submitted, hash not already set on the milestone).
  Registers the hash, records an `invoice_hash_attached` audit entry on the
  shipment, and publishes an `invoice_hash_attached` event. Panics with
  `"document hash already used"` if another shipment owns the hash.
- `attach_bol_hash(env, caller, shipment_id, milestone_index, bol_hash)`:
  same rules and behaviour for bill-of-lading hashes; event
  `bol_hash_attached`, audit action `bol_hash_attached`. Immutable once set
  (`"bol hash already set and is immutable"`).
- `get_bol_hash(env, shipment_id, milestone_index) -> Option<BytesN<32>>` and
  the existing `get_invoice_hash`: read-only accessors, `None` when unset.
- `get_document_hash_owner(env, doc_hash) -> Option<String>`: read-only
  lookup of the current owner, `None` when the hash is free to use.

## Release

`release_shipment_doc_hashes` removes the registry entries for every
milestone invoice/BOL hash of a shipment. It runs when a shipment is
cancelled (buyer `cancel_shipment` / `batch_cancel_shipments` via
`cancel_shipment_internal`, and supplier `supplier_cancel`) and when it is
compacted by `archive_shipment`. After release the same hashes can finance a
new shipment. Completed shipments that are neither cancelled nor archived keep
their registrations until archival.
