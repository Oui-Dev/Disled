// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The downloads of the app that the updater does not make: the firmware
//! files of the releases and the catalog of plugins, from GitHub, which
//! wants a user agent. Only the user's requests and the daily check of the
//! firmware reach the network.

use std::time::Duration;

use tauri::AppHandle;

/// Without them, a connection that stalls, as across a sleep, would hold a
/// check, and the automatic checks after it, for good. A read timeout
/// rather than a total one, so that a slow download still ends. The updater
/// takes them too.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
pub const READ_TIMEOUT: Duration = Duration::from_secs(30);
/// A signature of `tauri signer` is far smaller.
pub const MAX_SIGNATURE: usize = 4096;

/// Asks for a file.
pub async fn fetch(app: &AppHandle, url: &str) -> Result<reqwest::Response, String> {
    // reqwest is built without a TLS provider, `rustls-no-provider`, so this
    // installs ring, the one the updater uses, unless a provider is there.
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    let client = reqwest::Client::builder()
        .user_agent(format!("Disled/{}", app.package_info().version))
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;
    client
        .get(url)
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(|e| e.to_string())
}

/// Downloads a file of `max` bytes at most, and stops past them, whatever
/// the server says of its size.
pub async fn get_at_most(app: &AppHandle, url: &str, max: usize) -> Result<Vec<u8>, String> {
    read_at_most(fetch(app, url).await?, max).await
}

/// The body of an answer, if it holds `max` bytes at most.
async fn read_at_most(mut response: reqwest::Response, max: usize) -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if body.len() + chunk.len() > max {
            return Err(format!("{} is larger than {max} bytes", response.url()));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn answer(size: usize) -> reqwest::Response {
        reqwest::Response::from(axum::http::Response::new(vec![0u8; size]))
    }

    #[tokio::test]
    async fn a_body_past_its_limit_is_refused() {
        assert!(read_at_most(answer(11), 10).await.is_err());
        assert_eq!(read_at_most(answer(11), 11).await, Ok(vec![0u8; 11]));
    }
}
