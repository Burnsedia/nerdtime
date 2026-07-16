// SPDX-License-Identifier: AGPL-3.0-only
use anyhow::Result;
use nerdtime_core::{HeatmapCell, Insights};
use rusqlite::{params, Connection};

pub fn heatmap_data(
    conn: &Connection,
    days: i64,
    project: Option<&str>,
) -> Result<Vec<HeatmapCell>> {
    let mut sql = String::from(
        "SELECT CAST(strftime('%w', started_at) AS INTEGER) as day,
                CAST(strftime('%H', started_at) AS INTEGER) as hour,
                SUM(CAST((julianday(ended_at) - julianday(started_at)) * 86400 AS INTEGER)) as total_seconds
         FROM sessions
         WHERE started_at >= datetime('now', ?1 || ' days', 'localtime')
           AND ended_at IS NOT NULL",
    );
    if project.is_some() {
        sql.push_str(" AND project_name = ?2");
    }
    sql.push_str(" GROUP BY day, hour ORDER BY day, hour");

    let mut stmt = conn.prepare(&sql)?;
    let day_str = format!("-{}", days);

    let rows: Vec<HeatmapCell> = if let Some(p) = project {
        stmt.query_map(params![day_str, p], |row| {
            Ok(HeatmapCell {
                day: row.get(0)?,
                hour: row.get(1)?,
                total_seconds: row.get(2)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect()
    } else {
        stmt.query_map(params![day_str], |row| {
            Ok(HeatmapCell {
                day: row.get(0)?,
                hour: row.get(1)?,
                total_seconds: row.get(2)?,
            })
        })?
        .filter_map(|r| r.ok())
        .collect()
    };

    Ok(rows)
}

pub fn insights_data(conn: &Connection, days: i64, project: Option<&str>) -> Result<Insights> {
    let mut sql = String::from(
        "SELECT project_name, started_at, ended_at FROM sessions
         WHERE started_at >= datetime('now', ?1 || ' days', 'localtime')
           AND ended_at IS NOT NULL",
    );
    if project.is_some() {
        sql.push_str(" AND project_name = ?2");
    }

    let mut stmt = conn.prepare(&sql)?;

    let rows: Vec<(
        String,
        chrono::DateTime<chrono::Utc>,
        chrono::DateTime<chrono::Utc>,
    )> = if let Some(p) = project {
        stmt.query_map(params![format!("-{}", days), p], |row| {
            let started: String = row.get(1)?;
            let ended: String = row.get(2)?;
            Ok((
                row.get::<_, String>(0)?,
                chrono::DateTime::parse_from_rfc3339(&started)
                    .map(|d| d.to_utc())
                    .unwrap_or_default(),
                chrono::DateTime::parse_from_rfc3339(&ended)
                    .map(|d| d.to_utc())
                    .unwrap_or_default(),
            ))
        })?
        .filter_map(|r| r.ok())
        .collect()
    } else {
        stmt.query_map(params![format!("-{}", days)], |row| {
            let started: String = row.get(1)?;
            let ended: String = row.get(2)?;
            Ok((
                row.get::<_, String>(0)?,
                chrono::DateTime::parse_from_rfc3339(&started)
                    .map(|d| d.to_utc())
                    .unwrap_or_default(),
                chrono::DateTime::parse_from_rfc3339(&ended)
                    .map(|d| d.to_utc())
                    .unwrap_or_default(),
            ))
        })?
        .filter_map(|r| r.ok())
        .collect()
    };

    let session_count = rows.len() as i64;
    let mut total_seconds: i64 = 0;
    let mut per_block = [0i64; 4];
    let mut per_day_of_week = [0i64; 7];
    let mut project_map: std::collections::HashMap<String, i64> = std::collections::HashMap::new();

    for (proj, started, ended) in &rows {
        let dur = (*ended - *started).num_seconds();
        total_seconds += dur;

        let hour = started.format("%H").to_string().parse::<u32>().unwrap_or(0);
        let block_idx = match hour {
            6..=11 => 0,
            12..=17 => 1,
            18..=23 => 2,
            _ => 3,
        };
        per_block[block_idx] += dur;

        let dow = started
            .format("%w")
            .to_string()
            .parse::<usize>()
            .unwrap_or(0);
        per_day_of_week[dow] += dur;

        *project_map.entry(proj.clone()).or_insert(0) += dur;
    }

    let mut per_project: Vec<(String, i64)> = project_map.into_iter().collect();
    per_project.sort_by_key(|b| std::cmp::Reverse(b.1));

    Ok(Insights {
        total_seconds,
        session_count,
        per_block,
        per_day_of_week,
        per_project,
    })
}

pub fn label_summary(
    conn: &Connection,
    project: Option<&str>,
    label_filter: Option<&str>,
    start_date: &str,
    end_date: &str,
) -> Result<Vec<(String, i64, Vec<String>)>> {
    let mut sql = String::from(
        "SELECT j.value as label, SUM(CAST((julianday(s.ended_at) - julianday(s.started_at)) * 86400 AS INTEGER)) as total_seconds, s.project_name FROM sessions s, json_each(COALESCE(s.labels, '[]')) AS j WHERE s.ended_at IS NOT NULL AND s.started_at >= ?1 AND s.started_at <= ?2",
    );
    if let Some(p) = project {
        sql.push_str(&format!(
            " AND s.project_name = '{}'",
            p.replace('\'', "''")
        ));
    }
    sql.push_str(" GROUP BY j.value, s.project_name ORDER BY total_seconds DESC");

    let mut stmt = conn.prepare(&sql)?;
    let mut map: std::collections::HashMap<String, (i64, Vec<String>)> =
        std::collections::HashMap::new();
    for row in stmt
        .query_map(params![start_date, end_date], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })?
        .filter_map(|r| r.ok())
    {
        let (lbl, seconds, proj) = row;
        if lbl.is_empty() {
            continue;
        }
        if let Some(f) = label_filter {
            if lbl != f {
                continue;
            }
        }
        let entry = map.entry(lbl).or_insert((0, Vec::new()));
        entry.0 += seconds;
        if !entry.1.contains(&proj) {
            entry.1.push(proj);
        }
    }

    let mut result: Vec<(String, i64, Vec<String>)> =
        map.into_iter().map(|(k, (s, p))| (k, s, p)).collect();
    result.sort_by_key(|b| std::cmp::Reverse(b.1));
    Ok(result)
}
