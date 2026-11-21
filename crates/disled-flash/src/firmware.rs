// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The firmware files of the releases: the release build `disled-fw-X.Y.Z.hex`,
//! signed with the update key, its name and version in the signed comment, since
//! a replaced firmware could present the lamp to the computer as a keyboard.

use semver::Version;

use crate::signature::{Refusal, verify_signed};

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
    let file = format!("file:{}", file_name(version));
    let signed_version = format!("version:{version}");
    verify_signed(key, data, signature, &[&file, &signed_version]).map_err(
        |refusal| match refusal {
            Refusal::UnreadableKey => "the update key is unreadable".into(),
            Refusal::UnreadableSignature => "the firmware signature is unreadable".into(),
            Refusal::NotSigned => "the firmware is not signed with the update key".into(),
            Refusal::OtherFile => format!("the firmware signature is not for version {version}"),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signature::tests::{TEST_FILE, TEST_KEY, TEST_SIGNATURE};

    #[test]
    fn a_signed_firmware_passes() {
        assert_eq!(
            verify_with(TEST_KEY, TEST_FILE, TEST_SIGNATURE, &Version::new(9, 8, 7)),
            Ok(())
        );
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
