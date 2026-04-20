//! ETNA framework-neutral property functions for httparse.
//!
//! Each `property_<name>` is a pure function taking concrete, owned inputs and
//! returning `PropertyResult`. Framework adapters in `src/bin/etna.rs` and
//! witness tests in `tests/etna_witnesses.rs` all call these functions
//! directly — invariants are never re-implemented inside an adapter.

#![allow(missing_docs)]

use crate::{parse_chunk_size, InvalidChunkSize, Request, Response, Status, EMPTY_HEADER};

#[derive(Debug)]
pub enum PropertyResult {
    Pass,
    Fail(String),
    Discard,
}

// Helpers that mirror the private byte-class predicates in lib.rs. We re-derive
// them here so the etna surface does not depend on `pub(crate)` internals.

fn is_token_byte(b: u8) -> bool {
    matches!(
        b,
        b'A'..=b'Z'
            | b'a'..=b'z'
            | b'0'..=b'9'
            | b'!'
            | b'#'
            | b'$'
            | b'%'
            | b'&'
            | b'\''
            | b'*'
            | b'+'
            | b'-'
            | b'.'
            | b'^'
            | b'_'
            | b'`'
            | b'|'
            | b'~'
    )
}

fn is_header_value_byte(b: u8) -> bool {
    b == b'\t' || (0x20..=0x7E).contains(&b) || b >= 0x80
}

fn is_uri_byte(b: u8) -> bool {
    (b'!'..=0x7e).contains(&b) || b >= 0x80
}

fn hex_digit_value(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

// ============================================================================
// Properties
// ============================================================================

/// Invariant: parsing `{method}{sep}/ HTTP/1.1\r\n\r\n` succeeds iff `sep ==
/// b' '` and `method` is a non-empty valid HTTP token. When it succeeds, the
/// parsed method bytes equal the input method bytes.
///
/// Bugs this catches:
/// - `method_leading_space_9f6702b_1`: a buggy `is_method_token` that accepts
///   ASCII space makes inputs beginning with space parse successfully with a
///   method that starts with space.
/// - `invalid_token_delim_498de3f_1`: a buggy `parse_token` that terminates on
///   `\r` or `\n` makes inputs with those delimiters parse successfully.
pub fn property_request_method_is_valid(method: Vec<u8>, sep: u8) -> PropertyResult {
    if method.is_empty() || method.len() > 64 {
        return PropertyResult::Discard;
    }
    // Only test separators that carry meaning for these bugs.
    if !matches!(sep, b' ' | b'\r' | b'\n') {
        return PropertyResult::Discard;
    }
    // method[0] can be a single leading space (triggers method_leading_space)
    // OR a valid token byte. Nothing else — leading CR/LF would be skipped by
    // the parser's skip_empty_lines, a non-token byte would trivially error,
    // and either perturbs the equality check below.
    if method[0] != b' ' && !is_token_byte(method[0]) {
        return PropertyResult::Discard;
    }
    // method[1..] must be all-token: any embedded whitespace would cause
    // parse_token to truncate early, and any other non-token byte would make
    // the parse trivially error, which isn't what these bugs are about.
    if !method[1..].iter().all(|&b| is_token_byte(b)) {
        return PropertyResult::Discard;
    }

    let mut buf = Vec::new();
    buf.extend_from_slice(&method);
    buf.push(sep);
    buf.extend_from_slice(b"/ HTTP/1.1\r\n\r\n");

    let mut headers = [EMPTY_HEADER; 4];
    let mut req = Request::new(&mut headers[..]);
    let r = req.parse(&buf);

    // Fixed parser: method[0] must be a token byte (not space) AND sep must
    // be exactly space. Anything else is Err.
    let expected_ok = sep == b' ' && is_token_byte(method[0]);

    match r {
        Ok(Status::Complete(_)) => {
            if !expected_ok {
                return PropertyResult::Fail(format!(
                    "expected parse failure but got Complete: method={:?} sep=0x{:02x}",
                    method, sep
                ));
            }
            let parsed = req.method.map(|s| s.as_bytes());
            if parsed != Some(&method[..]) {
                return PropertyResult::Fail(format!(
                    "parsed method {:?} != input {:?}",
                    parsed, method
                ));
            }
            PropertyResult::Pass
        }
        Ok(Status::Partial) => PropertyResult::Discard,
        Err(_e) => {
            if expected_ok {
                PropertyResult::Fail(format!(
                    "expected parse success but got Err: method={:?} sep=0x{:02x}",
                    method, sep
                ))
            } else {
                PropertyResult::Pass
            }
        }
    }
}

/// Invariant: a request with a header value consisting of valid header-value
/// octets (HTAB, SP, printable ASCII, or obs-text) must parse, and the parsed
/// header value must equal the input bytes. Leading and trailing whitespace
/// is excluded from the domain so that value trimming does not perturb the
/// equality check.
///
/// Bugs this catches:
/// - `header_value_htab_59a9fd1_1`: a buggy `HEADER_VALUE_MAP` that rejects
///   `\t` causes any header value containing HTAB to fail to parse.
pub fn property_request_header_value_preserves(value: Vec<u8>) -> PropertyResult {
    if value.is_empty() || value.len() > 128 {
        return PropertyResult::Discard;
    }
    if !value.iter().all(|&b| is_header_value_byte(b)) {
        return PropertyResult::Discard;
    }
    // Disallow leading/trailing whitespace so equality holds without trimming.
    let first = value[0];
    let last = value[value.len() - 1];
    if matches!(first, b' ' | b'\t') || matches!(last, b' ' | b'\t') {
        return PropertyResult::Discard;
    }

    let mut buf = Vec::new();
    buf.extend_from_slice(b"GET / HTTP/1.1\r\nX: ");
    buf.extend_from_slice(&value);
    buf.extend_from_slice(b"\r\n\r\n");

    let mut headers = [EMPTY_HEADER; 4];
    let mut req = Request::new(&mut headers[..]);
    match req.parse(&buf) {
        Ok(Status::Complete(_)) => {
            if req.headers.len() == 1 && req.headers[0].value == &value[..] {
                PropertyResult::Pass
            } else {
                PropertyResult::Fail(format!(
                    "parsed headers={:?} vs input value={:?}",
                    req.headers
                        .iter()
                        .map(|h| (h.name, h.value))
                        .collect::<Vec<_>>(),
                    value
                ))
            }
        }
        Ok(Status::Partial) => PropertyResult::Discard,
        Err(e) => PropertyResult::Fail(format!(
            "unexpected Err({:?}) for value={:?}",
            e, value
        )),
    }
}

/// Invariant: a request whose path consists of valid URI octets must parse,
/// and the parsed path bytes must equal the input path bytes.
///
/// Bugs this catches:
/// - `backslash_in_uri_1a791f4_1`: a buggy `is_uri_token` that rejects `\`
///   makes any path containing backslash fail to parse.
pub fn property_request_path_preserves(path: Vec<u8>) -> PropertyResult {
    if path.is_empty() || path.len() > 128 {
        return PropertyResult::Discard;
    }
    if !path.iter().all(|&b| is_uri_byte(b)) {
        return PropertyResult::Discard;
    }
    let mut buf = Vec::new();
    buf.extend_from_slice(b"GET ");
    buf.extend_from_slice(&path);
    buf.extend_from_slice(b" HTTP/1.1\r\n\r\n");

    let mut headers = [EMPTY_HEADER; 4];
    let mut req = Request::new(&mut headers[..]);
    match req.parse(&buf) {
        Ok(Status::Complete(_)) => match req.path.map(|s| s.as_bytes()) {
            Some(p) if p == &path[..] => PropertyResult::Pass,
            other => PropertyResult::Fail(format!(
                "parsed path {:?} != input {:?}",
                other, path
            )),
        },
        Ok(Status::Partial) => PropertyResult::Discard,
        Err(e) => PropertyResult::Fail(format!(
            "unexpected Err({:?}) for path={:?}",
            e, path
        )),
    }
}

/// Invariant: for `{digits}\r\n` with hex digits only, `parse_chunk_size` must
/// agree with a big-integer oracle. Specifically:
///   * if digits has 17 or more hex digits, result must be
///     `Err(InvalidChunkSize)` (value overflows u64).
///   * if digits has 1..=16 hex digits, result must be
///     `Ok(Complete((digits.len()+2, oracle_value)))`.
///
/// Bugs this catches:
/// - `chunk_size_overflow_34efc1e_1`: missing digit-count guard lets 17+ hex
///   digit inputs silently wrap modulo `u64::MAX + 1`, disagreeing with the
///   oracle.
pub fn property_chunk_size_matches_oracle(digits: Vec<u8>) -> PropertyResult {
    if digits.is_empty() || digits.len() > 32 {
        return PropertyResult::Discard;
    }
    // Compute oracle with u128 arithmetic — safe for up to 32 hex digits.
    let mut oracle_value: u128 = 0;
    for &b in &digits {
        match hex_digit_value(b) {
            Some(d) => {
                oracle_value = oracle_value.saturating_mul(16).saturating_add(d as u128);
            }
            None => return PropertyResult::Discard,
        }
    }
    let oracle_overflows = digits.len() > 16 || oracle_value > u64::MAX as u128;

    let mut buf = Vec::new();
    buf.extend_from_slice(&digits);
    buf.extend_from_slice(b"\r\n");

    let got = parse_chunk_size(&buf);
    match got {
        Err(InvalidChunkSize) => {
            if oracle_overflows {
                PropertyResult::Pass
            } else {
                PropertyResult::Fail(format!(
                    "parse returned Err for in-range digits: {:?}",
                    String::from_utf8_lossy(&digits)
                ))
            }
        }
        Ok(Status::Complete((pos, value))) => {
            if oracle_overflows {
                PropertyResult::Fail(format!(
                    "parse returned Complete for overflowing digits: {:?} -> (pos={}, value={}), oracle={}",
                    String::from_utf8_lossy(&digits),
                    pos,
                    value,
                    oracle_value
                ))
            } else if pos == digits.len() + 2 && value as u128 == oracle_value {
                PropertyResult::Pass
            } else {
                PropertyResult::Fail(format!(
                    "parse mismatch: digits={:?}, got (pos={}, value={}), oracle_value={}",
                    String::from_utf8_lossy(&digits),
                    pos,
                    value,
                    oracle_value
                ))
            }
        }
        Ok(Status::Partial) => PropertyResult::Discard,
    }
}

/// Invariant: when an HTTP response parses successfully, every header name
/// must be a non-empty valid HTTP token — in particular, it must not contain
/// digits at the start, spaces, CR, LF, or colons.
///
/// Bugs this catches:
/// - `response_no_reason_c0631f2_1`: when a status line ends in bare LF with
///   no reason phrase (`HTTP/1.0 200\n...`), the buggy branch forgets to
///   `bytes.slice()`, leaving the slice-start pointer in the status line. The
///   next header name slice then absorbs status-line bytes, producing a name
///   like `200\nContent-type` — full of non-token bytes.
pub fn property_response_header_names_are_tokens(bytes: Vec<u8>) -> PropertyResult {
    if bytes.is_empty() || bytes.len() > 256 {
        return PropertyResult::Discard;
    }
    let mut headers = [EMPTY_HEADER; 4];
    let mut resp = Response::new(&mut headers[..]);
    match resp.parse(&bytes) {
        Ok(Status::Complete(_)) => {
            for h in resp.headers.iter() {
                let name_bytes = h.name.as_bytes();
                if name_bytes.is_empty() {
                    return PropertyResult::Fail(format!(
                        "empty header name in input {:?}",
                        String::from_utf8_lossy(&bytes)
                    ));
                }
                for &b in name_bytes {
                    if !is_token_byte(b) {
                        return PropertyResult::Fail(format!(
                            "header name {:?} contains non-token byte 0x{:02x} in input {:?}",
                            h.name,
                            b,
                            String::from_utf8_lossy(&bytes)
                        ));
                    }
                }
            }
            PropertyResult::Pass
        }
        _ => PropertyResult::Discard,
    }
}
