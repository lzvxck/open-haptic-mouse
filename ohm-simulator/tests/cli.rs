use std::process::{Command, Output};

const SINGLE_PRESS: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/scenarios/single_press.json");

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_ohm-simulator")).args(args).output().unwrap()
}

fn stdout(out: &Output) -> String {
    String::from_utf8(out.stdout.clone()).unwrap()
}

#[test]
fn plain_replay_has_no_latency_report() {
    let out = run(&[SINGLE_PRESS]);
    assert!(out.status.success());
    assert!(!stdout(&out).contains("MODELLED"));
}

#[test]
fn latency_flag_first_reports_latency() {
    let out = run(&["--latency", SINGLE_PRESS]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("detect_to_trigger_us="));
}

#[test]
fn latency_flag_last_reports_latency() {
    let out = run(&[SINGLE_PRESS, "--latency"]);
    assert!(out.status.success());
    assert!(stdout(&out).contains("detect_to_trigger_us="));
}

#[test]
fn unknown_flag_is_a_usage_error() {
    let out = run(&["--latenci", SINGLE_PRESS]);
    assert_eq!(out.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&out.stderr).contains("uso:"));
}

#[test]
fn missing_path_is_a_usage_error() {
    assert_eq!(run(&[]).status.code(), Some(2));
    assert_eq!(run(&["--latency"]).status.code(), Some(2));
}

#[test]
fn extra_path_is_a_usage_error() {
    assert_eq!(run(&[SINGLE_PRESS, SINGLE_PRESS]).status.code(), Some(2));
}

#[test]
fn invalid_scenario_exits_1_instead_of_hanging() {
    let dir = std::env::temp_dir().join(format!("ohm-cli-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("zero_period.json");
    let json = std::fs::read_to_string(SINGLE_PRESS).unwrap().replace("\"sample_period_us\": 1000", "\"sample_period_us\": 0");
    std::fs::write(&path, json).unwrap();
    let out = run(&[path.to_str().unwrap()]);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("escenario inválido"));
}
