// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The files signed with `tauri signer`, whose keys and signatures are
//! minisign ones in base64. The trusted comment of a signature names what it
//! signs, so that another file signed with the same key cannot pass for the
//! one expected.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use minisign_verify::{PublicKey, Signature};

/// Why a signed file is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    UnreadableKey,
    UnreadableSignature,
    /// The key did not sign these bytes.
    NotSigned,
    /// The trusted comment lacks a part asked for: the key signed another
    /// file.
    OtherFile,
}

/// Checks that `key` signed `data`, and that the trusted comment of
/// `signature`, the content of the `.sig` file, holds every part of
/// `comment`, such as `file:index.json`.
pub fn verify_signed(
    key: &str,
    data: &[u8],
    signature: &str,
    comment: &[&str],
) -> Result<(), Refusal> {
    let text = |base64: &str| {
        BASE64
            .decode(base64.trim())
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
    };
    let key = text(key)
        .and_then(|key| PublicKey::decode(&key).ok())
        .ok_or(Refusal::UnreadableKey)?;
    let signature = text(signature)
        .and_then(|signature| Signature::decode(&signature).ok())
        .ok_or(Refusal::UnreadableSignature)?;
    key.verify(data, &signature, false)
        .map_err(|_| Refusal::NotSigned)?;
    let parts: Vec<&str> = signature.trusted_comment().split('\t').collect();
    if comment.iter().all(|part| parts.contains(part)) {
        Ok(())
    } else {
        Err(Refusal::OtherFile)
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    // `tauri signer sign --app-version 9.8.7 disled-fw-9.8.7.hex`, with a
    // throwaway key.
    pub(crate) const TEST_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEUxM0IzMzY2MjQ4OTA3MTYKUldRV0I0a2taak03NFoydUxWY3JaZzRONWloUTBMVkJQem5XNmErS1dUQTNWUjYwZCtYcDNpeTQK";
    pub(crate) const TEST_FILE: &[u8] = b":020000040800F2\n:00000001FF\n";
    pub(crate) const TEST_SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVRV0I0a2taak03NGE0NE80MWlpQ0g4d1Q3MjNQYnJtRnA4dmFHaEMzcXl4K1lQTW04T1J3VWgwdng3SFVGU2o2aUR0TS9PRWtJeUNoYVFESUUwOGRVUVJ3R0RQWjRPUlFNPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzkyNTUzMTczCWZpbGU6ZGlzbGVkLWZ3LTkuOC43LmhleAl2ZXJzaW9uOjkuOC43CnAzN2RyZ2FZUVE0MXlZS1NRcStxYUFzUzN6WSthQU1GeXhrSlFCOVczUGFkekZkeXhtQ2ZBUVE0K0tjQndFNklNSTJDall0b0hYSlU5VWlQWThldENRPT0K";

    #[test]
    fn a_file_passes_only_signed_by_the_key_and_named_in_the_comment() {
        let file = ["file:disled-fw-9.8.7.hex"];
        assert_eq!(
            verify_signed(TEST_KEY, TEST_FILE, TEST_SIGNATURE, &file),
            Ok(())
        );
        let mut changed = TEST_FILE.to_vec();
        changed[1] = b'1';
        assert_eq!(
            verify_signed(TEST_KEY, &changed, TEST_SIGNATURE, &file),
            Err(Refusal::NotSigned)
        );
        assert_eq!(
            verify_signed(TEST_KEY, TEST_FILE, TEST_SIGNATURE, &["file:other.hex"]),
            Err(Refusal::OtherFile)
        );
        assert_eq!(
            verify_signed("", TEST_FILE, TEST_SIGNATURE, &file),
            Err(Refusal::UnreadableKey)
        );
        assert_eq!(
            verify_signed(TEST_KEY, TEST_FILE, "not a signature", &file),
            Err(Refusal::UnreadableSignature)
        );
    }
}
