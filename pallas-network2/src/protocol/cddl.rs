//! Shared test scaffolding for exercising mini-protocol codecs against the
//! vendored cardano-blueprint CDDL schemas.
//!
//! Today this hosts CBOR-vs-CDDL *conformance* checks; it is also where the
//! upcoming round-trip helpers will live, so codec tests across protocols share
//! one home rather than re-deriving the same plumbing.
//!
//! A protocol's conformance test needs only two protocol-specific things: a
//! `self_contained()` that turns its blueprint `messages.cddl` into a schema
//! `cddl-rs` can parse, and a table of [`conforms!`] cases (one per message).
//! Everything common — the CDDL preprocessing, the scalar prelude, and the
//! encode-then-validate kernel — lives here.
//!
//! The validating pieces are gated on the `blueprint` feature (they pull in the
//! `cddl` crate and `include_str!` the submodule); the [`conforms!`] macro is
//! always available so call sites compile feature-independently, expanding to a
//! `#[cfg(feature = "blueprint")]` test.

/// Emits one `#[test]` (gated on the `blueprint` feature) that encodes `$msg`
/// with our `Encode` impl and asserts the bytes conform to CDDL rule `$rule`,
/// using the schema produced by the `$schema` builder in scope at the call site.
///
/// ```ignore
/// conforms!(done_conforms, self_contained, "msgClientDone", Message::Done);
/// ```
macro_rules! conforms {
    ($name:ident, $schema:path, $rule:literal, $msg:expr) => {
        #[cfg(feature = "blueprint")]
        #[test]
        fn $name() {
            $crate::protocol::cddl::assert_conforms(
                &$schema(),
                $rule,
                &pallas_codec::minicbor::to_vec(&$msg).unwrap(),
            );
        }
    };
}
pub(crate) use conforms;

/// Minimal prelude for the scalar types the blueprint CDDLs `;# import` from
/// `base`. The opaque/word types collapse to `uint`/`bytes` because cddl-rs
/// can't resolve the imports and the bit-widths aren't what these tests check.
#[cfg(feature = "blueprint")]
pub(crate) const BASE_PRELUDE: &str =
    "slotno = uint\nhash = bytes\nword16 = uint\nword32 = uint\nword64 = uint\n";

/// Rewrites a vendored blueprint CDDL into something cddl-rs can parse: drops the
/// `;# import` pragmas and the `base.` namespace it can't resolve. Protocol-
/// specific relaxations (e.g. opaque payloads to `any`) are layered on by the
/// caller before appending [`BASE_PRELUDE`].
#[cfg(feature = "blueprint")]
pub(crate) fn preprocess(schema: &str) -> String {
    schema
        .lines()
        .filter(|l| !l.trim_start().starts_with(";#"))
        .collect::<Vec<_>>()
        .join("\n")
        .replace("base.", "")
}

/// Validates `cbor` against rule `rule` of `schema`, panicking with the rule name
/// on any mismatch. The single touch point for the `cddl` crate's API.
#[cfg(feature = "blueprint")]
pub(crate) fn assert_conforms(schema: &str, rule: &str, cbor: &[u8]) {
    let doc = format!("start = {rule}\n{schema}");
    ::cddl::validate_cbor_from_slice(&doc, cbor, None)
        .unwrap_or_else(|e| panic!("`{rule}` does not conform to CDDL: {e}"));
}
