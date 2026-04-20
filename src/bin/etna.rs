// ETNA workload runner for httparse.
//
// Usage: cargo run --release --bin etna -- <tool> <property>
//   tool:     etna | proptest | quickcheck | crabcheck | hegel
//   property: RequestMethodIsValid | RequestHeaderValuePreserves |
//             RequestPathPreserves | ChunkSizeMatchesOracle |
//             ResponseHeaderNamesAreTokens | All
//
// Every invocation prints exactly one JSON line to stdout and exits 0
// (except argv parsing, which exits 2).

use crabcheck::quickcheck as crabcheck_qc;
use crabcheck::quickcheck::Arbitrary as CcArbitrary;
use hegel::{generators as hgen, HealthCheck, Hegel, Settings as HegelSettings, TestCase};
use httparse::etna::{
    property_chunk_size_matches_oracle, property_request_header_value_preserves,
    property_request_method_is_valid, property_request_path_preserves,
    property_response_header_names_are_tokens, PropertyResult,
};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestCaseError, TestError, TestRunner};
use quickcheck_etna::{Arbitrary as QcArbitrary, Gen, QuickCheck, ResultStatus, TestResult};
use rand::Rng;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Default, Clone, Copy)]
struct Metrics {
    inputs: u64,
    elapsed_us: u128,
}

impl Metrics {
    fn combine(self, other: Metrics) -> Metrics {
        Metrics {
            inputs: self.inputs + other.inputs,
            elapsed_us: self.elapsed_us + other.elapsed_us,
        }
    }
}

type Outcome = (Result<(), String>, Metrics);

fn to_err(r: PropertyResult) -> Result<(), String> {
    match r {
        PropertyResult::Pass | PropertyResult::Discard => Ok(()),
        PropertyResult::Fail(m) => Err(m),
    }
}

const ALL_PROPERTIES: &[&str] = &[
    "RequestMethodIsValid",
    "RequestHeaderValuePreserves",
    "RequestPathPreserves",
    "ChunkSizeMatchesOracle",
    "ResponseHeaderNamesAreTokens",
];

fn cases_budget() -> u64 {
    std::env::var("ETNA_CASES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(u64::MAX)
}

fn run_all<F: FnMut(&str) -> Outcome>(mut f: F) -> Outcome {
    let mut total = Metrics::default();
    for p in ALL_PROPERTIES {
        let (r, m) = f(p);
        total = total.combine(m);
        if let Err(e) = r {
            return (Err(e), total);
        }
    }
    (Ok(()), total)
}

// ============================================================================
// Canonical witness inputs — used by `tool=etna` to deterministically replay
// the single most-reliable counterexample per property.
// ============================================================================

// method_leading_space_9f6702b_1: a leading space is accepted by the buggy
// is_method_token but rejected by the fix.
fn canonical_method_leading_space() -> (Vec<u8>, u8) {
    (vec![b' ', b'G', b'E', b'T'], b' ')
}

// header_value_htab_59a9fd1_1: a tab in the middle of the header value is
// rejected by the buggy HEADER_VALUE_MAP.
fn canonical_header_value_with_tab() -> Vec<u8> {
    b"some\tagent".to_vec()
}

// backslash_in_uri_1a791f4_1: a path with backslash bytes is rejected by the
// buggy is_uri_token.
fn canonical_path_with_backslash() -> Vec<u8> {
    b"/foo\\bar".to_vec()
}

// chunk_size_overflow_34efc1e_1: 17 hex digits overflow u64 and should be
// InvalidChunkSize.
fn canonical_overflow_digits() -> Vec<u8> {
    b"10000000000000000".to_vec()
}

// response_no_reason_c0631f2_1: a status line ending in bare LF with no
// reason phrase, followed by a header.
fn canonical_response_no_reason() -> Vec<u8> {
    b"HTTP/1.0 200\nContent-type: text/html\n\n".to_vec()
}

fn check_request_method_is_valid() -> Result<(), String> {
    let (method, sep) = canonical_method_leading_space();
    to_err(property_request_method_is_valid(method, sep))
}

fn check_request_header_value_preserves() -> Result<(), String> {
    to_err(property_request_header_value_preserves(
        canonical_header_value_with_tab(),
    ))
}

fn check_request_path_preserves() -> Result<(), String> {
    to_err(property_request_path_preserves(canonical_path_with_backslash()))
}

fn check_chunk_size_matches_oracle() -> Result<(), String> {
    to_err(property_chunk_size_matches_oracle(canonical_overflow_digits()))
}

fn check_response_header_names_are_tokens() -> Result<(), String> {
    to_err(property_response_header_names_are_tokens(
        canonical_response_no_reason(),
    ))
}

fn run_etna_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_etna_property);
    }
    let t0 = Instant::now();
    let result = match property {
        "RequestMethodIsValid" => check_request_method_is_valid(),
        "RequestHeaderValuePreserves" => check_request_header_value_preserves(),
        "RequestPathPreserves" => check_request_path_preserves(),
        "ChunkSizeMatchesOracle" => check_chunk_size_matches_oracle(),
        "ResponseHeaderNamesAreTokens" => check_response_header_names_are_tokens(),
        _ => {
            return (
                Err(format!("Unknown property for etna: {property}")),
                Metrics::default(),
            );
        }
    };
    (
        result,
        Metrics {
            inputs: 1,
            elapsed_us: t0.elapsed().as_micros(),
        },
    )
}

// ============================================================================
// Shared biased generators (quickcheck + crabcheck + proptest + hegel).
// Each newtype wraps the shape of a single property input.
// ============================================================================

// First byte of the method — biased toward leading space (to exercise the
// method_leading_space bug) while still producing plenty of "normal" starts.
const METHOD_FIRST_POOL: &[u8] = &[
    b' ', b' ', b' ', // leading-space bias (triggers method_leading_space)
    b'G', b'P', b'O', b'H', b'A', b'D', b'U', b'C', b'N', b'E', b'T', b'S',
];

// Bytes for method[1..] — all valid HTTP token bytes, so parse_token won't
// truncate early. Any byte here must pass is_token_byte in the property.
const METHOD_REST_POOL: &[u8] = &[
    b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O', b'P',
    b'Q', b'R', b'S', b'T', b'U', b'V', b'W', b'X', b'Y', b'Z', b'a', b'z', b'0', b'9', b'!', b'-',
    b'.', b'_',
];

// Separators that appear after the method. `b' '` is the only legal one; the
// other two exercise the invalid_token_delim bug.
const METHOD_SEP_POOL: &[u8] = &[b' ', b' ', b' ', b'\r', b'\n'];

// Header-value byte pool — includes HTAB deliberately so the header_value_htab
// bug fires on most trials.
const HEADER_VALUE_POOL: &[u8] = &[
    b'a', b'b', b'c', b'1', b'2', b'-', b' ', b'\t', 0xC3, 0xA9, // UTF-8 é
    b'A', b'Z', b',', b'.',
];

// Path byte pool — biased toward backslash.
const PATH_BYTE_POOL: &[u8] = &[
    b'/', b'a', b'b', b'0', b'1', b'\\', b'?', b'=', b'-', b'.', b'_',
];

// Hex digit pool with a bias toward producing overflow-length strings.
const HEX_POOL: &[u8] = &[
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f',
    b'A', b'B', b'C', b'D', b'E', b'F',
];

// Response templates — each one is a valid or slightly-malformed HTTP/1.x
// status line plus a header block. `{LF}` placeholder is filled with either
// `\r\n` or bare `\n` to produce buggy-trigger patterns.
const RESPONSE_TEMPLATES: &[&[u8]] = &[
    b"HTTP/1.0 200\nContent-type: text/html\n\n",
    b"HTTP/1.1 404 Not Found\r\nServer: test\r\n\r\n",
    b"HTTP/1.1 500\nX-Err: 1\n\n",
    b"HTTP/1.0 204\nConnection: close\n\n",
    b"HTTP/1.1 301 Moved Permanently\r\nLocation: /\r\n\r\n",
];

// ---------- newtype wrappers and Debug / Display impls ----------

#[derive(Clone)]
struct Method(Vec<u8>);
#[derive(Clone)]
struct Sep(u8);
#[derive(Clone)]
struct HeaderValue(Vec<u8>);
#[derive(Clone)]
struct Path(Vec<u8>);
#[derive(Clone)]
struct HexDigits(Vec<u8>);
#[derive(Clone)]
struct ResponseBytes(Vec<u8>);

fn fmt_bytes(b: &[u8], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{:?}", String::from_utf8_lossy(b))
}

macro_rules! impl_fmt_for_bytes {
    ($t:ty) => {
        impl fmt::Debug for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt_bytes(&self.0, f)
            }
        }
        impl fmt::Display for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt_bytes(&self.0, f)
            }
        }
    };
}
impl_fmt_for_bytes!(Method);
impl_fmt_for_bytes!(HeaderValue);
impl_fmt_for_bytes!(Path);
impl_fmt_for_bytes!(HexDigits);
impl_fmt_for_bytes!(ResponseBytes);

impl fmt::Debug for Sep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:02x}", self.0)
    }
}
impl fmt::Display for Sep {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "0x{:02x}", self.0)
    }
}

// ---------- generators ----------

fn random_bytes_from_pool<R: Rng>(
    rng: &mut R,
    pool: &[u8],
    min_len: usize,
    max_len: usize,
) -> Vec<u8> {
    let len = rng.random_range(min_len..=max_len);
    (0..len)
        .map(|_| pool[rng.random_range(0..pool.len())])
        .collect()
}

fn random_method<R: Rng>(rng: &mut R) -> Vec<u8> {
    let len = rng.random_range(1usize..=8);
    let first = METHOD_FIRST_POOL[rng.random_range(0..METHOD_FIRST_POOL.len())];
    let mut v = Vec::with_capacity(len);
    v.push(first);
    for _ in 1..len {
        v.push(METHOD_REST_POOL[rng.random_range(0..METHOD_REST_POOL.len())]);
    }
    v
}
fn random_sep<R: Rng>(rng: &mut R) -> u8 {
    METHOD_SEP_POOL[rng.random_range(0..METHOD_SEP_POOL.len())]
}
fn random_header_value<R: Rng>(rng: &mut R) -> Vec<u8> {
    // Ensure the value isn't leading/trailing whitespace; the property
    // discards those anyway, but biasing helps fuzz time.
    let mut v = random_bytes_from_pool(rng, HEADER_VALUE_POOL, 2, 16);
    if v[0] == b' ' || v[0] == b'\t' {
        v[0] = b'x';
    }
    let last = v.len() - 1;
    if v[last] == b' ' || v[last] == b'\t' {
        v[last] = b'y';
    }
    v
}
fn random_path<R: Rng>(rng: &mut R) -> Vec<u8> {
    random_bytes_from_pool(rng, PATH_BYTE_POOL, 1, 16)
}
fn random_hex_digits<R: Rng>(rng: &mut R) -> Vec<u8> {
    // Bias toward "around the 16-digit overflow boundary".
    let len = match rng.random_range(0..4) {
        0 => rng.random_range(1..=8),
        1 => rng.random_range(9..=16),
        _ => rng.random_range(17..=24),
    };
    (0..len)
        .map(|_| HEX_POOL[rng.random_range(0..HEX_POOL.len())])
        .collect()
}
fn random_response<R: Rng>(rng: &mut R) -> Vec<u8> {
    let idx = rng.random_range(0..RESPONSE_TEMPLATES.len());
    RESPONSE_TEMPLATES[idx].to_vec()
}

// ---------- Arbitrary impls for quickcheck ----------

impl QcArbitrary for Method {
    fn arbitrary(g: &mut Gen) -> Self {
        let len = g.random_range(1usize..=8);
        let first = METHOD_FIRST_POOL[g.random_range(0..METHOD_FIRST_POOL.len())];
        let mut v = Vec::with_capacity(len);
        v.push(first);
        for _ in 1..len {
            v.push(METHOD_REST_POOL[g.random_range(0..METHOD_REST_POOL.len())]);
        }
        Method(v)
    }
}
impl QcArbitrary for Sep {
    fn arbitrary(g: &mut Gen) -> Self {
        Sep(METHOD_SEP_POOL[g.random_range(0..METHOD_SEP_POOL.len())])
    }
}
impl QcArbitrary for HeaderValue {
    fn arbitrary(g: &mut Gen) -> Self {
        let len = g.random_range(2usize..=16);
        let mut v: Vec<u8> = (0..len)
            .map(|_| HEADER_VALUE_POOL[g.random_range(0..HEADER_VALUE_POOL.len())])
            .collect();
        if v[0] == b' ' || v[0] == b'\t' {
            v[0] = b'x';
        }
        let last = v.len() - 1;
        if v[last] == b' ' || v[last] == b'\t' {
            v[last] = b'y';
        }
        HeaderValue(v)
    }
}
impl QcArbitrary for Path {
    fn arbitrary(g: &mut Gen) -> Self {
        let len = g.random_range(1usize..=16);
        let v = (0..len)
            .map(|_| PATH_BYTE_POOL[g.random_range(0..PATH_BYTE_POOL.len())])
            .collect();
        Path(v)
    }
}
impl QcArbitrary for HexDigits {
    fn arbitrary(g: &mut Gen) -> Self {
        let len = match g.random_range(0..4) {
            0 => g.random_range(1usize..=8),
            1 => g.random_range(9usize..=16),
            _ => g.random_range(17usize..=24),
        };
        let v = (0..len)
            .map(|_| HEX_POOL[g.random_range(0..HEX_POOL.len())])
            .collect();
        HexDigits(v)
    }
}
impl QcArbitrary for ResponseBytes {
    fn arbitrary(g: &mut Gen) -> Self {
        let idx = g.random_range(0..RESPONSE_TEMPLATES.len());
        ResponseBytes(RESPONSE_TEMPLATES[idx].to_vec())
    }
}

// ---------- Arbitrary impls for crabcheck ----------

impl<R: Rng> CcArbitrary<R> for Method {
    fn generate(rng: &mut R, _n: usize) -> Self {
        Method(random_method(rng))
    }
}
impl<R: Rng> CcArbitrary<R> for Sep {
    fn generate(rng: &mut R, _n: usize) -> Self {
        Sep(random_sep(rng))
    }
}
impl<R: Rng> CcArbitrary<R> for HeaderValue {
    fn generate(rng: &mut R, _n: usize) -> Self {
        HeaderValue(random_header_value(rng))
    }
}
impl<R: Rng> CcArbitrary<R> for Path {
    fn generate(rng: &mut R, _n: usize) -> Self {
        Path(random_path(rng))
    }
}
impl<R: Rng> CcArbitrary<R> for HexDigits {
    fn generate(rng: &mut R, _n: usize) -> Self {
        HexDigits(random_hex_digits(rng))
    }
}
impl<R: Rng> CcArbitrary<R> for ResponseBytes {
    fn generate(rng: &mut R, _n: usize) -> Self {
        ResponseBytes(random_response(rng))
    }
}

// ============================================================================
// proptest adapter
// ============================================================================

fn method_strategy() -> BoxedStrategy<Vec<u8>> {
    (
        prop::sample::select(METHOD_FIRST_POOL.to_vec()),
        prop::collection::vec(prop::sample::select(METHOD_REST_POOL.to_vec()), 0..=7),
    )
        .prop_map(|(first, mut rest)| {
            let mut v = Vec::with_capacity(rest.len() + 1);
            v.push(first);
            v.append(&mut rest);
            v
        })
        .boxed()
}
fn sep_strategy() -> BoxedStrategy<u8> {
    prop::sample::select(METHOD_SEP_POOL.to_vec()).boxed()
}
fn header_value_strategy() -> BoxedStrategy<Vec<u8>> {
    prop::collection::vec(prop::sample::select(HEADER_VALUE_POOL.to_vec()), 2..=16).boxed()
}
fn path_strategy() -> BoxedStrategy<Vec<u8>> {
    prop::collection::vec(prop::sample::select(PATH_BYTE_POOL.to_vec()), 1..=16).boxed()
}
fn hex_digits_strategy() -> BoxedStrategy<Vec<u8>> {
    prop::collection::vec(prop::sample::select(HEX_POOL.to_vec()), 1..=24).boxed()
}
fn response_strategy() -> BoxedStrategy<Vec<u8>> {
    prop::sample::select(
        RESPONSE_TEMPLATES
            .iter()
            .map(|t| t.to_vec())
            .collect::<Vec<_>>(),
    )
    .boxed()
}

fn run_proptest_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_proptest_property);
    }
    let counter = Arc::new(AtomicU64::new(0));
    let t0 = Instant::now();
    let cfg = ProptestConfig {
        cases: cases_budget().min(u32::MAX as u64) as u32,
        max_shrink_iters: 32,
        failure_persistence: None,
        ..ProptestConfig::default()
    };
    let mut runner = TestRunner::new(cfg);
    let c = counter.clone();
    let result: Result<(), String> = match property {
        "RequestMethodIsValid" => runner
            .run(&(method_strategy(), sep_strategy()), move |(m, s)| {
                c.fetch_add(1, Ordering::Relaxed);
                let cex = format!("({:?} 0x{:02x})", m, s);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_request_method_is_valid(m, s)
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => Ok(()),
                    Ok(PropertyResult::Fail(_)) | Err(_) => Err(TestCaseError::fail(cex)),
                }
            })
            .map_err(|e| match e {
                TestError::Fail(reason, _) => reason.to_string(),
                other => other.to_string(),
            }),
        "RequestHeaderValuePreserves" => runner
            .run(&header_value_strategy(), move |v| {
                c.fetch_add(1, Ordering::Relaxed);
                let cex = format!("({:?})", v);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_request_header_value_preserves(v)
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => Ok(()),
                    Ok(PropertyResult::Fail(_)) | Err(_) => Err(TestCaseError::fail(cex)),
                }
            })
            .map_err(|e| match e {
                TestError::Fail(reason, _) => reason.to_string(),
                other => other.to_string(),
            }),
        "RequestPathPreserves" => runner
            .run(&path_strategy(), move |p| {
                c.fetch_add(1, Ordering::Relaxed);
                let cex = format!("({:?})", p);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_request_path_preserves(p)
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => Ok(()),
                    Ok(PropertyResult::Fail(_)) | Err(_) => Err(TestCaseError::fail(cex)),
                }
            })
            .map_err(|e| match e {
                TestError::Fail(reason, _) => reason.to_string(),
                other => other.to_string(),
            }),
        "ChunkSizeMatchesOracle" => runner
            .run(&hex_digits_strategy(), move |d| {
                c.fetch_add(1, Ordering::Relaxed);
                let cex = format!("({:?})", d);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_chunk_size_matches_oracle(d)
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => Ok(()),
                    Ok(PropertyResult::Fail(_)) | Err(_) => Err(TestCaseError::fail(cex)),
                }
            })
            .map_err(|e| match e {
                TestError::Fail(reason, _) => reason.to_string(),
                other => other.to_string(),
            }),
        "ResponseHeaderNamesAreTokens" => runner
            .run(&response_strategy(), move |b| {
                c.fetch_add(1, Ordering::Relaxed);
                let cex = format!("({:?})", b);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_response_header_names_are_tokens(b)
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => Ok(()),
                    Ok(PropertyResult::Fail(_)) | Err(_) => Err(TestCaseError::fail(cex)),
                }
            })
            .map_err(|e| match e {
                TestError::Fail(reason, _) => reason.to_string(),
                other => other.to_string(),
            }),
        _ => {
            return (
                Err(format!("Unknown property for proptest: {property}")),
                Metrics::default(),
            );
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = counter.load(Ordering::Relaxed);
    (result, Metrics { inputs, elapsed_us })
}

// ============================================================================
// quickcheck adapter (fork with `etna` feature — fn-pointer API)
// ============================================================================

static QC_COUNTER: AtomicU64 = AtomicU64::new(0);

fn qc_request_method_is_valid(Method(m): Method, Sep(s): Sep) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_request_method_is_valid(m, s) {
        PropertyResult::Pass => TestResult::passed(),
        PropertyResult::Discard => TestResult::discard(),
        PropertyResult::Fail(_) => TestResult::failed(),
    }
}
fn qc_request_header_value_preserves(HeaderValue(v): HeaderValue) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_request_header_value_preserves(v) {
        PropertyResult::Pass => TestResult::passed(),
        PropertyResult::Discard => TestResult::discard(),
        PropertyResult::Fail(_) => TestResult::failed(),
    }
}
fn qc_request_path_preserves(Path(p): Path) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_request_path_preserves(p) {
        PropertyResult::Pass => TestResult::passed(),
        PropertyResult::Discard => TestResult::discard(),
        PropertyResult::Fail(_) => TestResult::failed(),
    }
}
fn qc_chunk_size_matches_oracle(HexDigits(d): HexDigits) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_chunk_size_matches_oracle(d) {
        PropertyResult::Pass => TestResult::passed(),
        PropertyResult::Discard => TestResult::discard(),
        PropertyResult::Fail(_) => TestResult::failed(),
    }
}
fn qc_response_header_names_are_tokens(ResponseBytes(b): ResponseBytes) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_response_header_names_are_tokens(b) {
        PropertyResult::Pass => TestResult::passed(),
        PropertyResult::Discard => TestResult::discard(),
        PropertyResult::Fail(_) => TestResult::failed(),
    }
}

fn run_quickcheck_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_quickcheck_property);
    }
    QC_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let budget = cases_budget();
    let mut qc = QuickCheck::new()
        .tests(budget)
        .max_tests(budget.saturating_mul(4))
        .max_time(Duration::from_secs(86_400));
    let result = match property {
        "RequestMethodIsValid" => {
            qc.quicktest(qc_request_method_is_valid as fn(Method, Sep) -> TestResult)
        }
        "RequestHeaderValuePreserves" => qc.quicktest(
            qc_request_header_value_preserves as fn(HeaderValue) -> TestResult,
        ),
        "RequestPathPreserves" => {
            qc.quicktest(qc_request_path_preserves as fn(Path) -> TestResult)
        }
        "ChunkSizeMatchesOracle" => {
            qc.quicktest(qc_chunk_size_matches_oracle as fn(HexDigits) -> TestResult)
        }
        "ResponseHeaderNamesAreTokens" => qc.quicktest(
            qc_response_header_names_are_tokens as fn(ResponseBytes) -> TestResult,
        ),
        _ => {
            return (
                Err(format!("Unknown property for quickcheck: {property}")),
                Metrics::default(),
            );
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = QC_COUNTER.load(Ordering::Relaxed);
    let status = match result.status {
        ResultStatus::Finished => Ok(()),
        ResultStatus::Failed { arguments } => Err(format!("({})", arguments.join(" "))),
        ResultStatus::Aborted { err } => Err(format!("quickcheck aborted: {err:?}")),
        ResultStatus::TimedOut => Err("quickcheck timed out".to_string()),
        ResultStatus::GaveUp => Err(format!(
            "quickcheck gave up after {} tests",
            result.n_tests_passed
        )),
    };
    (status, Metrics { inputs, elapsed_us })
}

// ============================================================================
// crabcheck adapter (fn-pointer API, tuple args)
// ============================================================================

static CC_COUNTER: AtomicU64 = AtomicU64::new(0);

fn cc_request_method_is_valid((Method(m), Sep(s)): (Method, Sep)) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_request_method_is_valid(m, s) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}
fn cc_request_header_value_preserves(HeaderValue(v): HeaderValue) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_request_header_value_preserves(v) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}
fn cc_request_path_preserves(Path(p): Path) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_request_path_preserves(p) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}
fn cc_chunk_size_matches_oracle(HexDigits(d): HexDigits) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_chunk_size_matches_oracle(d) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}
fn cc_response_header_names_are_tokens(ResponseBytes(b): ResponseBytes) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    match property_response_header_names_are_tokens(b) {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn run_crabcheck_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_crabcheck_property);
    }
    CC_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let cc_config = crabcheck_qc::Config {
        tests: cases_budget(),
    };
    let result = match property {
        "RequestMethodIsValid" => {
            crabcheck_qc::quickcheck_with_config(cc_config, cc_request_method_is_valid)
        }
        "RequestHeaderValuePreserves" => {
            crabcheck_qc::quickcheck_with_config(cc_config, cc_request_header_value_preserves)
        }
        "RequestPathPreserves" => {
            crabcheck_qc::quickcheck_with_config(cc_config, cc_request_path_preserves)
        }
        "ChunkSizeMatchesOracle" => {
            crabcheck_qc::quickcheck_with_config(cc_config, cc_chunk_size_matches_oracle)
        }
        "ResponseHeaderNamesAreTokens" => {
            crabcheck_qc::quickcheck_with_config(cc_config, cc_response_header_names_are_tokens)
        }
        _ => {
            return (
                Err(format!("Unknown property for crabcheck: {property}")),
                Metrics::default(),
            );
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = CC_COUNTER.load(Ordering::Relaxed);
    let status = match result.status {
        crabcheck_qc::ResultStatus::Finished => Ok(()),
        crabcheck_qc::ResultStatus::Failed { arguments } => {
            Err(format!("({})", arguments.join(" ")))
        }
        crabcheck_qc::ResultStatus::TimedOut => Err("crabcheck timed out".to_string()),
        crabcheck_qc::ResultStatus::GaveUp => Err(format!(
            "crabcheck gave up: passed={}, discarded={}",
            result.passed, result.discarded
        )),
        crabcheck_qc::ResultStatus::Aborted { error } => {
            Err(format!("crabcheck aborted: {error}"))
        }
    };
    (status, Metrics { inputs, elapsed_us })
}

// ============================================================================
// hegel adapter (real hegeltest 0.3.7 — panic-on-cex API)
// ============================================================================

static HG_COUNTER: AtomicU64 = AtomicU64::new(0);

fn hegel_settings() -> HegelSettings {
    HegelSettings::new()
        .test_cases(cases_budget())
        .suppress_health_check(HealthCheck::all())
}

fn hg_draw_byte_from(tc: &TestCase, pool: &[u8]) -> u8 {
    let idx = tc.draw(
        hgen::integers::<usize>()
            .min_value(0)
            .max_value(pool.len() - 1),
    );
    pool[idx]
}

fn hg_draw_method(tc: &TestCase) -> Vec<u8> {
    let len = tc.draw(hgen::integers::<usize>().min_value(1).max_value(8));
    let first = hg_draw_byte_from(tc, METHOD_FIRST_POOL);
    let mut v = Vec::with_capacity(len);
    v.push(first);
    for _ in 1..len {
        v.push(hg_draw_byte_from(tc, METHOD_REST_POOL));
    }
    v
}
fn hg_draw_sep(tc: &TestCase) -> u8 {
    hg_draw_byte_from(tc, METHOD_SEP_POOL)
}
fn hg_draw_header_value(tc: &TestCase) -> Vec<u8> {
    let len = tc.draw(hgen::integers::<usize>().min_value(2).max_value(16));
    let mut v: Vec<u8> = (0..len)
        .map(|_| hg_draw_byte_from(tc, HEADER_VALUE_POOL))
        .collect();
    if v[0] == b' ' || v[0] == b'\t' {
        v[0] = b'x';
    }
    let last = v.len() - 1;
    if v[last] == b' ' || v[last] == b'\t' {
        v[last] = b'y';
    }
    v
}
fn hg_draw_path(tc: &TestCase) -> Vec<u8> {
    let len = tc.draw(hgen::integers::<usize>().min_value(1).max_value(16));
    (0..len).map(|_| hg_draw_byte_from(tc, PATH_BYTE_POOL)).collect()
}
fn hg_draw_hex_digits(tc: &TestCase) -> Vec<u8> {
    let len = tc.draw(hgen::integers::<usize>().min_value(1).max_value(24));
    (0..len).map(|_| hg_draw_byte_from(tc, HEX_POOL)).collect()
}
fn hg_draw_response(tc: &TestCase) -> Vec<u8> {
    let idx = tc.draw(
        hgen::integers::<usize>()
            .min_value(0)
            .max_value(RESPONSE_TEMPLATES.len() - 1),
    );
    RESPONSE_TEMPLATES[idx].to_vec()
}

fn run_hegel_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_hegel_property);
    }
    HG_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let settings = hegel_settings();
    let run_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match property {
        "RequestMethodIsValid" => {
            Hegel::new(|tc: TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let m = hg_draw_method(&tc);
                let s = hg_draw_sep(&tc);
                let cex = format!("({:?} 0x{:02x})", m, s);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_request_method_is_valid(m, s)
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => {}
                    Ok(PropertyResult::Fail(_)) | Err(_) => panic!("{cex}"),
                }
            })
            .settings(settings.clone())
            .run();
        }
        "RequestHeaderValuePreserves" => {
            Hegel::new(|tc: TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let v = hg_draw_header_value(&tc);
                let cex = format!("({:?})", v);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_request_header_value_preserves(v)
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => {}
                    Ok(PropertyResult::Fail(_)) | Err(_) => panic!("{cex}"),
                }
            })
            .settings(settings.clone())
            .run();
        }
        "RequestPathPreserves" => {
            Hegel::new(|tc: TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let p = hg_draw_path(&tc);
                let cex = format!("({:?})", p);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_request_path_preserves(p)
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => {}
                    Ok(PropertyResult::Fail(_)) | Err(_) => panic!("{cex}"),
                }
            })
            .settings(settings.clone())
            .run();
        }
        "ChunkSizeMatchesOracle" => {
            Hegel::new(|tc: TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let d = hg_draw_hex_digits(&tc);
                let cex = format!("({:?})", d);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_chunk_size_matches_oracle(d)
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => {}
                    Ok(PropertyResult::Fail(_)) | Err(_) => panic!("{cex}"),
                }
            })
            .settings(settings.clone())
            .run();
        }
        "ResponseHeaderNamesAreTokens" => {
            Hegel::new(|tc: TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                let b = hg_draw_response(&tc);
                let cex = format!("({:?})", b);
                let out = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_response_header_names_are_tokens(b)
                }));
                match out {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => {}
                    Ok(PropertyResult::Fail(_)) | Err(_) => panic!("{cex}"),
                }
            })
            .settings(settings.clone())
            .run();
        }
        _ => panic!("__unknown_property:{}", property),
    }));
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = HG_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match run_result {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "hegel panicked with non-string payload".to_string()
            };
            if let Some(rest) = msg.strip_prefix("__unknown_property:") {
                return (
                    Err(format!("Unknown property for hegel: {rest}")),
                    Metrics::default(),
                );
            }
            Err(msg
                .strip_prefix("Property test failed: ")
                .unwrap_or(&msg)
                .to_string())
        }
    };
    (status, metrics)
}

// ============================================================================
// dispatch + main
// ============================================================================

fn run(tool: &str, property: &str) -> Outcome {
    match tool {
        "etna" => run_etna_property(property),
        "proptest" => run_proptest_property(property),
        "quickcheck" => run_quickcheck_property(property),
        "crabcheck" => run_crabcheck_property(property),
        "hegel" => run_hegel_property(property),
        _ => (
            Err(format!("Unknown tool: {tool}")),
            Metrics::default(),
        ),
    }
}

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn emit_json(
    tool: &str,
    property: &str,
    status: &str,
    metrics: Metrics,
    counterexample: Option<&str>,
    error: Option<&str>,
) {
    let cex = counterexample.map_or("null".to_string(), json_str);
    let err = error.map_or("null".to_string(), json_str);
    println!(
        "{{\"status\":{},\"tests\":{},\"discards\":0,\"time\":{},\"counterexample\":{},\"error\":{},\"tool\":{},\"property\":{}}}",
        json_str(status),
        metrics.inputs,
        json_str(&format!("{}us", metrics.elapsed_us)),
        cex,
        err,
        json_str(tool),
        json_str(property),
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <tool> <property>", args[0]);
        eprintln!("Tools: etna | proptest | quickcheck | crabcheck | hegel");
        eprintln!(
            "Properties: RequestMethodIsValid | RequestHeaderValuePreserves | RequestPathPreserves | ChunkSizeMatchesOracle | ResponseHeaderNamesAreTokens | All"
        );
        std::process::exit(2);
    }
    let (tool, property) = (args[1].as_str(), args[2].as_str());

    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(tool, property)));
    std::panic::set_hook(previous_hook);

    let (result, metrics) = match caught {
        Ok(outcome) => outcome,
        Err(payload) => {
            let msg = if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = payload.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "panic with non-string payload".to_string()
            };
            emit_json(tool, property, "aborted", Metrics::default(), None, Some(&msg));
            return;
        }
    };

    match result {
        Ok(()) => emit_json(tool, property, "passed", metrics, None, None),
        Err(e) => emit_json(tool, property, "failed", metrics, Some(&e), None),
    }
}
