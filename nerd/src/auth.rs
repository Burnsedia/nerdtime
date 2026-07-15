// SPDX-License-Identifier: AGPL-3.0-only

use anyhow::{Context, Result};
use colored::Colorize;
use reqwest::blocking::Client;
use serde::Deserialize;
use serde::Serialize;

use crate::config;

#[derive(Serialize)]
struct LoginRequest {
    email: String,
    password: String,
}

#[derive(Serialize)]
struct RegisterRequest {
    email: String,
    password: String,
    name: String,
}

#[derive(Deserialize)]
struct AuthResponse {
    token: String,
    user: UserInfo,
}

#[derive(Deserialize)]
struct UserInfo {
    #[allow(dead_code)]
    #[serde(default)]
    id: i64,
    email: String,
    #[allow(dead_code)]
    #[serde(default)]
    name: String,
}

pub fn login(email: &str, password: &str) -> Result<()> {
    let cfg = config::load()?;
    let url = format!("{}/auth/login", cfg.api_url.trim_end_matches('/'));

    let client = Client::new();
    let resp = client
        .post(&url)
        .json(&LoginRequest {
            email: email.to_string(),
            password: password.to_string(),
        })
        .send()
        .context("Failed to connect to server")?;

    if resp.status().is_success() {
        let body: AuthResponse = resp.json()?;
        let mut cfg = config::load()?;
        cfg.token = Some(body.token);
        cfg.user_email = Some(body.user.email.clone());
        config::save(&cfg)?;
        println!("{} Logged in as {}", "✓".green(), body.user.email);
        Ok(())
    } else if resp.status().as_u16() == 401 {
        anyhow::bail!("Invalid email or password.");
    } else {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        anyhow::bail!("Login failed ({}): {}", status, text);
    }
}

pub fn signup(email: &str, password: &str, name: &str) -> Result<()> {
    let cfg = config::load()?;
    let url = format!("{}/auth/register", cfg.api_url.trim_end_matches('/'));

    let client = Client::new();
    let resp = client
        .post(&url)
        .json(&RegisterRequest {
            email: email.to_string(),
            password: password.to_string(),
            name: name.to_string(),
        })
        .send()
        .context("Failed to connect to server")?;

    if resp.status().is_success() {
        let body: AuthResponse = resp.json()?;
        let mut cfg = config::load()?;
        cfg.token = Some(body.token);
        cfg.user_email = Some(body.user.email.clone());
        config::save(&cfg)?;
        println!(
            "{} Registered! Logged in as {}",
            "✓".green(),
            body.user.email
        );
        Ok(())
    } else if resp.status().as_u16() == 422 {
        let text = resp.text().unwrap_or_default();
        anyhow::bail!("Registration failed: {}", text);
    } else {
        let status = resp.status();
        let text = resp.text().unwrap_or_default();
        anyhow::bail!("Registration failed ({}): {}", status, text);
    }
}

pub fn logout() -> Result<()> {
    let mut cfg = config::load()?;
    cfg.token = None;
    cfg.user_email = None;
    config::save(&cfg)?;
    println!("{} Logged out.", "✓".green());
    Ok(())
}
