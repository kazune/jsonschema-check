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
        "Usage: jsonschema-check <schema.json> <data.json>",
        "Arguments:",
        "Options:",
        "--help",
        "--version",
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
        vec!["--help", "data.json"],
        vec!["schema.json", "--help"],
        vec!["--version", "data.json"],
        vec!["schema.json", "--version"],
        vec!["--help", "--version"],
        vec!["--help", "schema.json", "data.json"],
        vec!["--version", "schema.json", "data.json"],
        vec!["-h"],
        vec!["-V"],
    ] {
        let output = run(&args);
        assert_eq!(output.status.code(), Some(2), "arguments: {args:?}");
        assert!(output.stdout.is_empty(), "arguments: {args:?}");
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "Error: Usage: jsonschema-check <schema.json> <data.json>\n",
            "arguments: {args:?}"
        );
    }
}

struct Fixture(PathBuf);

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
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
