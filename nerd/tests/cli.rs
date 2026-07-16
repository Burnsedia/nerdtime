// SPDX-License-Identifier: AGPL-3.0-only
use assert_cmd::Command;
use predicates::prelude::*;

fn nerd_bin() -> Command {
    let path = assert_cmd::cargo_bin!("nerd");
    Command::new(path)
}

#[test]
fn test_cli_help() {
    nerd_bin()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage"));
}

#[test]
fn test_cli_version() {
    nerd_bin()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}

#[test]
fn test_cli_status() {
    nerd_bin()
        .arg("status")
        .assert()
        .success();
}

#[test]
fn test_cli_unknown_command() {
    nerd_bin()
        .arg("this-command-does-not-exist")
        .assert()
        .failure();
}

#[test]
fn test_cli_heatmap_help() {
    nerd_bin()
        .args(["heatmap", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("days"));
}

#[test]
fn test_cli_devlog_list_empty() {
    nerd_bin()
        .args(["devlog", "list", "--limit", "5"])
        .assert()
        .success();
}

#[test]
fn test_cli_task_list_empty() {
    nerd_bin()
        .args(["task", "list"])
        .assert()
        .success();
}

#[test]
fn test_cli_estimate_help() {
    nerd_bin()
        .arg("estimate").arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("project"));
}

#[test]
fn test_cli_summary_help() {
    nerd_bin()
        .args(["summary", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("days"));
}

#[test]
fn test_cli_insights_help() {
    nerd_bin()
        .args(["insights", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("days"));
}
