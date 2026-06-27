# unifi-discovery

- Parse leniently, never panic on malformed input. Emit `warn!` for unexpected data (non-zero reserved bytes, trailing
  bytes, oversized fields) and continue processing.
- Preserve all parsed data including unknown fields. No silent data loss - unknown TLV types stay in the bag and
  round-trip through encode.
- Separate parsing from interpretation. Raw frame structs (`Frame`, `TlvValues`) hold bytes without domain knowledge.
  Interpretation structs (`DiscoveryMessage`, `DeviceInfo`) own the typed accessors and command-to-variant mapping.
- Typed accessors return `Result<Option<T>, Error>` to distinguish absent, present+valid, and present+malformed values.
  Never silently default a malformed field.
- Verify byte-exact round-trips against captured frames in tests. `parse -> encode` must produce the original bytes for
  any spec-compliant input.
- Keep protocol constants (header lengths, type codes, command bytes) close to their consumers, not in a central public
  module. Export only what external callers need.
- Implement both sides of each protocol (controller and device) in the same crate. The CLI crate exercises both sides as
  an integration testing ground.
