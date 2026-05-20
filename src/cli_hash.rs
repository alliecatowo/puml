//! `puml hash` subcommand — print a deterministic content hash of a file.
//!
//! Currently uses FNV-1a 64-bit over the raw file bytes, encoded as lowercase
//! hex or base64.  The `sha256` and `blake3` algorithm options are reserved for
//! future backends; selecting them today returns an error.
//!
//! No external crates required.

use crate::cli::{HashAlgoArg, HashArgs, HashFormatArg};

/// Run the `puml hash` subcommand.
///
/// Reads `args.file`, hashes its raw bytes, and prints the digest to stdout.
///
/// Returns `Ok(0)` on success; `Err((exit_code, message))` on failure.
pub fn run_hash(args: &HashArgs) -> Result<i32, (i32, String)> {
    let bytes = std::fs::read(&args.file).map_err(|e| {
        (
            2_i32,
            format!("failed to read '{}': {e}", args.file.display()),
        )
    })?;

    let digest = match args.algo {
        HashAlgoArg::Sha256 => {
            // TODO: wire in a sha256 backend (ring / sha2 crate) once the
            // dependency decision is made.
            return Err((
                1_i32,
                "--algo sha256 is not yet implemented; use --algo fnv for now \
                 (sha256 backend is planned)"
                    .to_string(),
            ));
        }
        HashAlgoArg::Blake3 => {
            // TODO: wire in the blake3 crate once the dependency decision is made.
            return Err((
                1_i32,
                "--algo blake3 is not yet implemented; use --algo fnv for now \
                 (blake3 backend is planned)"
                    .to_string(),
            ));
        }
        HashAlgoArg::Fnv => fnv1a_hex(&bytes),
    };

    let output = match args.format {
        HashFormatArg::Hex => digest,
        HashFormatArg::Base64 => base64_encode(digest.as_bytes()),
    };

    println!("{output}");
    Ok(0)
}

/// Compute a stable FNV-1a 64-bit hash over `data` and return it as lowercase hex.
fn fnv1a_hex(data: &[u8]) -> String {
    const FNV_OFFSET_BASIS: u64 = 14695981039346656037;
    const FNV_PRIME: u64 = 1099511628211;
    let mut hash = FNV_OFFSET_BASIS;
    for &byte in data {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    format!("{hash:016x}")
}

/// Minimal RFC 4648 standard base64 encoder with padding.
fn base64_encode(input: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(input.len().div_ceil(3) * 4);
    for chunk in input.chunks(3) {
        let b0 = chunk[0];
        let b1 = if chunk.len() > 1 { chunk[1] } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] } else { 0 };
        let n = (u32::from(b0) << 16) | (u32::from(b1) << 8) | u32::from(b2);
        out.push(char::from(ALPHABET[((n >> 18) & 0x3f) as usize]));
        out.push(char::from(ALPHABET[((n >> 12) & 0x3f) as usize]));
        if chunk.len() > 1 {
            out.push(char::from(ALPHABET[((n >> 6) & 0x3f) as usize]));
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(char::from(ALPHABET[(n & 0x3f) as usize]));
        } else {
            out.push('=');
        }
    }
    out
}
