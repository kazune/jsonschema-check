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
    assert!(output.stderr.is_empty());
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
    assert!(output.stderr.is_empty());
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
                assert!(output.stdout.is_empty());
                assert!(output.stderr.is_empty());

                fs::remove_file(&path).unwrap();
                let output = command.output().unwrap();
                assert_eq!(output.status.code(), Some(2));
                assert!(output.stdout.is_empty());
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
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
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
    assert!(output.stderr.is_empty());
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
        assert!(!record["message"].as_str().unwrap().is_empty());
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
    assert!(output.stderr.is_empty());
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
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
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
        assert!(output.stdout.is_empty());
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
    let (reader, writer) = std::os::unix::net::UnixStream::pair().unwrap();
    drop(reader);
    let output = jsonl_command(&fixture)
        .stdout(std::process::Stdio::from(std::os::fd::OwnedFd::from(
            writer,
        )))
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(
        String::from_utf8(output.stderr)
            .unwrap()
            .starts_with("Error: ")
    );
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
                assert!(output.stdout.is_empty());
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
        assert!(output.stdout.is_empty());
        match code {
            0 => assert!(output.stderr.is_empty()),
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
