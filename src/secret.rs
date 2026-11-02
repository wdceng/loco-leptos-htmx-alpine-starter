//! A secret handed to a browser and kept only as its digest: a session's id
//! in its cookie (src/models/sessions.rs). The row holds the SHA-256 of
//! what was handed out, so a copy of the database signs nobody in; what
//! comes back is hashed again and looked up by equality. As an islands app
//! on this stack keeps its sessions.
//!
//! Made by Loco's `hash::random_string`, the generator the magic link's
//! token already comes from (src/models/users.rs): one generator for the
//! site's secrets.

use loco_rs::hash;
use sha2::{Digest, Sha256};

/// How many characters a fresh secret has: letters and digits, about 190
/// bits, safe in a cookie as they are.
const LENGTH: usize = 32;

/// A fresh secret, as handed to the browser.
#[must_use]
pub fn fresh() -> String {
    hash::random_string(LENGTH)
}

/// What the row keeps of `secret`: its SHA-256, 64 hexadecimal digits.
#[must_use]
pub fn digest(secret: &str) -> String {
    format!("{:x}", Sha256::digest(secret.as_bytes()))
}

/// Whether `secret` has the shape [`fresh`] gives: one that has is hashed
/// and looked up, anything else never reaches the database.
#[must_use]
pub fn is_secret(secret: &str) -> bool {
    secret.len() == LENGTH && secret.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_secret_has_the_shape_and_is_new_each_time() {
        let one = fresh();
        assert!(is_secret(&one), "{one}");
        assert_ne!(one, fresh());
    }

    #[test]
    fn the_digest_is_sha256_in_hex() {
        // SHA-256 of "abc", FIPS 180-2's own example.
        assert_eq!(
            digest("abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn anything_else_is_no_secret() {
        assert!(!is_secret(""));
        assert!(!is_secret(&"a".repeat(LENGTH - 1)));
        assert!(!is_secret(&format!("{}-", "a".repeat(LENGTH - 1))));
        assert!(!is_secret("'; DROP TABLE sessions; --"));
    }
}
