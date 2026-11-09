// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The firmware files of the releases: the release build `disled-fw-X.Y.Z.hex`,
//! signed with the update key, its name and version in the signed comment, since
//! a replaced firmware could present the lamp to the computer as a keyboard.

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use minisign_verify::{PublicKey, Signature};
use semver::Version;

/// Public half of the update key. The app updater reads the same key from
/// `plugins.updater.pubkey` in `app/src-tauri/tauri.conf.json`; a test
/// keeps both equal.
const UPDATE_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDM1QThCQjQ2ODRCODJGREYKUldUZkw3aUVScnVvTlFvSng1R29rOUYxWFlXTnM5bEZUVU5GNG53d3lNekFQRmozK1RwOGxtYi8K";

/// Name of the release build of a version, as the releases attach it.
pub fn file_name(version: &Version) -> String {
    format!("disled-fw-{version}.hex")
}

/// Name of its signature, attached next to it.
pub fn signature_name(version: &Version) -> String {
    format!("{}.sig", file_name(version))
}

/// The version in the name of a release build.
pub fn version_of(file_name: &str) -> Option<Version> {
    let version = file_name.strip_prefix("disled-fw-")?.strip_suffix(".hex")?;
    Version::parse(version).ok()
}

/// Checks that `data` is the release build of `version`, signed with the
/// update key, so that an older signed file cannot pass for a newer one.
/// `signature` is the content of the `.sig` file.
pub fn verify(data: &[u8], signature: &str, version: &Version) -> Result<(), String> {
    verify_with(UPDATE_KEY, data, signature, version)
}

fn verify_with(key: &str, data: &[u8], signature: &str, version: &Version) -> Result<(), String> {
    let text = |base64: &str| {
        BASE64
            .decode(base64.trim())
            .ok()
            .and_then(|bytes| String::from_utf8(bytes).ok())
    };
    let key = text(key)
        .and_then(|key| PublicKey::decode(&key).ok())
        .ok_or("the update key is unreadable")?;
    let signature = text(signature)
        .and_then(|signature| Signature::decode(&signature).ok())
        .ok_or("the firmware signature is unreadable")?;
    key.verify(data, &signature, false)
        .map_err(|_| "the firmware is not signed with the update key")?;
    let comment: Vec<&str> = signature.trusted_comment().split('\t').collect();
    let file = format!("file:{}", file_name(version));
    let signed_version = format!("version:{version}");
    if comment.contains(&file.as_str()) && comment.contains(&signed_version.as_str()) {
        Ok(())
    } else {
        Err(format!(
            "the firmware signature is not for version {version}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // `tauri signer sign --app-version 9.8.7 disled-fw-9.8.7.hex`, with a
    // throwaway key.
    const TEST_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IEUxM0IzMzY2MjQ4OTA3MTYKUldRV0I0a2taak03NFoydUxWY3JaZzRONWloUTBMVkJQem5XNmErS1dUQTNWUjYwZCtYcDNpeTQK";
    const TEST_FILE: &[u8] = b":020000040800F2\n:00000001FF\n";
    const TEST_SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVRV0I0a2taak03NGE0NE80MWlpQ0g4d1Q3MjNQYnJtRnA4dmFHaEMzcXl4K1lQTW04T1J3VWgwdng3SFVGU2o2aUR0TS9PRWtJeUNoYVFESUUwOGRVUVJ3R0RQWjRPUlFNPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzkyNTUzMTczCWZpbGU6ZGlzbGVkLWZ3LTkuOC43LmhleAl2ZXJzaW9uOjkuOC43CnAzN2RyZ2FZUVE0MXlZS1NRcStxYUFzUzN6WSthQU1GeXhrSlFCOVczUGFkekZkeXhtQ2ZBUVE0K0tjQndFNklNSTJDall0b0hYSlU5VWlQWThldENRPT0K";

    #[test]
    fn a_signed_firmware_passes() {
        assert_eq!(
            verify_with(TEST_KEY, TEST_FILE, TEST_SIGNATURE, &Version::new(9, 8, 7)),
            Ok(())
        );
    }

    #[test]
    fn a_changed_firmware_fails() {
        let mut changed = TEST_FILE.to_vec();
        changed[1] = b'1';
        assert!(verify_with(TEST_KEY, &changed, TEST_SIGNATURE, &Version::new(9, 8, 7)).is_err());
    }

    #[test]
    fn a_signature_for_another_version_fails() {
        assert_eq!(
            verify_with(TEST_KEY, TEST_FILE, TEST_SIGNATURE, &Version::new(9, 8, 8)),
            Err("the firmware signature is not for version 9.8.8".into())
        );
    }

    #[test]
    fn another_key_fails() {
        // The update key did not sign the test file.
        assert_eq!(
            verify(TEST_FILE, TEST_SIGNATURE, &Version::new(9, 8, 7)),
            Err("the firmware is not signed with the update key".into())
        );
    }

    #[test]
    fn the_app_updater_has_the_same_key() {
        let config: serde_json::Value =
            serde_json::from_str(include_str!("../../../app/src-tauri/tauri.conf.json"))
                .expect("tauri.conf.json");
        assert_eq!(config["plugins"]["updater"]["pubkey"], UPDATE_KEY);
    }

    #[test]
    fn file_names_carry_the_version() {
        let version = Version::new(0, 3, 0);
        assert_eq!(file_name(&version), "disled-fw-0.3.0.hex");
        assert_eq!(signature_name(&version), "disled-fw-0.3.0.hex.sig");
        assert_eq!(version_of("disled-fw-0.3.0.hex"), Some(version));
        assert_eq!(version_of("firmware.hex"), None);
        assert_eq!(version_of("disled-fw-0.3.0.bin"), None);
    }
}
