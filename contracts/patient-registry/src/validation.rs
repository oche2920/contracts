//! Pure input validation helpers (no `Env`).
//!
//! These functions are used by the contract and by **fuzz targets** so random
//! inputs never rely on Soroban host APIs. All return `Result` — **no panics**
//! on malformed input.

#[allow(clippy::result_unit_err)]
pub fn validate_cid_bytes(cid: &[u8]) -> Result<(), ()> {
    let len = cid.len();
    if len == 0 || len > 512 {
        return Err(());
    }
    let first = *cid.first().ok_or(())?;
    if first == b'b' {
        return if len >= 36 { Ok(()) } else { Err(()) };
    }
    if len >= 2 {
        let second = cid[1];
        if first == b'Q' && second == b'm' && len == 46 {
            return Ok(());
        }
        if len == 34 && first == 0x12 && second == 0x20 {
            return Ok(());
        }
    }
    Err(())
}

/// Minimal W3C DID Core–style check: UTF-8, `did:` prefix, non-empty method,
/// non-empty method-specific id. Designed to reject garbage without panicking.
#[allow(clippy::result_unit_err, clippy::manual_range_contains)]
pub fn validate_did_bytes(did: &[u8]) -> Result<(), ()> {
    let len = did.len();
    if len < 7 || len > 256 {
        return Err(());
    }
    let s = core::str::from_utf8(did).map_err(|_| ())?;
    if !s.starts_with("did:") {
        return Err(());
    }
    let rest = s.get(4..).ok_or(())?;
    let colon = rest.find(':').ok_or(())?;
    if colon == 0 {
        return Err(());
    }
    let method = rest.get(..colon).ok_or(())?;
    if method.is_empty() {
        return Err(());
    }
    for &b in method.as_bytes() {
        if !b.is_ascii_alphanumeric() && b != b'-' {
            return Err(());
        }
    }
    let id_part = rest.get(colon + 1..).ok_or(())?;
    if id_part.is_empty() || id_part.len() > 200 {
        return Err(());
    }
    Ok(())
}

/// Inclusive clinical-style score band (e.g. 0–100). Extend off-chain docs if the
/// product uses a different scale.
pub const SCORE_MIN: i32 = 0;
pub const SCORE_MAX: i32 = 100;

#[allow(clippy::result_unit_err)]
pub fn validate_score_i32(score: i32) -> Result<(), ()> {
    if (SCORE_MIN..=SCORE_MAX).contains(&score) {
        Ok(())
    } else {
        Err(())
    }
}

#[cfg(test)]
mod proptest_validators {
    //! Property-based checks: validators must never panic (only `Ok` / `Err`).
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn prop_validate_cid_bytes_no_panic(bytes in prop::collection::vec(any::<u8>(), 0..600)) {
            let _ = validate_cid_bytes(&bytes);
        }

        #[test]
        fn prop_validate_did_bytes_no_panic(bytes in prop::collection::vec(any::<u8>(), 0..300)) {
            let _ = validate_did_bytes(&bytes);
        }

        #[test]
        fn prop_validate_score_i32_no_panic(score in any::<i32>()) {
            let _ = validate_score_i32(score);
        }
    }
}
