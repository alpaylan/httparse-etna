//! Deterministic witness tests for ETNA httparse variants.
//!
//! Each `witness_<property>_case_<tag>` is a fixed concrete input chosen so
//! that:
//!   * on base HEAD the property holds (test passes),
//!   * on the `etna/<variant>` branch (or with the mutation patch applied),
//!     the property fails (test fails).
//!
//! The tests delegate to `property_<name>` — no invariant logic lives here.

use httparse::etna::{
    property_chunk_size_matches_oracle, property_request_header_value_preserves,
    property_request_method_is_valid, property_request_path_preserves,
    property_response_header_names_are_tokens, PropertyResult,
};

fn assert_pass(r: PropertyResult) {
    match r {
        PropertyResult::Pass => {}
        PropertyResult::Discard => {}
        PropertyResult::Fail(m) => panic!("property failed: {m}"),
    }
}

// --- chunk_size_overflow_34efc1e_1 -------------------------------

/// 17 hex digits encode a value strictly larger than u64::MAX, so the
/// spec-correct parser returns InvalidChunkSize. The missing `count > 15`
/// guard in the buggy parser lets the u64 wrap around and returns Ok with a
/// truncated value — catching the bug.
#[test]
fn witness_chunk_size_matches_oracle_case_overflow_17() {
    let digits = b"10000000000000000".to_vec();
    assert_pass(property_chunk_size_matches_oracle(digits));
}

// --- method_leading_space_9f6702b_1 ------------------------------

/// A single leading space before "GET" is rejected by the fix but accepted by
/// the pre-fix `is_method_token` that only checked `b > 0x1F && b < 0x7F`.
/// The buggy parser happily returns a method of `" GET"`.
#[test]
fn witness_request_method_is_valid_case_leading_space() {
    let method = vec![b' ', b'G', b'E', b'T'];
    assert_pass(property_request_method_is_valid(method, b' '));
}

// --- invalid_token_delim_498de3f_1 -------------------------------

/// "GET\r" — the fix only accepts `' '` as a token/method delimiter, but the
/// pre-fix parser terminates on `\r` and `\n` too, so the buggy parser produces
/// a successful parse when the property expects failure.
#[test]
fn witness_request_method_is_valid_case_cr_delim() {
    let method = b"GET".to_vec();
    assert_pass(property_request_method_is_valid(method, b'\r'));
}

// --- header_value_htab_59a9fd1_1 ---------------------------------

/// A header value containing an HTAB byte (`0x09`) between printable bytes.
/// The fix added HTAB to HEADER_VALUE_MAP; the buggy map rejects it, so the
/// parser errors on this otherwise well-formed request.
#[test]
fn witness_request_header_value_preserves_case_htab() {
    let value = b"some\tagent".to_vec();
    assert_pass(property_request_header_value_preserves(value));
}

// --- backslash_in_uri_1a791f4_1 ----------------------------------

/// A path containing a backslash byte. The fix widened `is_uri_token` to
/// accept `\`; the buggy parser still rejects it, failing the parse.
#[test]
fn witness_request_path_preserves_case_backslash() {
    let path = b"/foo\\bar".to_vec();
    assert_pass(property_request_path_preserves(path));
}

// --- response_no_reason_c0631f2_1 --------------------------------

/// A response status line ending in bare LF with no reason phrase. The fix
/// added a `bytes.slice()` call in that branch; the buggy parser leaves the
/// slice-start pointer in the status-line digits, so the next header name
/// ends up including status-line bytes and is no longer a valid HTTP token.
#[test]
fn witness_response_header_names_are_tokens_case_bare_lf() {
    let bytes = b"HTTP/1.0 200\nContent-type: text/html\n\n".to_vec();
    assert_pass(property_response_header_names_are_tokens(bytes));
}
