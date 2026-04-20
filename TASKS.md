# httparse — ETNA Tasks

Total tasks: 24

ETNA tasks are **mutation/property/witness triplets**. Each row below is one runnable task. The `<PropertyKey>` token in the command column uses the PascalCase key recognised by `src/bin/etna.rs`; passing `All` runs every property for the named framework in a single invocation.

## Property keys

| Property | PropertyKey |
|----------|-------------|
| `property_chunk_size_matches_oracle` | `ChunkSizeMatchesOracle` |
| `property_request_method_is_valid` | `RequestMethodIsValid` |
| `property_request_header_value_preserves` | `RequestHeaderValuePreserves` |
| `property_request_path_preserves` | `RequestPathPreserves` |
| `property_response_header_names_are_tokens` | `ResponseHeaderNamesAreTokens` |

## Task Index

| Task | Variant | Framework | Property | Witness | Command |
|------|---------|-----------|----------|---------|---------|
| 001 | `chunk_size_overflow_34efc1e_1` | proptest | `property_chunk_size_matches_oracle` | `witness_chunk_size_matches_oracle_case_overflow_17` | `cargo run --release --bin etna -- proptest ChunkSizeMatchesOracle` |
| 002 | `chunk_size_overflow_34efc1e_1` | quickcheck | `property_chunk_size_matches_oracle` | `witness_chunk_size_matches_oracle_case_overflow_17` | `cargo run --release --bin etna -- quickcheck ChunkSizeMatchesOracle` |
| 003 | `chunk_size_overflow_34efc1e_1` | crabcheck | `property_chunk_size_matches_oracle` | `witness_chunk_size_matches_oracle_case_overflow_17` | `cargo run --release --bin etna -- crabcheck ChunkSizeMatchesOracle` |
| 004 | `chunk_size_overflow_34efc1e_1` | hegel | `property_chunk_size_matches_oracle` | `witness_chunk_size_matches_oracle_case_overflow_17` | `cargo run --release --bin etna -- hegel ChunkSizeMatchesOracle` |
| 005 | `method_leading_space_9f6702b_1` | proptest | `property_request_method_is_valid` | `witness_request_method_is_valid_case_leading_space` | `cargo run --release --bin etna -- proptest RequestMethodIsValid` |
| 006 | `method_leading_space_9f6702b_1` | quickcheck | `property_request_method_is_valid` | `witness_request_method_is_valid_case_leading_space` | `cargo run --release --bin etna -- quickcheck RequestMethodIsValid` |
| 007 | `method_leading_space_9f6702b_1` | crabcheck | `property_request_method_is_valid` | `witness_request_method_is_valid_case_leading_space` | `cargo run --release --bin etna -- crabcheck RequestMethodIsValid` |
| 008 | `method_leading_space_9f6702b_1` | hegel | `property_request_method_is_valid` | `witness_request_method_is_valid_case_leading_space` | `cargo run --release --bin etna -- hegel RequestMethodIsValid` |
| 009 | `invalid_token_delim_498de3f_1` | proptest | `property_request_method_is_valid` | `witness_request_method_is_valid_case_cr_delim` | `cargo run --release --bin etna -- proptest RequestMethodIsValid` |
| 010 | `invalid_token_delim_498de3f_1` | quickcheck | `property_request_method_is_valid` | `witness_request_method_is_valid_case_cr_delim` | `cargo run --release --bin etna -- quickcheck RequestMethodIsValid` |
| 011 | `invalid_token_delim_498de3f_1` | crabcheck | `property_request_method_is_valid` | `witness_request_method_is_valid_case_cr_delim` | `cargo run --release --bin etna -- crabcheck RequestMethodIsValid` |
| 012 | `invalid_token_delim_498de3f_1` | hegel | `property_request_method_is_valid` | `witness_request_method_is_valid_case_cr_delim` | `cargo run --release --bin etna -- hegel RequestMethodIsValid` |
| 013 | `header_value_htab_59a9fd1_1` | proptest | `property_request_header_value_preserves` | `witness_request_header_value_preserves_case_htab` | `cargo run --release --bin etna -- proptest RequestHeaderValuePreserves` |
| 014 | `header_value_htab_59a9fd1_1` | quickcheck | `property_request_header_value_preserves` | `witness_request_header_value_preserves_case_htab` | `cargo run --release --bin etna -- quickcheck RequestHeaderValuePreserves` |
| 015 | `header_value_htab_59a9fd1_1` | crabcheck | `property_request_header_value_preserves` | `witness_request_header_value_preserves_case_htab` | `cargo run --release --bin etna -- crabcheck RequestHeaderValuePreserves` |
| 016 | `header_value_htab_59a9fd1_1` | hegel | `property_request_header_value_preserves` | `witness_request_header_value_preserves_case_htab` | `cargo run --release --bin etna -- hegel RequestHeaderValuePreserves` |
| 017 | `backslash_in_uri_1a791f4_1` | proptest | `property_request_path_preserves` | `witness_request_path_preserves_case_backslash` | `cargo run --release --bin etna -- proptest RequestPathPreserves` |
| 018 | `backslash_in_uri_1a791f4_1` | quickcheck | `property_request_path_preserves` | `witness_request_path_preserves_case_backslash` | `cargo run --release --bin etna -- quickcheck RequestPathPreserves` |
| 019 | `backslash_in_uri_1a791f4_1` | crabcheck | `property_request_path_preserves` | `witness_request_path_preserves_case_backslash` | `cargo run --release --bin etna -- crabcheck RequestPathPreserves` |
| 020 | `backslash_in_uri_1a791f4_1` | hegel | `property_request_path_preserves` | `witness_request_path_preserves_case_backslash` | `cargo run --release --bin etna -- hegel RequestPathPreserves` |
| 021 | `response_no_reason_c0631f2_1` | proptest | `property_response_header_names_are_tokens` | `witness_response_header_names_are_tokens_case_bare_lf` | `cargo run --release --bin etna -- proptest ResponseHeaderNamesAreTokens` |
| 022 | `response_no_reason_c0631f2_1` | quickcheck | `property_response_header_names_are_tokens` | `witness_response_header_names_are_tokens_case_bare_lf` | `cargo run --release --bin etna -- quickcheck ResponseHeaderNamesAreTokens` |
| 023 | `response_no_reason_c0631f2_1` | crabcheck | `property_response_header_names_are_tokens` | `witness_response_header_names_are_tokens_case_bare_lf` | `cargo run --release --bin etna -- crabcheck ResponseHeaderNamesAreTokens` |
| 024 | `response_no_reason_c0631f2_1` | hegel | `property_response_header_names_are_tokens` | `witness_response_header_names_are_tokens_case_bare_lf` | `cargo run --release --bin etna -- hegel ResponseHeaderNamesAreTokens` |

## Witness catalog

Each witness is a deterministic concrete test. Base build: passes. Variant-active build: fails. Witnesses live in `tests/etna_witnesses.rs`.

| Witness | Property | Detects | Input shape |
|---------|----------|---------|-------------|
| `witness_chunk_size_matches_oracle_case_overflow_17` | `property_chunk_size_matches_oracle` | `chunk_size_overflow_34efc1e_1` | hex digits `b"10000000000000000"` (17 nibbles → 2^64) — oracle says `Err(InvalidChunkSize)`; buggy parser wraps to 0 and returns `Ok(Complete)` |
| `witness_request_method_is_valid_case_leading_space` | `property_request_method_is_valid` | `method_leading_space_9f6702b_1` | method bytes `[b' ', b'G', b'E', b'T']` with SP separator — fixed parser returns Err; buggy `is_method_token` admits SP and returns a method starting with SP |
| `witness_request_method_is_valid_case_cr_delim` | `property_request_method_is_valid` | `invalid_token_delim_498de3f_1` | method `b"GET"` followed by `\r` — fixed `parse_token` only terminates on SP; buggy loop terminates on `\r`/`\n` and accepts the malformed request |
| `witness_request_header_value_preserves_case_htab` | `property_request_header_value_preserves` | `header_value_htab_59a9fd1_1` | header value `b"some\tagent"` containing an embedded HTAB — fixed `HEADER_VALUE_MAP` allows `\t`; buggy map rejects it and errors |
| `witness_request_path_preserves_case_backslash` | `property_request_path_preserves` | `backslash_in_uri_1a791f4_1` | path `b"/foo\\bar"` — fixed `is_uri_token` admits `\`; buggy scalar + SIMD paths reject it, and the whole request errors |
| `witness_response_header_names_are_tokens_case_bare_lf` | `property_response_header_names_are_tokens` | `response_no_reason_c0631f2_1` | status line `b"HTTP/1.0 200\nContent-type: text/html\n\n"` — bare LF with no reason; buggy branch skips `bytes.slice()`, so the next header name absorbs status-line digits and LF |
