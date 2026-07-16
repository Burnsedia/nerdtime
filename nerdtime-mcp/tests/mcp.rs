// SPDX-License-Identifier: AGPL-3.0-only
use std::io::{BufRead, Read, Write};
use std::process::Stdio;
use std::sync::mpsc;
use std::time::Duration;

use serial_test::serial;

fn call_tool(name: &str, args: &str) -> serde_json::Value {
    let path = assert_cmd::cargo_bin!("nerdtime-mcp");
    let mut child = std::process::Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn nerdtime-mcp");

    let mut stdin = child.stdin.take().expect("stdin not available");
    let stdout = child.stdout.take().expect("stdout not available");
    let mut reader = std::io::BufReader::new(stdout);

    // Send initialize request
    stdin
        .write_all(
            b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"0.1.0\",\"capabilities\":{},\"clientInfo\":{\"name\":\"test\",\"version\":\"1.0\"}}}\n",
        )
        .unwrap();
    stdin.flush().unwrap();

    // Read init response line
    let mut line = String::new();
    reader.read_line(&mut line).expect("no init response");

    // Send tool call
    let request = format!(
        r#"{{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{{"name":"{}","arguments":{}}}}}"#,
        name, args
    );
    writeln!(stdin, "{}", request).unwrap();
    stdin.flush().unwrap();

    // Close stdin so server exits after processing
    drop(stdin);

    // Read remaining stdout on a thread so we can enforce a timeout
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut output = String::new();
        let _ = reader.read_to_string(&mut output);
        let _ = tx.send(output);
    });

    let output = match rx.recv_timeout(Duration::from_secs(10)) {
        Ok(o) => o,
        Err(_) => {
            let _ = child.kill();
            let _ = child.wait();
            return serde_json::json!({"error": "timeout"});
        }
    };

    let _ = child.wait();

    // Find the response with id=2 (the tool call response)
    for l in output.lines() {
        let l = l.trim();
        if l.is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(l) {
            if v.get("id") == Some(&serde_json::json!(2)) {
                return v;
            }
        }
    }

    serde_json::json!({"error": "no response"})
}

#[test]
fn test_mcp_initialize() {
    let path = assert_cmd::cargo_bin!("nerdtime-mcp");
    let mut child = std::process::Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn nerdtime-mcp");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = std::io::BufReader::new(stdout);

    stdin
        .write_all(
            b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"0.1.0\",\"capabilities\":{},\"clientInfo\":{\"name\":\"test\",\"version\":\"1.0\"}}}\n",
        )
        .unwrap();
    stdin.flush().unwrap();

    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    drop(stdin);
    child.wait().ok();

    let resp: serde_json::Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(resp["jsonrpc"], "2.0");
}

#[test]
fn test_mcp_tool_list() {
    let path = assert_cmd::cargo_bin!("nerdtime-mcp");
    let mut child = std::process::Command::new(path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to spawn nerdtime-mcp");

    let mut stdin = child.stdin.take().unwrap();
    let stdout = child.stdout.take().unwrap();
    let mut reader = std::io::BufReader::new(stdout);

    stdin
        .write_all(
            b"{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"initialize\",\"params\":{\"protocolVersion\":\"0.1.0\",\"capabilities\":{},\"clientInfo\":{\"name\":\"test\",\"version\":\"1.0\"}}}\n",
        )
        .unwrap();
    stdin.flush().unwrap();

    let mut line = String::new();
    reader.read_line(&mut line).unwrap();

    writeln!(
        stdin,
        r#"{{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{{}}}}"#
    )
    .unwrap();
    stdin.flush().unwrap();
    drop(stdin);

    let mut output = String::new();
    reader.read_to_string(&mut output).unwrap();
    child.wait().ok();

    let names: Vec<String> = output
        .lines()
        .filter_map(|l| {
            let l = l.trim();
            if l.is_empty() {
                return None;
            }
            let v: serde_json::Value = serde_json::from_str(l).ok()?;
            if v.get("id") != Some(&serde_json::json!(2)) {
                return None;
            }
            v["result"]["tools"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|t| t["name"].as_str().map(|s| s.to_string()))
                        .collect::<Vec<_>>()
                })
        })
        .flatten()
        .collect::<Vec<_>>();

    assert!(names.contains(&"get_status".to_string()), "missing get_status");
    assert!(names.contains(&"start_tracking".to_string()), "missing start_tracking");
    assert!(names.contains(&"stop_tracking".to_string()), "missing stop_tracking");
    assert!(names.contains(&"list_sessions".to_string()), "missing list_sessions");
    assert!(names.contains(&"task_create".to_string()), "missing task_create");
    assert!(names.contains(&"task_list".to_string()), "missing task_list");
    assert!(names.contains(&"devlog_log".to_string()), "missing devlog_log");
    assert!(names.contains(&"devlog_query".to_string()), "missing devlog_query");
    assert!(names.contains(&"devlog_generate".to_string()), "missing devlog_generate");
    assert!(names.contains(&"what_should_i_work_on".to_string()), "missing what_should_i_work_on");
    assert!(names.contains(&"sync".to_string()), "missing sync");
    assert!(names.contains(&"get_stats".to_string()), "missing get_stats");
    assert!(names.contains(&"task_matrix".to_string()), "missing task_matrix");
}

#[test]
fn test_mcp_get_status() {
    let resp = call_tool("get_status", r#"{}"#);
    assert!(!resp.get("error").is_some(), "error: {:?}", resp);
}

#[test]
#[serial]
fn test_mcp_session_start_stop() {
    let start = call_tool("start_tracking", r#"{"project":"test-proj"}"#);
    assert!(!start.get("error").is_some(), "start failed: {:?}", start);

    let status = call_tool("get_status", r#"{}"#);
    let text = status["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("");
    assert!(text.contains("test-proj"), "should show test-proj: {}", text);

    let stop = call_tool("stop_tracking", r#"{}"#);
    assert!(!stop.get("error").is_some(), "stop failed: {:?}", stop);
}

#[test]
#[serial]
fn test_mcp_session_list() {
    call_tool("start_tracking", r#"{"project":"list-test"}"#);
    call_tool("stop_tracking", r#"{}"#);

    let list = call_tool("list_sessions", r#"{"limit":10}"#);
    let text = list["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("");
    assert!(text.contains("list-test"), "should find list-test: {}", text);
}

#[test]
#[serial]
fn test_mcp_task_add_list() {
    let add = call_tool(
        "task_create",
        r#"{"project":"test","title":"MCP task","estimate":"1h"}"#,
    );
    assert!(!add.get("error").is_some(), "add failed: {:?}", add);

    let list = call_tool("task_list", r#"{"project":"test"}"#);
    let text = list["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("");
    assert!(
        text.contains("MCP task"),
        "list should contain MCP task: {}",
        text
    );
}

#[test]
#[serial]
fn test_mcp_devlog_log() {
    let log = call_tool("devlog_log", r#"{"text":"MCP devlog test","tags":"mcp,test"}"#);
    let text = log["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("");
    assert!(text.contains("Logged"), "should confirm log: {}", text);
}

#[test]
#[serial]
fn test_mcp_devlog_query() {
    call_tool("devlog_log", r#"{"text":"Queryable entry","tags":"test"}"#);

    let query = call_tool("devlog_query", r#"{"text":"Queryable"}"#);
    let text = query["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("");
    assert!(
        text.contains("Queryable"),
        "query should find entry: {}",
        text
    );
}

#[test]
#[serial]
fn test_mcp_devlog_generate() {
    call_tool("devlog_log", r#"{"text":"For generate test"}"#);

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("DEVLOG.md");
    let gen = call_tool(
        "devlog_generate",
        &format!(r#"{{"output_path":"{}"}}"#, path.display()),
    );
    assert!(!gen.get("error").is_some(), "generate failed: {:?}", gen);
    assert!(path.exists(), "DEVLOG.md should exist");
}

#[test]
#[serial]
fn test_mcp_advisor_decide() {
    call_tool(
        "task_create",
        r#"{"project":"test","title":"Advisor task","estimate":"2h","q1":true}"#,
    );

    let decide = call_tool(
        "what_should_i_work_on",
        r#"{"available_seconds":7200,"energy":"high"}"#,
    );
    assert!(!decide.get("error").is_some(), "decide failed: {:?}", decide);
}

#[test]
#[serial]
fn test_mcp_sync() {
    let sync = call_tool("sync", r#"{}"#);
    // Accept any valid response — sync may succeed, find nothing, or error
    if let Some(result) = sync.get("result") {
        let text = result["content"][0]["text"].as_str().unwrap_or("");
        assert!(
            !text.is_empty(),
            "sync result should have content text: {:?}",
            result
        );
    } else if let Some(err) = sync.get("error") {
        assert!(
            err.get("message").is_some(),
            "sync error should have message: {:?}",
            err
        );
    } else {
        panic!("unexpected sync response: {:?}", sync);
    }
}
