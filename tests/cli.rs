use std::{
    fs,
    path::PathBuf,
    process::{Command, Output},
};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_jsonschema-check"))
        .args(args)
        .output()
        .expect("failed to run jsonschema-check")
}

#[test]
fn help_succeeds_without_files() {
    let output = run(&["--help"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stderr, [] as [u8; 0]);
    let help = String::from_utf8(output.stdout).unwrap();
    for text in [
        "Usage: jsonschema-check [--output jsonl] <schema.json> <data.json>",
        "Arguments:",
        "Options:",
        "--help",
        "--version",
        "--output jsonl",
        "Exit codes:",
    ] {
        assert!(help.contains(text), "help is missing {text}");
    }
}

#[test]
fn version_succeeds_without_files() {
    let output = run(&["--version"]);
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stderr, [] as [u8; 0]);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        format!("jsonschema-check {}\n", env!("CARGO_PKG_VERSION"))
    );
}

#[test]
fn invalid_arguments_report_usage() {
    for args in [
        vec![],
        vec!["schema.json"],
        vec!["schema.json", "data.json", "extra.json"],
        vec!["--help", "schema.json", "data.json"],
        vec!["--version", "schema.json", "data.json"],
        vec!["-h"],
        vec!["-V"],
        vec!["--output"],
        vec!["--output", "jsonl"],
        vec!["--output", "jsonl", "schema.json"],
        vec!["--output", "text", "schema.json", "data.json"],
        vec!["--output=jsonl", "schema.json", "data.json"],
        vec!["schema.json", "--output", "jsonl", "data.json"],
        vec!["schema.json", "data.json", "--output", "jsonl"],
        vec![
            "--output",
            "jsonl",
            "--output",
            "jsonl",
            "schema.json",
            "data.json",
        ],
        vec![
            "--output",
            "jsonl",
            "schema.json",
            "data.json",
            "extra.json",
        ],
        vec!["--output", "data.json"],
        vec!["schema.json", "--output"],
        vec!["--output=jsonl", "data.json"],
    ] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(2), "arguments: {args:?}");
        assert!(output.stdout.is_empty(), "arguments: {args:?}");
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "Error: Usage: jsonschema-check [--output jsonl] <schema.json> <data.json>\n",
            "arguments: {args:?}"
        );
    }
}

struct Fixture(PathBuf);

fn jsonl_fixture(name: &str, schema: &serde_json::Value, data: &serde_json::Value) -> Fixture {
    let fixture = Fixture(
        std::env::temp_dir().join(format!("jsonschema-check-{name}-{}", std::process::id())),
    );
    fs::create_dir(&fixture.0).unwrap();
    fs::write(fixture.0.join("schema.json"), schema.to_string()).unwrap();
    fs::write(fixture.0.join("data.json"), data.to_string()).unwrap();
    fixture
}

fn jsonl_command(fixture: &Fixture) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_jsonschema-check"));
    command
        .args(["--output", "jsonl"])
        .arg(fixture.0.join("schema.json"))
        .arg(fixture.0.join("data.json"));
    command
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

#[test]
fn help_and_version_in_file_positions_are_literal_paths() {
    let fixture = jsonl_fixture(
        "option-paths",
        &serde_json::json!(true),
        &serde_json::json!({}),
    );
    for name in ["--help", "--version"] {
        for role in ["schema", "data"] {
            for jsonl in [false, true] {
                let path = fixture.0.join(name);
                fs::write(&path, if role == "schema" { "true" } else { "{}" }).unwrap();
                let mut command = Command::new(env!("CARGO_BIN_EXE_jsonschema-check"));
                command.current_dir(&fixture.0);
                if jsonl {
                    command.args(["--output", "jsonl"]);
                }
                command.args(if role == "schema" {
                    [name, "data.json"]
                } else {
                    ["schema.json", name]
                });
                let output = command.output().unwrap();
                assert_eq!(output.status.code(), Some(0));
                assert_eq!(output.stdout, [] as [u8; 0]);
                assert_eq!(output.stderr, [] as [u8; 0]);

                fs::remove_file(&path).unwrap();
                let output = command.output().unwrap();
                assert_eq!(output.status.code(), Some(2));
                assert_eq!(output.stdout, [] as [u8; 0]);
                assert!(
                    String::from_utf8(output.stderr)
                        .unwrap()
                        .starts_with(&format!("Error: failed to read {role} {name:?}: "))
                );
            }
        }
    }

    fs::write(fixture.0.join("--help"), "true").unwrap();
    fs::write(fixture.0.join("--version"), "{}").unwrap();
    for jsonl in [false, true] {
        let mut command = Command::new(env!("CARGO_BIN_EXE_jsonschema-check"));
        command.current_dir(&fixture.0);
        if jsonl {
            command.args(["--output", "jsonl"]);
        }
        let output = command.args(["--help", "--version"]).output().unwrap();
        assert_eq!(output.status.code(), Some(0));
        assert_eq!(output.stdout, [] as [u8; 0]);
        assert_eq!(output.stderr, [] as [u8; 0]);
    }
}

#[test]
fn jsonl_reports_all_errors_with_three_string_attributes() {
    let fixture = jsonl_fixture(
        "jsonl-errors",
        &serde_json::json!({
            "type": "object",
            "properties": {"a/b~\"\n": {"type": "array", "items": {"type": "integer"}}},
            "required": ["missing"],
        }),
        &serde_json::json!({"a/b~\"\n": ["quote\"\n", false]}),
    );
    let output = jsonl_command(&fixture).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stderr, [] as [u8; 0]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.ends_with('\n'));
    assert!(stdout.contains("\\n"));
    assert!(stdout.contains("\\\""));
    let records: Vec<serde_json::Value> = stdout
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert_eq!(records.len(), 3);
    for record in &records {
        assert_eq!(record.as_object().unwrap().len(), 3);
        for key in ["instancePath", "schemaPath", "message"] {
            assert!(record[key].is_string());
        }
        assert_ne!(record["message"].as_str().unwrap(), "");
    }
    for (instance, schema) in [
        ("", "/required"),
        ("/a~1b~0\"\n/0", "/properties/a~1b~0\"\n/items/type"),
        ("/a~1b~0\"\n/1", "/properties/a~1b~0\"\n/items/type"),
    ] {
        assert!(records.iter().any(|record| {
            record["instancePath"] == instance && record["schemaPath"] == schema
        }));
    }
    let required = records
        .iter()
        .find(|record| record["instancePath"] == "")
        .unwrap();
    assert!(required["message"].as_str().unwrap().contains("missing"));
}

#[test]
fn jsonl_root_schema_error_uses_empty_pointers() {
    let fixture = jsonl_fixture(
        "jsonl-root",
        &serde_json::json!(false),
        &serde_json::json!({}),
    );
    let output = jsonl_command(&fixture).output().unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(output.stderr, [] as [u8; 0]);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert_eq!(stdout.lines().count(), 1);
    let record: serde_json::Value = serde_json::from_str(stdout.trim_end()).unwrap();
    assert_eq!(record["instancePath"], "");
    assert_eq!(record["schemaPath"], "");
}

#[test]
fn jsonl_success_is_silent() {
    let fixture = jsonl_fixture(
        "jsonl-valid",
        &serde_json::json!(true),
        &serde_json::json!({}),
    );
    let output = jsonl_command(&fixture).output().unwrap();
    assert_eq!(output.status.code(), Some(0));
    assert_eq!(output.stdout, [] as [u8; 0]);
    assert_eq!(output.stderr, [] as [u8; 0]);
}

#[test]
fn jsonl_input_errors_remain_text_on_stderr() {
    let fixture = jsonl_fixture(
        "jsonl-input",
        &serde_json::json!(true),
        &serde_json::json!({}),
    );
    let schema = fixture.0.join("schema.json");
    let data = fixture.0.join("data.json");
    for (schema_contents, data_contents, prefix) in [
        (None, Some("{}"), "Error: failed to read schema "),
        (Some("true"), None, "Error: failed to read data "),
        (Some("{"), Some("{}"), "Error: failed to parse schema "),
        (Some("true"), Some("{"), "Error: failed to parse data "),
        (Some(r#"{"type":"invalid"}"#), Some("{}"), "Error: "),
    ] {
        for (path, contents) in [(&schema, schema_contents), (&data, data_contents)] {
            if let Some(contents) = contents {
                fs::write(path, contents).unwrap();
            } else {
                fs::remove_file(path).unwrap();
            }
        }
        let output = jsonl_command(&fixture).output().unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(output.stdout, [] as [u8; 0]);
        assert!(
            String::from_utf8(output.stderr)
                .unwrap()
                .starts_with(prefix)
        );
    }
}

#[cfg(unix)]
#[test]
fn jsonl_output_failure_exits_with_two() {
    let fixture = jsonl_fixture(
        "jsonl-output",
        &serde_json::json!(false),
        &serde_json::json!({}),
    );
    let output = jsonl_command(&fixture)
        .stdout(closed_output())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .starts_with("Error: ")
    );
}

#[cfg(unix)]
fn closed_output() -> std::process::Stdio {
    let (reader, writer) = std::os::unix::net::UnixStream::pair().unwrap();
    let output = shutdown_output(writer);
    drop(reader);
    output
}

#[cfg(unix)]
fn shutdown_output(writer: std::os::unix::net::UnixStream) -> std::process::Stdio {
    // A concurrent spawn can inherit the peer before CLOEXEC is set on macOS.
    writer.shutdown(std::net::Shutdown::Write).unwrap();
    std::process::Stdio::from(std::os::fd::OwnedFd::from(writer))
}

#[cfg(unix)]
#[test]
fn output_failure_with_retained_peer_exits_with_two() {
    for option in ["--help", "--version"] {
        let (reader, writer) = std::os::unix::net::UnixStream::pair().unwrap();
        let retained_reader = reader.try_clone().unwrap();
        drop(reader);
        let output = Command::new(env!("CARGO_BIN_EXE_jsonschema-check"))
            .arg(option)
            .stdout(shutdown_output(writer))
            .output()
            .unwrap();
        drop(retained_reader);
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(output.stdout, [] as [u8; 0]);
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.starts_with("Error: "));
        assert!(!stderr.contains("panicked"));
    }
}

#[cfg(unix)]
#[test]
fn help_and_version_output_failures_exit_with_two() {
    for option in ["--help", "--version"] {
        let output = Command::new(env!("CARGO_BIN_EXE_jsonschema-check"))
            .arg(option)
            .stdout(closed_output())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.starts_with("Error: "));
        assert!(!stderr.contains("panicked"));
    }
}

#[cfg(unix)]
#[test]
fn text_output_failure_exits_with_two() {
    let fixture = jsonl_fixture(
        "text-output",
        &serde_json::json!(false),
        &serde_json::json!({}),
    );
    let output = Command::new(env!("CARGO_BIN_EXE_jsonschema-check"))
        .arg(fixture.0.join("schema.json"))
        .arg(fixture.0.join("data.json"))
        .stderr(closed_output())
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert_eq!(output.stdout, [] as [u8; 0]);
}

#[cfg(unix)]
#[test]
fn diagnostic_output_failure_exits_with_two() {
    for args in [vec![], vec!["--help"], vec!["--version"]] {
        let output = Command::new(env!("CARGO_BIN_EXE_jsonschema-check"))
            .args(args)
            .stdout(closed_output())
            .stderr(closed_output())
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
    }
}

#[cfg(unix)]
#[test]
fn non_utf8_arguments_report_normal_errors() {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    let fixture = jsonl_fixture(
        "non-utf8-errors",
        &serde_json::json!(true),
        &serde_json::json!({}),
    );
    let filename = OsStr::from_bytes(b"input-\xff.json");
    for role in ["schema", "data"] {
        for jsonl in [false, true] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_jsonschema-check"));
            command.current_dir(&fixture.0);
            if jsonl {
                command.args(["--output", "jsonl"]);
            }
            command.args(if role == "schema" {
                [filename, OsStr::new("data.json")]
            } else {
                [OsStr::new("schema.json"), filename]
            });
            let output = command.output().unwrap();
            assert_eq!(output.status.code(), Some(2));
            assert_eq!(output.stdout, [] as [u8; 0]);
            let stderr = String::from_utf8(output.stderr).unwrap();
            assert!(stderr.starts_with(&format!("Error: failed to read {role} {filename:?}: ")));
            assert!(!stderr.contains("panicked"));
        }
    }

    for args in [
        vec![filename],
        vec![OsStr::new("--output"), filename, filename, filename],
        vec![OsStr::from_bytes(b"--output=\xff"), filename],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_jsonschema-check"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(2));
        assert_eq!(output.stdout, [] as [u8; 0]);
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "Error: Usage: jsonschema-check [--output jsonl] <schema.json> <data.json>\n"
        );
    }
}

#[cfg(unix)]
#[cfg_attr(
    not(target_os = "linux"),
    ignore = "Requires a filesystem supporting non-UTF-8 file names"
)]
#[test]
fn non_utf8_paths_support_validation_and_parse_errors() {
    use std::{ffi::OsStr, os::unix::ffi::OsStrExt};

    let fixture = jsonl_fixture(
        "non-utf8-paths",
        &serde_json::json!(true),
        &serde_json::json!({}),
    );
    for role in ["schema", "data"] {
        let path = fixture.0.join(OsStr::from_bytes(b"input-\xff.json"));
        for jsonl in [false, true] {
            let mut command = Command::new(env!("CARGO_BIN_EXE_jsonschema-check"));
            if jsonl {
                command.args(["--output", "jsonl"]);
            }
            command.arg(if role == "schema" {
                path.clone()
            } else {
                fixture.0.join("schema.json")
            });
            command.arg(if role == "data" {
                path.clone()
            } else {
                fixture.0.join("data.json")
            });
            for (contents, code) in [("true", 0), ("false", 1), ("{", 2)] {
                if role == "data" {
                    fs::write(fixture.0.join("schema.json"), "{\"const\":true}").unwrap();
                }
                fs::write(&path, contents).unwrap();
                let output = command.output().unwrap();
                assert_eq!(output.status.code(), Some(code));
                match code {
                    0 => {
                        assert_eq!(output.stdout, [] as [u8; 0]);
                        assert_eq!(output.stderr, [] as [u8; 0]);
                    }
                    1 if jsonl => {
                        assert_eq!(output.stderr, [] as [u8; 0]);
                        serde_json::from_slice::<serde_json::Value>(&output.stdout).unwrap();
                    }
                    1 => {
                        assert_eq!(output.stdout, [] as [u8; 0]);
                        assert_ne!(output.stderr, [] as [u8; 0]);
                    }
                    _ => {
                        assert_eq!(output.stdout, [] as [u8; 0]);
                        let stderr = String::from_utf8(output.stderr).unwrap();
                        let native_path = path.as_os_str();
                        assert!(stderr.starts_with(&format!(
                            "Error: failed to parse {role} {native_path:?}: "
                        )));
                    }
                }
            }
        }
    }
}

#[test]
fn file_errors_identify_operation_role_and_escaped_path() {
    let fixture = Fixture(std::env::temp_dir().join(format!(
        "jsonschema-check-file-errors-{}",
        std::process::id()
    )));
    fs::create_dir(&fixture.0).unwrap();
    let valid_schema = fixture.0.join("valid-schema.json");
    let valid_data = fixture.0.join("valid-data.json");
    fs::write(&valid_schema, "true").unwrap();
    fs::write(&valid_data, "{}").unwrap();

    for role in ["schema", "data"] {
        for filename in [
            "input.json",
            "input with spaces.json",
            "input\"quoted\n.json",
        ] {
            let path = fixture.0.join(filename);
            let args = if role == "schema" {
                [&path, &valid_data]
            } else {
                [&valid_schema, &path]
            };

            for operation in ["read", "parse"] {
                let details = if operation == "read" {
                    fs::read(&path).unwrap_err().to_string()
                } else {
                    fs::write(&path, "{\n").unwrap();
                    serde_json::from_slice::<serde_json::Value>(b"{\n")
                        .unwrap_err()
                        .to_string()
                };
                let output = Command::new(env!("CARGO_BIN_EXE_jsonschema-check"))
                    .args(args)
                    .output()
                    .unwrap();
                assert_eq!(output.status.code(), Some(2));
                assert_eq!(output.stdout, [] as [u8; 0]);
                assert_eq!(
                    String::from_utf8(output.stderr).unwrap(),
                    format!(
                        "Error: failed to {operation} {role} {:?}: {details}\n",
                        path.to_str().unwrap()
                    )
                );
            }
            fs::remove_file(&path).unwrap();
        }
    }
}

#[test]
fn validation_and_input_errors_keep_their_exit_codes() {
    let fixture =
        Fixture(std::env::temp_dir().join(format!("jsonschema-check-cli-{}", std::process::id())));
    fs::create_dir(&fixture.0).unwrap();
    let schema = fixture.0.join("schema.json");
    let data = fixture.0.join("data.json");
    fs::write(&schema, r#"{"type":"integer","minimum":0}"#).unwrap();

    for (contents, code) in [("30", 0), ("-1", 1), ("{", 2)] {
        fs::write(&data, contents).unwrap();
        let output = Command::new(env!("CARGO_BIN_EXE_jsonschema-check"))
            .arg(&schema)
            .arg(&data)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(code));
        assert_eq!(output.stdout, [] as [u8; 0]);
        match code {
            0 => assert_eq!(output.stderr, [] as [u8; 0]),
            1 => assert!(
                String::from_utf8(output.stderr)
                    .unwrap()
                    .contains("minimum")
            ),
            2 => assert!(
                String::from_utf8(output.stderr)
                    .unwrap()
                    .starts_with("Error: ")
            ),
            _ => unreachable!(),
        }
    }
}

const DRAFT_URIS: [&str; 5] = [
    "http://json-schema.org/draft-04/schema#",
    "http://json-schema.org/draft-06/schema#",
    "http://json-schema.org/draft-07/schema#",
    "https://json-schema.org/draft/2019-09/schema",
    "https://json-schema.org/draft/2020-12/schema",
];

#[test]
fn readme_examples_match_cli_output() {
    fn code_block<'a>(text: &'a str, language: &str) -> &'a str {
        text.split_once(&format!("```{language}\n"))
            .expect("README example code block is missing")
            .1
            .split_once("\n```")
            .expect("README example code block is not closed")
            .0
    }

    let readme = include_str!("../README.md");
    let usage = readme
        .split_once("## Usage\n")
        .expect("README Usage section is missing")
        .1;
    let schema: serde_json::Value = serde_json::from_str(code_block(usage, "json")).unwrap();
    let data_section = usage
        .split_once("Save the following data as `data.json`:")
        .expect("README data example is missing")
        .1;
    let mut data: serde_json::Value =
        serde_json::from_str(code_block(data_section, "json")).unwrap();
    let expected_text = format!("{}\n", code_block(usage, "text"));
    let expected_jsonl: serde_json::Value =
        serde_json::from_str(code_block(usage, "jsonl")).unwrap();
    let fixture = jsonl_fixture("readme-examples", &schema, &data);

    for jsonl in [false, true] {
        assert_validation_output(&fixture, jsonl, 0);
    }

    data["age"] = serde_json::json!(-1);
    fs::write(fixture.0.join("data.json"), data.to_string()).unwrap();
    for jsonl in [false, true] {
        let output = assert_validation_output(&fixture, jsonl, 1);
        if jsonl {
            let record: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
            assert_eq!(record, expected_jsonl);
        } else {
            assert_eq!(output.stderr, expected_text.as_bytes());
        }
    }
}

fn assert_validation_output(fixture: &Fixture, jsonl: bool, expected_code: i32) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_jsonschema-check"));
    if jsonl {
        command.args(["--output", "jsonl"]);
    }
    let output = command
        .arg(fixture.0.join("schema.json"))
        .arg(fixture.0.join("data.json"))
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(expected_code),
        "jsonl={jsonl}, stdout={:?}, stderr={:?}",
        output.stdout,
        output.stderr
    );
    match expected_code {
        0 => {
            assert_eq!(output.stdout, [] as [u8; 0]);
            assert_eq!(output.stderr, [] as [u8; 0]);
        }
        1 if jsonl => {
            assert_eq!(output.stderr, [] as [u8; 0]);
            let stdout = std::str::from_utf8(&output.stdout).unwrap();
            assert!(stdout.ends_with('\n'));
            for line in stdout.lines() {
                let record: serde_json::Value = serde_json::from_str(line).unwrap();
                assert_eq!(record.as_object().unwrap().len(), 3);
                for key in ["instancePath", "schemaPath", "message"] {
                    assert!(record[key].is_string());
                }
            }
        }
        1 => {
            assert_eq!(output.stdout, [] as [u8; 0]);
            assert_ne!(output.stderr, [] as [u8; 0]);
            assert!(
                !std::str::from_utf8(&output.stderr)
                    .unwrap()
                    .starts_with("Error: ")
            );
        }
        2 => {
            assert_eq!(output.stdout, [] as [u8; 0]);
            assert!(
                std::str::from_utf8(&output.stderr)
                    .unwrap()
                    .starts_with("Error: ")
            );
        }
        _ => panic!("unexpected exit code in test case: {expected_code}"),
    }
    output
}

#[test]
fn schema_property_selects_each_supported_draft() {
    let fixture = jsonl_fixture(
        "draft-selection",
        &serde_json::json!({}),
        &serde_json::json!(0),
    );
    for (uri, mut schema, valid, invalid) in [
        (
            DRAFT_URIS[0],
            serde_json::json!({"minimum": 5, "exclusiveMinimum": true}),
            serde_json::json!(6),
            serde_json::json!(5),
        ),
        (
            DRAFT_URIS[1],
            serde_json::json!({"const": 5}),
            serde_json::json!(5),
            serde_json::json!(6),
        ),
        (
            DRAFT_URIS[2],
            serde_json::json!({"if": {"type": "integer"}, "then": {"minimum": 5}}),
            serde_json::json!(5),
            serde_json::json!(4),
        ),
        (
            DRAFT_URIS[3],
            serde_json::json!({"items": [{"type": "integer"}], "additionalItems": false}),
            serde_json::json!([5]),
            serde_json::json!([5, 6]),
        ),
        (
            DRAFT_URIS[4],
            serde_json::json!({"prefixItems": [{"type": "integer"}], "items": false}),
            serde_json::json!([5]),
            serde_json::json!([5, 6]),
        ),
    ] {
        schema["$schema"] = serde_json::json!(uri);
        fs::write(fixture.0.join("schema.json"), schema.to_string()).unwrap();
        for (data, code) in [(valid, 0), (invalid, 1)] {
            fs::write(fixture.0.join("data.json"), data.to_string()).unwrap();
            for jsonl in [false, true] {
                assert_validation_output(&fixture, jsonl, code);
            }
        }
    }
}

#[test]
fn omitted_schema_property_uses_draft_2020_12() {
    let fixture = jsonl_fixture(
        "default-draft",
        &serde_json::json!({"prefixItems": [{"type": "integer"}], "items": false}),
        &serde_json::json!([5]),
    );
    for (data, code) in [(serde_json::json!([5]), 0), (serde_json::json!([5, 6]), 1)] {
        fs::write(fixture.0.join("data.json"), data.to_string()).unwrap();
        for jsonl in [false, true] {
            assert_validation_output(&fixture, jsonl, code);
        }
    }
}

#[test]
fn tuple_schema_syntax_depends_on_the_selected_draft() {
    let fixture = jsonl_fixture(
        "draft-syntax",
        &serde_json::json!({}),
        &serde_json::json!([]),
    );
    for uri in DRAFT_URIS.into_iter().map(Some).chain([None]) {
        let mut schema = serde_json::json!({"items": [{"type": "integer"}]});
        if let Some(uri) = uri {
            schema["$schema"] = serde_json::json!(uri);
        }
        fs::write(fixture.0.join("schema.json"), schema.to_string()).unwrap();
        let code = if uri.is_none() || uri == Some(DRAFT_URIS[4]) {
            2
        } else {
            0
        };
        for jsonl in [false, true] {
            assert_validation_output(&fixture, jsonl, code);
        }
    }
}

#[test]
fn standard_drafts_control_format_validation() {
    let fixture = jsonl_fixture("formats", &serde_json::json!({}), &serde_json::json!(""));
    for uri in DRAFT_URIS.into_iter().map(Some).chain([None]) {
        let mut schema = serde_json::json!({"type": "string", "format": "email"});
        if let Some(uri) = uri {
            schema["$schema"] = serde_json::json!(uri);
        }
        let invalid_format_code = i32::from(uri.is_some_and(|uri| DRAFT_URIS[..3].contains(&uri)));
        for (data, code) in [
            (serde_json::json!("test@example.com"), 0),
            (serde_json::json!("not-an-email"), invalid_format_code),
            (serde_json::json!(42), 1),
        ] {
            fs::write(fixture.0.join("schema.json"), schema.to_string()).unwrap();
            fs::write(fixture.0.join("data.json"), data.to_string()).unwrap();
            for jsonl in [false, true] {
                assert_validation_output(&fixture, jsonl, code);
            }
        }
        schema["format"] = serde_json::json!("unknown-format");
        fs::write(fixture.0.join("schema.json"), schema.to_string()).unwrap();
        for (data, code) in [
            (serde_json::json!("anything"), 0),
            (serde_json::json!(42), 1),
        ] {
            fs::write(fixture.0.join("data.json"), data.to_string()).unwrap();
            for jsonl in [false, true] {
                assert_validation_output(&fixture, jsonl, code);
            }
        }
    }
}

#[test]
fn internal_references_validate_and_missing_targets_report_errors() {
    let fixture = jsonl_fixture(
        "internal-refs",
        &serde_json::json!({}),
        &serde_json::json!(0),
    );
    for uri in DRAFT_URIS {
        let mut schema = serde_json::json!({
            "$schema": uri,
            "definitions": {"item": {"type": "integer"}},
            "$ref": "#/definitions/item",
        });
        fs::write(fixture.0.join("schema.json"), schema.to_string()).unwrap();
        for (data, code) in [
            (serde_json::json!(42), 0),
            (serde_json::json!("wrong type"), 1),
        ] {
            fs::write(fixture.0.join("data.json"), data.to_string()).unwrap();
            for jsonl in [false, true] {
                assert_validation_output(&fixture, jsonl, code);
            }
        }
        schema["$ref"] = serde_json::json!("#/definitions/missing");
        fs::write(fixture.0.join("schema.json"), schema.to_string()).unwrap();
        for jsonl in [false, true] {
            assert_validation_output(&fixture, jsonl, 2);
        }
    }
}

#[test]
fn external_references_are_not_retrieved() {
    let fixture = jsonl_fixture(
        "external-refs",
        &serde_json::json!({}),
        &serde_json::json!(0),
    );
    let external = fixture.0.join("external.json");
    fs::write(&external, "true").unwrap();
    for (reference, diagnostic) in [
        (
            "http://example.invalid/external.json".to_owned(),
            "`resolve-http` feature",
        ),
        (
            "https://example.invalid/external.json".to_owned(),
            "`resolve-http` feature",
        ),
        (
            format!("file://{}", external.display()),
            "`resolve-file` feature",
        ),
        ("external.json".to_owned(), "No base URI is available"),
    ] {
        let schema = serde_json::json!({"$ref": reference});
        fs::write(fixture.0.join("schema.json"), schema.to_string()).unwrap();
        for jsonl in [false, true] {
            let output = assert_validation_output(&fixture, jsonl, 2);
            assert!(
                String::from_utf8(output.stderr)
                    .unwrap()
                    .contains(diagnostic)
            );
        }
    }
}

fn assert_numeric_cases(name: &str, cases: &[(&str, &str, i32)]) {
    let fixture = jsonl_fixture(name, &serde_json::json!({}), &serde_json::json!(0));
    for &(schema, data, code) in cases {
        // Write the original JSON text so Rust literals cannot round the numbers.
        fs::write(fixture.0.join("schema.json"), schema).unwrap();
        fs::write(fixture.0.join("data.json"), data).unwrap();
        for jsonl in [false, true] {
            assert_validation_output(&fixture, jsonl, code);
        }
    }
}

#[test]
fn large_integers_preserve_equality_and_integer_type() {
    assert_numeric_cases(
        "large-integers",
        &[
            (
                r#"{"const":18446744073709551616}"#,
                "18446744073709551616",
                0,
            ),
            (
                r#"{"const":18446744073709551616}"#,
                "18446744073709551617",
                1,
            ),
            (
                r#"{"const":-9223372036854775809}"#,
                "-9223372036854775809",
                0,
            ),
            (
                r#"{"const":-9223372036854775809}"#,
                "-9223372036854775810",
                1,
            ),
            (
                r#"{"enum":[18446744073709551616]}"#,
                "18446744073709551616",
                0,
            ),
            (
                r#"{"enum":[18446744073709551616]}"#,
                "18446744073709551617",
                1,
            ),
            (
                r#"{"uniqueItems":true}"#,
                "[18446744073709551616,18446744073709551617]",
                0,
            ),
            (
                r#"{"uniqueItems":true}"#,
                "[18446744073709551616,18446744073709551616]",
                1,
            ),
            (r#"{"type":"integer"}"#, "18446744073709551616", 0),
            (r#"{"type":"integer"}"#, "18446744073709551616.1", 1),
        ],
    );
}

#[test]
fn high_precision_decimals_preserve_equality() {
    assert_numeric_cases(
        "precise-decimals",
        &[
            (
                r#"{"const":0.123456789012345678901}"#,
                "0.123456789012345678901",
                0,
            ),
            (
                r#"{"const":0.123456789012345678901}"#,
                "0.123456789012345678902",
                1,
            ),
            (
                r#"{"enum":[0.123456789012345678901]}"#,
                "0.123456789012345678901",
                0,
            ),
            (
                r#"{"enum":[0.123456789012345678901]}"#,
                "0.123456789012345678902",
                1,
            ),
            (
                r#"{"uniqueItems":true}"#,
                "[0.123456789012345678901,0.123456789012345678902]",
                0,
            ),
            (
                r#"{"uniqueItems":true}"#,
                "[0.123456789012345678901,0.123456789012345678901]",
                1,
            ),
            (r#"{"const":0.1}"#, "0.100000000000000000000", 0),
        ],
    );
}

#[test]
fn numeric_bounds_preserve_large_and_decimal_boundaries() {
    assert_numeric_cases(
        "precise-bounds",
        &[
            (
                r#"{"minimum":18446744073709551617}"#,
                "18446744073709551617",
                0,
            ),
            (
                r#"{"minimum":18446744073709551617}"#,
                "18446744073709551616",
                1,
            ),
            (
                r#"{"maximum":18446744073709551616}"#,
                "18446744073709551616",
                0,
            ),
            (
                r#"{"maximum":18446744073709551616}"#,
                "18446744073709551617",
                1,
            ),
            (
                r#"{"exclusiveMinimum":18446744073709551616}"#,
                "18446744073709551617",
                0,
            ),
            (
                r#"{"exclusiveMinimum":18446744073709551616}"#,
                "18446744073709551616",
                1,
            ),
            (
                r#"{"exclusiveMaximum":18446744073709551617}"#,
                "18446744073709551616",
                0,
            ),
            (
                r#"{"exclusiveMaximum":18446744073709551617}"#,
                "18446744073709551617",
                1,
            ),
            (
                r#"{"minimum":0.123456789012345678902}"#,
                "0.123456789012345678902",
                0,
            ),
            (
                r#"{"minimum":0.123456789012345678902}"#,
                "0.123456789012345678901",
                1,
            ),
            (
                r#"{"maximum":0.123456789012345678901}"#,
                "0.123456789012345678901",
                0,
            ),
            (
                r#"{"maximum":0.123456789012345678901}"#,
                "0.123456789012345678902",
                1,
            ),
        ],
    );
}

#[test]
fn multiple_of_uses_exact_decimal_and_integer_arithmetic() {
    assert_numeric_cases(
        "precise-multiples",
        &[
            (r#"{"multipleOf":0.1}"#, "0.3", 0),
            (
                r#"{"multipleOf":0.1}"#,
                "0.3000000000000000000000000000001",
                1,
            ),
            (r#"{"multipleOf":0.1}"#, "-0.3", 0),
            (r#"{"multipleOf":3}"#, "18446744073709551618", 0),
            (r#"{"multipleOf":3}"#, "18446744073709551619", 1),
            (r#"{"multipleOf":1e-400}"#, "3e-400", 0),
            (r#"{"multipleOf":1e-400}"#, "3.1e-400", 1),
        ],
    );
}

#[test]
fn numbers_outside_f64_range_are_validated_without_overflow_or_underflow() {
    assert_numeric_cases(
        "large-exponents",
        &[
            (r#"{"type":"number"}"#, "1e400", 0),
            (r#"{"type":"number"}"#, "1e-400", 0),
            (r#"{"type":"integer"}"#, "1e400", 0),
            (r#"{"type":"integer"}"#, "1e-400", 1),
            (r#"{"minimum":1e400}"#, "1e400", 0),
            (r#"{"minimum":1e400}"#, "9e399", 1),
            (r#"{"const":1e400}"#, "10e399", 0),
            (r#"{"const":1e400}"#, "1e401", 1),
            (r#"{"const":0}"#, "1e-400", 1),
            (r#"{"maximum":-1e400}"#, "-2e400", 0),
            (r#"{"maximum":-1e400}"#, "-9e399", 1),
        ],
    );
}

#[test]
fn arbitrary_precision_still_rejects_invalid_json_numbers_and_schemas() {
    assert_numeric_cases(
        "invalid-numbers",
        &[
            (r#"{"type":"number"}"#, "01", 2),
            (r#"{"type":"number"}"#, "1e", 2),
            (r#"{"type":"number"}"#, "NaN", 2),
            (r#"{"type":"number"}"#, "Infinity", 2),
            (r#"{"const":01}"#, "0", 2),
            (r#"{"multipleOf":0}"#, "0", 2),
        ],
    );
}
