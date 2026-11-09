// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The tokens of the plugins with events: `dl_` and 24 characters in base
//! 62, about 143 bits, one for each plugin. They live in a file of their own
//! that only the user may read, never in the settings, whose values every
//! window gets.

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;

const PREFIX: &str = "dl_";
const LENGTH: usize = 24;
const ALPHABET: &[u8; 62] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz";

/// A new token. Bytes of 248 and more are drawn again, so that every
/// character is as likely as the others.
pub fn generate() -> io::Result<String> {
    let mut token = String::from(PREFIX);
    while token.len() < PREFIX.len() + LENGTH {
        let mut bytes = [0u8; 32];
        getrandom::fill(&mut bytes).map_err(|error| io::Error::other(error.to_string()))?;
        for byte in bytes.into_iter().filter(|byte| *byte < 248) {
            if token.len() == PREFIX.len() + LENGTH {
                break;
            }
            token.push(char::from(ALPHABET[usize::from(byte % 62)]));
        }
    }
    Ok(token)
}

/// The tokens by plugin, and their file.
#[derive(Debug)]
pub struct Tokens {
    path: PathBuf,
    tokens: BTreeMap<String, String>,
}

impl Tokens {
    /// The tokens of the file, none when it is missing or unreadable.
    pub fn load(path: PathBuf) -> Self {
        let tokens = fs::read_to_string(&path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default();
        Self { path, tokens }
    }

    pub fn get(&self, id: &str) -> Option<&str> {
        self.tokens.get(id).map(String::as_str)
    }

    /// The token of a plugin, made at its first approval and kept through
    /// its updates.
    pub fn ensure(&mut self, id: &str) -> io::Result<String> {
        if let Some(token) = self.tokens.get(id) {
            return Ok(token.clone());
        }
        self.regenerate(id)
    }

    /// A new token, which ends the old one.
    pub fn regenerate(&mut self, id: &str) -> io::Result<String> {
        let token = generate()?;
        self.tokens.insert(id.to_owned(), token.clone());
        self.save()?;
        Ok(token)
    }

    pub fn remove(&mut self, id: &str) -> io::Result<()> {
        if self.tokens.remove(id).is_some() {
            self.save()?;
        }
        Ok(())
    }

    /// Writes a temporary file that only the user may read, then puts it in
    /// place, so that a crash never leaves half a file.
    fn save(&self) -> io::Result<()> {
        if let Some(folder) = self.path.parent() {
            fs::create_dir_all(folder)?;
        }
        let temporary = self.path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(&self.tokens).map_err(io::Error::other)?;
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        io::Write::write_all(&mut options.open(&temporary)?, text.as_bytes())?;
        fs::rename(&temporary, &self.path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_token_has_its_prefix_and_24_characters_of_base_62() {
        let token = generate().expect("token");
        assert_eq!(token.len(), 27);
        assert!(token.starts_with("dl_"));
        assert!(token[3..].bytes().all(|byte| byte.is_ascii_alphanumeric()));
        assert_ne!(generate().expect("token"), token);
    }

    #[test]
    fn tokens_survive_in_their_file() {
        let folder = tempfile::tempdir().expect("folder");
        let path = folder.path().join("plugin-tokens.json");
        let mut tokens = Tokens::load(path.clone());
        let first = tokens.ensure("claude-code").expect("token");
        assert_eq!(tokens.ensure("claude-code").expect("token"), first, "kept");
        let reloaded = Tokens::load(path.clone());
        assert_eq!(reloaded.get("claude-code"), Some(first.as_str()));
        let second = tokens.regenerate("claude-code").expect("token");
        assert_ne!(second, first);
        tokens.remove("claude-code").expect("remove");
        assert_eq!(Tokens::load(path.clone()).get("claude-code"), None);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).expect("file").permissions().mode();
            assert_eq!(mode & 0o777, 0o600);
        }
    }
}
