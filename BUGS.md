# httparse — Injected Bugs

Total mutations: 6

## Bug Index

| # | Variant | Name | Location | Injection | Fix Commit |
|---|---------|------|----------|-----------|------------|
| 1 | `backslash_in_uri_1a791f4_1` | `backslash_in_uri` | `src/lib.rs` | `patch` | `1a791f4eee2dbb4e51f5211195a6f14da9aa5c12` |
| 2 | `chunk_size_overflow_34efc1e_1` | `chunk_size_overflow` | `src/lib.rs` | `patch` | `34efc1e39726d7f8d3afe34e2b44d2eebb6ba952` |
| 3 | `header_value_htab_59a9fd1_1` | `header_value_htab` | `src/lib.rs` | `patch` | `59a9fd11b3023581055b4997ff21829e03e909a2` |
| 4 | `invalid_token_delim_498de3f_1` | `invalid_token_delim` | `src/lib.rs` | `patch` | `498de3fa707a4889395850e88e8260261258bbd2` |
| 5 | `method_leading_space_9f6702b_1` | `method_leading_space` | `src/lib.rs` | `patch` | `9f6702be571b19ac84e19678b0c4f7eefd2a11b7` |
| 6 | `response_no_reason_c0631f2_1` | `response_no_reason` | `src/lib.rs` | `patch` | `c0631f26e86157a8110a202c112b0ea7a051025b` |

## Property Mapping

| Variant | Property | Witness(es) |
|---------|----------|-------------|
| `backslash_in_uri_1a791f4_1` | `RequestPathPreserves` | `witness_request_path_preserves_case_backslash` |
| `chunk_size_overflow_34efc1e_1` | `ChunkSizeMatchesOracle` | `witness_chunk_size_matches_oracle_case_overflow_17` |
| `header_value_htab_59a9fd1_1` | `RequestHeaderValuePreserves` | `witness_request_header_value_preserves_case_htab` |
| `invalid_token_delim_498de3f_1` | `RequestMethodIsValid` | `witness_request_method_is_valid_case_cr_delim` |
| `method_leading_space_9f6702b_1` | `RequestMethodIsValid` | `witness_request_method_is_valid_case_leading_space` |
| `response_no_reason_c0631f2_1` | `ResponseHeaderNamesAreTokens` | `witness_response_header_names_are_tokens_case_bare_lf` |

## Framework Coverage

| Property | proptest | quickcheck | crabcheck | hegel |
|----------|---------:|-----------:|----------:|------:|
| `RequestPathPreserves` | ✓ | ✓ | ✓ | ✓ |
| `ChunkSizeMatchesOracle` | ✓ | ✓ | ✓ | ✓ |
| `RequestHeaderValuePreserves` | ✓ | ✓ | ✓ | ✓ |
| `RequestMethodIsValid` | ✓ | ✓ | ✓ | ✓ |
| `ResponseHeaderNamesAreTokens` | ✓ | ✓ | ✓ | ✓ |

## Bug Details

### 1. backslash_in_uri

- **Variant**: `backslash_in_uri_1a791f4_1`
- **Location**: `src/lib.rs`
- **Property**: `RequestPathPreserves`
- **Witness(es)**:
  - `witness_request_path_preserves_case_backslash`
- **Source**: Fix parsing backslashes in request-targets (#57)
  > `is_uri_token` rejected `\` even though RFC 3986 permits it in paths (and web clients routinely emit it). The fix removes the backslash exclusion from both the scalar check and the SIMD fast paths so backslash-bearing request targets round-trip through the parser.
- **Fix commit**: `1a791f4eee2dbb4e51f5211195a6f14da9aa5c12` — Fix parsing backslashes in request-targets (#57)
- **Invariant violated**: A path built from valid URI bytes (`!`..`~` or `0x80..`) must parse, and the parsed path bytes must equal the input bytes.
- **How the mutation triggers**: The buggy `is_uri_token` adds `&& b != b'\\'` to the URI-token check, so any path containing `\` fails the scalar check. The SIMD fast paths are also redirected: SSE4.2/AVX2/NEON `match_uri_vectored` now forward to the SWAR fallback, and SWAR itself skips its block-wise fast path and walks byte-by-byte via `is_uri_token` so the backslash rejection actually fires.

### 2. chunk_size_overflow

- **Variant**: `chunk_size_overflow_34efc1e_1`
- **Location**: `src/lib.rs`
- **Property**: `ChunkSizeMatchesOracle`
- **Witness(es)**:
  - `witness_chunk_size_matches_oracle_case_overflow_17`
- **Source**: fix u64 overflow if chunk size is too big
  > `parse_chunk_size` accumulated digits into a `u64` with plain `*` / `+`, so a 17-hex-digit chunk size (i.e. ≥ 2^64) silently wrapped and the parser returned `Ok(Complete)` on a value it couldn't actually represent. The fix adds a `count > 15` guard that returns `InvalidChunkSize` before the overflow can happen.
- **Fix commit**: `34efc1e39726d7f8d3afe34e2b44d2eebb6ba952` — fix u64 overflow if chunk size is too big
- **Invariant violated**: For hex-digit input, the parser's result must agree with a big-integer (u128) oracle; in particular, any input with 17+ hex digits must return `InvalidChunkSize` because the value cannot fit in u64.
- **How the mutation triggers**: The buggy `parse_chunk_size` drops the `count > 15` guard and the debug-assert, and switches to `wrapping_mul`/`wrapping_add`, so `"10000000000000000"` (17 hex digits → 2^64) wraps to 0 and returns `Ok(Complete)` instead of `Err(InvalidChunkSize)`.

### 3. header_value_htab

- **Variant**: `header_value_htab_59a9fd1_1`
- **Location**: `src/lib.rs`
- **Property**: `RequestHeaderValuePreserves`
- **Witness(es)**:
  - `witness_request_header_value_preserves_case_htab`
- **Source**: allow htabs in header values
  > `HEADER_VALUE_MAP` omitted HTAB (`0x09`), so header values containing tabs — explicitly permitted by RFC 7230 — were rejected as invalid. The fix adds HTAB to the allowed set alongside `0x20..=0x7E` and `0x80..`.
- **Fix commit**: `59a9fd11b3023581055b4997ff21829e03e909a2` — allow htabs in header values
- **Invariant violated**: Header values made of valid value octets — including HTAB (`0x09`) — must parse, and the parsed bytes must equal the input bytes.
- **How the mutation triggers**: The buggy `HEADER_VALUE_MAP` drops HTAB from the allowed set (range shrinks from `b'\t' | 0x20..=0x7E | 0x80..` to `0x20..=0x7E | 0x80..`), so any header value containing `\t` fails to parse.

### 4. invalid_token_delim

- **Variant**: `invalid_token_delim_498de3f_1`
- **Location**: `src/lib.rs`
- **Property**: `RequestMethodIsValid`
- **Witness(es)**:
  - `witness_request_method_is_valid_case_cr_delim`
- **Source**: stop parsing requests with invalid method or path delimiters
  > `parse_token` terminated on SP, CR, or LF, so `"GET\r / HTTP/1.1"` parsed successfully with method `"GET"` even though the method/path boundary must be a single SP. The fix restricts the terminator to SP, causing any other non-token byte to error.
- **Fix commit**: `498de3fa707a4889395850e88e8260261258bbd2` — stop parsing requests with invalid method or path delimiters
- **Invariant violated**: Only `SP` terminates the method token. A bare `\r` (or `\n`) after the method is not a legal delimiter and must error.
- **How the mutation triggers**: The buggy `parse_token` loop ends on SP **or** `\r`/`\n`, so `"GET\r"` parses as method `"GET"` with a successful completion where the fixed parser returns `Err(Token)`.

### 5. method_leading_space

- **Variant**: `method_leading_space_9f6702b_1`
- **Location**: `src/lib.rs`
- **Property**: `RequestMethodIsValid`
- **Witness(es)**:
  - `witness_request_method_is_valid_case_leading_space`
- **Source**: Fix method parsing to reject a leading space (#190)
  > `is_method_token` was implemented as `b > 0x1F && b < 0x7F`, which admits `0x20` (SP). A request line like `" GET / HTTP/1.1"` therefore parsed as method `" GET"` instead of erroring. The fix swaps to the strict HTTP tchar check so SP is rejected.
- **Fix commit**: `9f6702be571b19ac84e19678b0c4f7eefd2a11b7` — Fix method parsing to reject a leading space (#190)
- **Invariant violated**: Parsing `{method} / HTTP/1.1\r\n\r\n` succeeds iff `method` is a non-empty valid HTTP token. A leading SP is not a token byte and must be rejected.
- **How the mutation triggers**: The buggy `is_method_token` is the pre-fix `b > 0x1F && b < 0x7F`, which admits SP. The parser happily accepts a method of `" GET"` whose first byte is SP — the property expects an error and fails.

### 6. response_no_reason

- **Variant**: `response_no_reason_c0631f2_1`
- **Location**: `src/lib.rs`
- **Property**: `ResponseHeaderNamesAreTokens`
- **Witness(es)**:
  - `witness_response_header_names_are_tokens_case_bare_lf`
- **Source**: Fix parsing responses that have no reason phrase followed by no CRs (#96)
  > When the status line ended in bare LF with no reason phrase, `Response::parse` set `reason = Some("")` without advancing the byte-slice cursor, so the next header-name slice started inside the status-line digits (producing names like `"200\nContent-type"`). The fix calls `bytes.slice()` to snap the cursor before parsing headers.
- **Fix commit**: `c0631f26e86157a8110a202c112b0ea7a051025b` — Fix parsing responses that have no reason phrase followed by no CRs (#96)
- **Invariant violated**: When a response parses successfully, every header name must be a non-empty valid HTTP token (no digits at start, no SP/CR/LF, no colons).
- **How the mutation triggers**: When the status line ends in bare `\n` (no reason phrase), the buggy branch sets `self.reason = Some("")` without calling `bytes.slice()`. The slice-start pointer stays inside the status-line digits, so the next header-name slice begins there — producing a header name like `"200\nContent-type"`, which is not a valid token.
