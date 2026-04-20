# httparse — Injected Bugs

Total mutations: 6 (all expressible against the current tree)

## Bug Index

| # | Name | Variant | File | Injection | Fix Commit |
|---|------|---------|------|-----------|------------|
| 1 | `chunk_size_overflow` | `chunk_size_overflow_34efc1e_1` | `patches/chunk_size_overflow_34efc1e_1.patch` | `patch` | `34efc1e39726d7f8d3afe34e2b44d2eebb6ba952` |
| 2 | `method_leading_space` | `method_leading_space_9f6702b_1` | `patches/method_leading_space_9f6702b_1.patch` | `patch` | `9f6702be571b19ac84e19678b0c4f7eefd2a11b7` |
| 3 | `invalid_token_delim` | `invalid_token_delim_498de3f_1` | `patches/invalid_token_delim_498de3f_1.patch` | `patch` | `498de3fa707a4889395850e88e8260261258bbd2` |
| 4 | `header_value_htab` | `header_value_htab_59a9fd1_1` | `patches/header_value_htab_59a9fd1_1.patch` | `patch` | `59a9fd11b3023581055b4997ff21829e03e909a2` |
| 5 | `backslash_in_uri` | `backslash_in_uri_1a791f4_1` | `patches/backslash_in_uri_1a791f4_1.patch` | `patch` | `1a791f4eee2dbb4e51f5211195a6f14da9aa5c12` |
| 6 | `response_no_reason` | `response_no_reason_c0631f2_1` | `patches/response_no_reason_c0631f2_1.patch` | `patch` | `c0631f26e86157a8110a202c112b0ea7a051025b` |

## Property Mapping

| Variant | Property | Witness(es) |
|---------|----------|-------------|
| `chunk_size_overflow_34efc1e_1` | `property_chunk_size_matches_oracle` | `witness_chunk_size_matches_oracle_case_overflow_17` |
| `method_leading_space_9f6702b_1` | `property_request_method_is_valid` | `witness_request_method_is_valid_case_leading_space` |
| `invalid_token_delim_498de3f_1` | `property_request_method_is_valid` | `witness_request_method_is_valid_case_cr_delim` |
| `header_value_htab_59a9fd1_1` | `property_request_header_value_preserves` | `witness_request_header_value_preserves_case_htab` |
| `backslash_in_uri_1a791f4_1` | `property_request_path_preserves` | `witness_request_path_preserves_case_backslash` |
| `response_no_reason_c0631f2_1` | `property_response_header_names_are_tokens` | `witness_response_header_names_are_tokens_case_bare_lf` |

## Framework Coverage

Every property is detected on every injected variant by all four frameworks and by the deterministic etna witness runner.

| Property | proptest | quickcheck | crabcheck | hegel |
|----------|---------:|-----------:|----------:|------:|
| `property_chunk_size_matches_oracle` | OK | OK | OK | OK |
| `property_request_method_is_valid` | OK | OK | OK | OK |
| `property_request_header_value_preserves` | OK | OK | OK | OK |
| `property_request_path_preserves` | OK | OK | OK | OK |
| `property_response_header_names_are_tokens` | OK | OK | OK | OK |

## Bug Details

### 1. chunk_size_overflow (34efc1e_1)
- **Variant**: `chunk_size_overflow_34efc1e_1`
- **Location**: `src/lib.rs`, `parse_chunk_size`
- **Property**: `property_chunk_size_matches_oracle`
- **Witness**: `witness_chunk_size_matches_oracle_case_overflow_17`
- **Fix commit**: `34efc1e` — "fix u64 overflow if chunk size is too big"
- **Invariant violated**: For hex-digit input, the parser's result must agree with a big-integer (u128) oracle; in particular, any input with 17+ hex digits must return `InvalidChunkSize` because the value cannot fit in u64.
- **How the mutation triggers**: The buggy `parse_chunk_size` drops the `count > 15` guard and the debug-assert, and switches to `wrapping_mul`/`wrapping_add`, so `"10000000000000000"` (17 hex digits → 2^64) wraps to 0 and returns `Ok(Complete)` instead of `Err(InvalidChunkSize)`.

### 2. method_leading_space (9f6702b_1)
- **Variant**: `method_leading_space_9f6702b_1`
- **Location**: `src/lib.rs`, `is_method_token` (which gates `parse_method`)
- **Property**: `property_request_method_is_valid`
- **Witness**: `witness_request_method_is_valid_case_leading_space`
- **Fix commit**: `9f6702b` — "Fix method parsing to reject a leading space (#190)"
- **Invariant violated**: Parsing `{method} / HTTP/1.1\r\n\r\n` succeeds iff `method` is a non-empty valid HTTP token. A leading SP is not a token byte and must be rejected.
- **How the mutation triggers**: The buggy `is_method_token` is the pre-fix `b > 0x1F && b < 0x7F`, which admits SP. The parser happily accepts a method of `" GET"` whose first byte is SP — the property expects an error and fails.

### 3. invalid_token_delim (498de3f_1)
- **Variant**: `invalid_token_delim_498de3f_1`
- **Location**: `src/lib.rs`, `parse_token`
- **Property**: `property_request_method_is_valid`
- **Witness**: `witness_request_method_is_valid_case_cr_delim`
- **Fix commit**: `498de3f` — "stop parsing requests with invalid method or path delimiters"
- **Invariant violated**: Only `SP` terminates the method token. A bare `\r` (or `\n`) after the method is not a legal delimiter and must error.
- **How the mutation triggers**: The buggy `parse_token` loop ends on SP **or** `\r`/`\n`, so `"GET\r"` parses as method `"GET"` with a successful completion where the fixed parser returns `Err(Token)`.

### 4. header_value_htab (59a9fd1_1)
- **Variant**: `header_value_htab_59a9fd1_1`
- **Location**: `src/lib.rs`, `HEADER_VALUE_MAP` (lookup used by `parse_header_value`)
- **Property**: `property_request_header_value_preserves`
- **Witness**: `witness_request_header_value_preserves_case_htab`
- **Fix commit**: `59a9fd1` — "allow htabs in header values"
- **Invariant violated**: Header values made of valid value octets — including HTAB (`0x09`) — must parse, and the parsed bytes must equal the input bytes.
- **How the mutation triggers**: The buggy `HEADER_VALUE_MAP` drops HTAB from the allowed set (range shrinks from `b'\t' | 0x20..=0x7E | 0x80..` to `0x20..=0x7E | 0x80..`), so any header value containing `\t` fails to parse.

### 5. backslash_in_uri (1a791f4_1)
- **Variant**: `backslash_in_uri_1a791f4_1`
- **Location**: `src/lib.rs` `is_uri_token`; `src/simd/{swar,neon,sse42,avx2}.rs` `match_uri_vectored` fast paths
- **Property**: `property_request_path_preserves`
- **Witness**: `witness_request_path_preserves_case_backslash`
- **Fix commit**: `1a791f4` — "Fix parsing backslashes in request-targets (#57)"
- **Invariant violated**: A path built from valid URI bytes (`!`..`~` or `0x80..`) must parse, and the parsed path bytes must equal the input bytes.
- **How the mutation triggers**: The buggy `is_uri_token` adds `&& b != b'\\'` to the URI-token check, so any path containing `\` fails the scalar check. The SIMD fast paths are also redirected: SSE4.2/AVX2/NEON `match_uri_vectored` now forward to the SWAR fallback, and SWAR itself skips its block-wise fast path and walks byte-by-byte via `is_uri_token` so the backslash rejection actually fires.

### 6. response_no_reason (c0631f2_1)
- **Variant**: `response_no_reason_c0631f2_1`
- **Location**: `src/lib.rs`, `Response::parse` — the `b'\n'` branch of the status-line terminator match
- **Property**: `property_response_header_names_are_tokens`
- **Witness**: `witness_response_header_names_are_tokens_case_bare_lf`
- **Fix commit**: `c0631f2` — "Fix parsing responses that have no reason phrase followed by no CRs (#96)"
- **Invariant violated**: When a response parses successfully, every header name must be a non-empty valid HTTP token (no digits at start, no SP/CR/LF, no colons).
- **How the mutation triggers**: When the status line ends in bare `\n` (no reason phrase), the buggy branch sets `self.reason = Some("")` without calling `bytes.slice()`. The slice-start pointer stays inside the status-line digits, so the next header-name slice begins there — producing a header name like `"200\nContent-type"`, which is not a valid token.
