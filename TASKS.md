# httparse — ETNA Tasks

Total tasks: 24

## Task Index

| Task | Variant | Framework | Property | Witness |
|------|---------|-----------|----------|---------|
| 001 | `backslash_in_uri_1a791f4_1` | proptest | `RequestPathPreserves` | `witness_request_path_preserves_case_backslash` |
| 002 | `backslash_in_uri_1a791f4_1` | quickcheck | `RequestPathPreserves` | `witness_request_path_preserves_case_backslash` |
| 003 | `backslash_in_uri_1a791f4_1` | crabcheck | `RequestPathPreserves` | `witness_request_path_preserves_case_backslash` |
| 004 | `backslash_in_uri_1a791f4_1` | hegel | `RequestPathPreserves` | `witness_request_path_preserves_case_backslash` |
| 005 | `chunk_size_overflow_34efc1e_1` | proptest | `ChunkSizeMatchesOracle` | `witness_chunk_size_matches_oracle_case_overflow_17` |
| 006 | `chunk_size_overflow_34efc1e_1` | quickcheck | `ChunkSizeMatchesOracle` | `witness_chunk_size_matches_oracle_case_overflow_17` |
| 007 | `chunk_size_overflow_34efc1e_1` | crabcheck | `ChunkSizeMatchesOracle` | `witness_chunk_size_matches_oracle_case_overflow_17` |
| 008 | `chunk_size_overflow_34efc1e_1` | hegel | `ChunkSizeMatchesOracle` | `witness_chunk_size_matches_oracle_case_overflow_17` |
| 009 | `header_value_htab_59a9fd1_1` | proptest | `RequestHeaderValuePreserves` | `witness_request_header_value_preserves_case_htab` |
| 010 | `header_value_htab_59a9fd1_1` | quickcheck | `RequestHeaderValuePreserves` | `witness_request_header_value_preserves_case_htab` |
| 011 | `header_value_htab_59a9fd1_1` | crabcheck | `RequestHeaderValuePreserves` | `witness_request_header_value_preserves_case_htab` |
| 012 | `header_value_htab_59a9fd1_1` | hegel | `RequestHeaderValuePreserves` | `witness_request_header_value_preserves_case_htab` |
| 013 | `invalid_token_delim_498de3f_1` | proptest | `RequestMethodIsValid` | `witness_request_method_is_valid_case_cr_delim` |
| 014 | `invalid_token_delim_498de3f_1` | quickcheck | `RequestMethodIsValid` | `witness_request_method_is_valid_case_cr_delim` |
| 015 | `invalid_token_delim_498de3f_1` | crabcheck | `RequestMethodIsValid` | `witness_request_method_is_valid_case_cr_delim` |
| 016 | `invalid_token_delim_498de3f_1` | hegel | `RequestMethodIsValid` | `witness_request_method_is_valid_case_cr_delim` |
| 017 | `method_leading_space_9f6702b_1` | proptest | `RequestMethodIsValid` | `witness_request_method_is_valid_case_leading_space` |
| 018 | `method_leading_space_9f6702b_1` | quickcheck | `RequestMethodIsValid` | `witness_request_method_is_valid_case_leading_space` |
| 019 | `method_leading_space_9f6702b_1` | crabcheck | `RequestMethodIsValid` | `witness_request_method_is_valid_case_leading_space` |
| 020 | `method_leading_space_9f6702b_1` | hegel | `RequestMethodIsValid` | `witness_request_method_is_valid_case_leading_space` |
| 021 | `response_no_reason_c0631f2_1` | proptest | `ResponseHeaderNamesAreTokens` | `witness_response_header_names_are_tokens_case_bare_lf` |
| 022 | `response_no_reason_c0631f2_1` | quickcheck | `ResponseHeaderNamesAreTokens` | `witness_response_header_names_are_tokens_case_bare_lf` |
| 023 | `response_no_reason_c0631f2_1` | crabcheck | `ResponseHeaderNamesAreTokens` | `witness_response_header_names_are_tokens_case_bare_lf` |
| 024 | `response_no_reason_c0631f2_1` | hegel | `ResponseHeaderNamesAreTokens` | `witness_response_header_names_are_tokens_case_bare_lf` |

## Witness Catalog

- `witness_request_path_preserves_case_backslash` — base passes, variant fails
- `witness_chunk_size_matches_oracle_case_overflow_17` — base passes, variant fails
- `witness_request_header_value_preserves_case_htab` — base passes, variant fails
- `witness_request_method_is_valid_case_cr_delim` — base passes, variant fails
- `witness_request_method_is_valid_case_leading_space` — base passes, variant fails
- `witness_response_header_names_are_tokens_case_bare_lf` — base passes, variant fails
