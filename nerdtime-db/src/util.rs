// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;

pub fn parse_duration(input: &str) -> Result<Option<i64>> {
    let input = input.trim().to_lowercase();
    if input == "0" || input == "none" || input.is_empty() {
        return Ok(None);
    }
    if let Some(s) = input.strip_suffix('h') {
        let s = s.trim();
        if let Ok(h) = s.parse::<f64>() {
            return Ok(Some((h * 3600.0) as i64));
        }
        if s.contains('m') {
            let parts: Vec<&str> = s.splitn(2, 'm').collect();
            if parts.len() == 2 {
                let h = parts[0].parse::<f64>().unwrap_or(0.0);
                let m = parts[1].parse::<f64>().unwrap_or(0.0);
                return Ok(Some((h * 3600.0 + m * 60.0) as i64));
            }
        }
    }
    if let Some(s) = input.strip_suffix('m') {
        let s = s.trim();
        if let Ok(m) = s.parse::<f64>() {
            return Ok(Some((m * 60.0) as i64));
        }
    }
    anyhow::bail!("invalid duration: {}", input)
}

pub fn fmt_duration(seconds: i64) -> String {
    let hours = seconds / 3600;
    let mins = (seconds % 3600) / 60;
    let secs = seconds % 60;
    if hours > 0 {
        format!("{}h {:02}m", hours, mins)
    } else if mins > 0 {
        format!("{}m", mins)
    } else {
        format!("{}s", secs)
    }
}
