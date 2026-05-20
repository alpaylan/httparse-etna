use std::fmt;

use crabcheck::profiling::quickcheck;
use crabcheck::quickcheck::{Arbitrary, Mutate};
use httparse::etna::{
    property_chunk_size_matches_oracle, property_request_header_value_preserves,
    property_request_method_is_valid, property_request_path_preserves,
    property_response_header_names_are_tokens, PropertyResult,
};
use rand::Rng;

#[derive(Clone)]
struct Bytes(Vec<u8>);
impl fmt::Debug for Bytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(f) }
}

#[derive(Clone)]
struct MethodInput { method: Vec<u8>, sep: u8 }
impl fmt::Debug for MethodInput {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "method={:?} sep=0x{:02x}", self.method, self.sep)
    }
}

// Pools mirror the existing crabcheck adapter in src/bin/etna.rs so the
// generators bias toward bug-triggering inputs (leading-space methods, hex
// digits, backslash paths, HTAB header values, HTTP/1.1-shaped responses).
const METHOD_FIRST_POOL: &[u8] = &[
    b' ', b' ', b' ',
    b'G', b'P', b'O', b'H', b'A', b'D', b'U', b'C', b'N', b'E', b'T', b'S',
];
const METHOD_REST_POOL: &[u8] = &[
    b'A', b'B', b'C', b'D', b'E', b'F', b'G', b'H', b'I', b'J', b'K', b'L', b'M', b'N', b'O', b'P',
    b'Q', b'R', b'S', b'T', b'U', b'V', b'W', b'X', b'Y', b'Z', b'a', b'z', b'0', b'9', b'!', b'-',
    b'.', b'_',
];
const METHOD_SEP_POOL: &[u8] = &[b' ', b' ', b' ', b'\r', b'\n'];
const HEADER_VALUE_POOL: &[u8] = &[
    b'a', b'b', b'c', b'1', b'2', b'-', b' ', b'\t', 0xC3, 0xA9,
    b'A', b'Z', b',', b'.',
];
const PATH_BYTE_POOL: &[u8] = &[
    b'/', b'a', b'b', b'0', b'1', b'\\', b'?', b'=', b'-', b'.', b'_',
];
const HEX_POOL: &[u8] = &[
    b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7', b'8', b'9',
    b'a', b'b', b'c', b'd', b'e', b'f',
    b'A', b'B', b'C', b'D', b'E', b'F',
];
const RESPONSE_TEMPLATES: &[&[u8]] = &[
    b"HTTP/1.0 200\nContent-type: text/html\n\n",
    b"HTTP/1.1 404 Not Found\r\nServer: test\r\n\r\n",
    b"HTTP/1.1 500\nX-Err: 1\n\n",
    b"HTTP/1.0 204\nConnection: close\n\n",
];

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

fn random_bytes_from_pool<R: Rng>(rng: &mut R, pool: &[u8], min_len: usize, max_len: usize) -> Vec<u8> {
    let len = rng.random_range(min_len..=max_len);
    (0..len).map(|_| pool[rng.random_range(0..pool.len())]).collect()
}

fn random_header_value<R: Rng>(rng: &mut R) -> Vec<u8> {
    let mut v = random_bytes_from_pool(rng, HEADER_VALUE_POOL, 2, 16);
    if v[0] == b' ' || v[0] == b'\t' { v[0] = b'x'; }
    let last = v.len() - 1;
    if v[last] == b' ' || v[last] == b'\t' { v[last] = b'y'; }
    v
}

fn random_path<R: Rng>(rng: &mut R) -> Vec<u8> {
    random_bytes_from_pool(rng, PATH_BYTE_POOL, 1, 16)
}

fn random_hex_digits<R: Rng>(rng: &mut R) -> Vec<u8> {
    let len = match rng.random_range(0..4) {
        0 => rng.random_range(1..=8),
        1 => rng.random_range(9..=16),
        _ => rng.random_range(17..=24),
    };
    (0..len).map(|_| HEX_POOL[rng.random_range(0..HEX_POOL.len())]).collect()
}

fn random_response<R: Rng>(rng: &mut R) -> Vec<u8> {
    RESPONSE_TEMPLATES[rng.random_range(0..RESPONSE_TEMPLATES.len())].to_vec()
}

// Property-specific generator selection. Each property has its own naming
// scheme, but all wrap into Bytes/MethodInput. We pick the right one in main.

impl<R: Rng> Arbitrary<R> for Bytes {
    fn generate(_rng: &mut R, _n: usize) -> Self {
        // Default fallback: empty. Each property wires its own generator below.
        Bytes(vec![b'a'])
    }
}
impl<R: Rng> Arbitrary<R> for MethodInput {
    fn generate(rng: &mut R, _n: usize) -> Self {
        MethodInput {
            method: random_method(rng),
            sep: METHOD_SEP_POOL[rng.random_range(0..METHOD_SEP_POOL.len())],
        }
    }
}

// Per-property Bytes wrappers — using newtypes around Bytes so each property
// can have its own Arbitrary distribution.
#[derive(Clone)] struct HeaderValueBytes(Vec<u8>);
impl fmt::Debug for HeaderValueBytes { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(f) } }
#[derive(Clone)] struct PathBytes(Vec<u8>);
impl fmt::Debug for PathBytes { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(f) } }
#[derive(Clone)] struct HexBytes(Vec<u8>);
impl fmt::Debug for HexBytes { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(f) } }
#[derive(Clone)] struct ResponseBytes(Vec<u8>);
impl fmt::Debug for ResponseBytes { fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(f) } }

impl<R: Rng> Arbitrary<R> for HeaderValueBytes {
    fn generate(rng: &mut R, _n: usize) -> Self { HeaderValueBytes(random_header_value(rng)) }
}
impl<R: Rng> Arbitrary<R> for PathBytes {
    fn generate(rng: &mut R, _n: usize) -> Self { PathBytes(random_path(rng)) }
}
impl<R: Rng> Arbitrary<R> for HexBytes {
    fn generate(rng: &mut R, _n: usize) -> Self { HexBytes(random_hex_digits(rng)) }
}
impl<R: Rng> Arbitrary<R> for ResponseBytes {
    fn generate(rng: &mut R, _n: usize) -> Self { ResponseBytes(random_response(rng)) }
}

impl<R: Rng> Mutate<R> for HeaderValueBytes {
    fn mutate(&self, rng: &mut R, _n: usize) -> Self {
        // Single-pool-byte swap to keep the value within the bug-triggering distribution.
        let mut v = self.0.clone();
        if !v.is_empty() {
            let i = rng.random_range(0..v.len());
            v[i] = HEADER_VALUE_POOL[rng.random_range(0..HEADER_VALUE_POOL.len())];
        }
        HeaderValueBytes(v)
    }
}
impl<R: Rng> Mutate<R> for PathBytes {
    fn mutate(&self, rng: &mut R, _n: usize) -> Self {
        let mut v = self.0.clone();
        if !v.is_empty() {
            let i = rng.random_range(0..v.len());
            v[i] = PATH_BYTE_POOL[rng.random_range(0..PATH_BYTE_POOL.len())];
        }
        PathBytes(v)
    }
}
impl<R: Rng> Mutate<R> for HexBytes {
    fn mutate(&self, rng: &mut R, _n: usize) -> Self {
        let mut v = self.0.clone();
        if !v.is_empty() {
            let i = rng.random_range(0..v.len());
            v[i] = HEX_POOL[rng.random_range(0..HEX_POOL.len())];
        }
        HexBytes(v)
    }
}
impl<R: Rng> Mutate<R> for ResponseBytes {
    fn mutate(&self, rng: &mut R, _n: usize) -> Self {
        // Re-pick template; keeps shape valid.
        ResponseBytes(RESPONSE_TEMPLATES[rng.random_range(0..RESPONSE_TEMPLATES.len())].to_vec())
    }
}

fn mutate_bytes<R: Rng>(rng: &mut R, v: &[u8], max: usize) -> Vec<u8> {
    let mut out = v.to_vec();
    match rng.random_range(0u8..3) {
        0 if !out.is_empty() => {
            let i = rng.random_range(0..out.len());
            let b = rng.random_range(0u32..8);
            out[i] ^= 1u8 << b;
        },
        1 if out.len() < max => out.push(rng.random_range(0u16..=255) as u8),
        _ if out.len() > 1 => { out.pop(); },
        _ => {},
    }
    out
}

impl<R: Rng> Mutate<R> for Bytes {
    fn mutate(&self, rng: &mut R, _n: usize) -> Self { Bytes(mutate_bytes(rng, &self.0, 64)) }
}
impl<R: Rng> Mutate<R> for MethodInput {
    fn mutate(&self, rng: &mut R, _n: usize) -> Self {
        let mut out = self.clone();
        if rng.random_bool(0.7) {
            out.method = mutate_bytes(rng, &out.method, 8);
        } else {
            let b = rng.random_range(0u32..8); out.sep ^= 1u8 << b;
        }
        out
    }
}

fn to_opt(r: PropertyResult) -> Option<bool> {
    match r {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 3 { return; }
    let result = match (args[1].as_str(), args[2].as_str()) {
        ("crabcheck", "RequestMethodIsValid") => {
            quickcheck(|i: MethodInput| to_opt(property_request_method_is_valid(i.method, i.sep)))
        },
        ("crabcheck", "RequestHeaderValuePreserves") => {
            quickcheck(|HeaderValueBytes(v)| to_opt(property_request_header_value_preserves(v)))
        },
        ("crabcheck", "RequestPathPreserves") => {
            quickcheck(|PathBytes(v)| to_opt(property_request_path_preserves(v)))
        },
        ("crabcheck", "ChunkSizeMatchesOracle") => {
            quickcheck(|HexBytes(v)| to_opt(property_chunk_size_matches_oracle(v)))
        },
        ("crabcheck", "ResponseHeaderNamesAreTokens") => {
            quickcheck(|ResponseBytes(v)| to_opt(property_response_header_names_are_tokens(v)))
        },
        (a, b) => panic!("Unknown: {a} {b}"),
    };
    println!("Result: {:?}", result);
}
