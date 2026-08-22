use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};

fn smoke(protocol: &str) {
    let mut child = Command::new(env!("CARGO_BIN_EXE_mcp-notifications"))
        .current_dir(std::env::temp_dir())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    for message in [
        serde_json::json!({"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":protocol,"capabilities":{},"clientInfo":{"name":"smoke","version":"1"}}}),
        serde_json::json!({"jsonrpc":"2.0","method":"notifications/initialized"}),
        serde_json::json!({"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}),
        serde_json::json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"list_templates","arguments":{}}}),
    ] {
        writeln!(input, "{message}").unwrap();
    }
    drop(input);
    let responses = BufReader::new(child.stdout.take().unwrap())
        .lines()
        .map(|line| serde_json::from_str::<serde_json::Value>(&line.unwrap()).unwrap())
        .collect::<Vec<_>>();
    assert!(child.wait().unwrap().success());
    let init = responses.iter().find(|value| value["id"] == 1).unwrap();
    assert_eq!(init["result"]["protocolVersion"], protocol);
    assert!(
        init["result"]["capabilities"]["extensions"]["io.modelcontextprotocol/tasks"].is_object()
    );
    let list = responses.iter().find(|value| value["id"] == 2).unwrap();
    let tools = list["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 13);
    assert!(
        tools
            .iter()
            .all(|tool| tool["outputSchema"].is_object() && tool["annotations"].is_object())
    );
    let call = responses.iter().find(|value| value["id"] == 3).unwrap();
    if protocol == "2026-07-28" {
        assert!(call["result"]["structuredContent"].is_array());
    }
}

#[test]
fn legacy() {
    smoke("2025-11-25");
}

#[test]
fn modern() {
    smoke("2026-07-28");
}
