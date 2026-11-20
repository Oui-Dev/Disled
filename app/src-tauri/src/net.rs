// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

//! The downloads of the app that the updater does not make: the firmware
//! files of the releases and the catalog of plugins, from GitHub, which
//! wants a user agent. Only the user's requests and the daily check of the
//! firmware reach the network.

use std::time::Duration;

use tauri::AppHandle;

/// Asks for a file.
pub async fn fetch(app: &AppHandle, url: &str) -> Result<reqwest::Response, String> {
    // reqwest is built without a TLS provider, `rustls-no-provider`, so this
    // installs ring, the one the updater uses, unless a provider is there.
    if rustls::crypto::CryptoProvider::get_default().is_none() {
        let _ = rustls::crypto::ring::default_provider().install_default();
    }
    let client = reqwest::Client::builder()
        .user_agent(format!("Disled/{}", app.package_info().version))
        .connect_timeout(Duration::from_secs(15))
        .timeout(Duration::from_secs(60))
        .build()
        .map_err(|e| e.to_string())?;
    client
        .get(url)
        .send()
        .await
        .and_then(|response| response.error_for_status())
        .map_err(|e| e.to_string())
}

/// Downloads a file.
pub async fn get(app: &AppHandle, url: &str) -> Result<Vec<u8>, String> {
    let body = fetch(app, url)
        .await?
        .bytes()
        .await
        .map_err(|e| e.to_string())?;
    Ok(body.to_vec())
}

/// Downloads a file of `max` bytes at most, and stops past them, whatever
/// the server says of its size.
pub async fn get_at_most(app: &AppHandle, url: &str, max: usize) -> Result<Vec<u8>, String> {
    let mut response = fetch(app, url).await?;
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|e| e.to_string())? {
        if body.len() + chunk.len() > max {
            return Err(format!("{url} is larger than {max} bytes"));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}
