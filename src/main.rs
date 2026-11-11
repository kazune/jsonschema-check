use serde_json::Value;
use std::{
    env, fs,
    io::{self, Write},
    process,
};

const USAGE: &str = "Usage: jsonschema-check [--output jsonl] <schema.json> <data.json>";

#[derive(Clone, Copy)]
enum OutputFormat {
    Text,
    Jsonl,
}

fn is_file_arg(arg: &str) -> bool {
    arg != "--output" && !arg.starts_with("--output=")
}

fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().skip(1).collect();

    match args.as_slice() {
        [arg] if arg == "--help" => {
            print_help();
            Ok(true)
        }
        [arg] if arg == "--version" => {
            print_version();
            Ok(true)
        }
        [option, format, schema, data]
            if option == "--output"
                && format == "jsonl"
                && is_file_arg(schema)
                && is_file_arg(data) =>
        {
            validate(schema, data, OutputFormat::Jsonl)
        }
        [schema, data] if is_file_arg(schema) && is_file_arg(data) => {
            validate(schema, data, OutputFormat::Text)
        }
        _ => Err(USAGE.into()),
    }
}

fn print_help() {
    println!(
        "A minimal JSON Schema validation CLI.\n\n\
         {USAGE}\n\
         Usage: jsonschema-check --help\n\
         Usage: jsonschema-check --version\n\n\
         Arguments:\n  <schema.json>  Path to the JSON Schema file\n  <data.json>    Path to the JSON data file\n\n\
         Options:\n  --help         Print help\n  --version      Print version\n  --output jsonl Write validation errors as JSON Lines to stdout\n\n\
         Exit codes:\n  0  Validation succeeded, or help/version was printed\n  1  Validation failed\n  2  Argument, file reading, JSON parsing, schema compilation, or output error"
    );
}

fn print_version() {
    println!("jsonschema-check {}", env!("CARGO_PKG_VERSION"));
}

fn validate(
    schema_path: &str,
    data_path: &str,
    output: OutputFormat,
) -> Result<bool, Box<dyn std::error::Error>> {
    // ファイルの読み込みとJSONパース
    let schema_bytes = fs::read(schema_path)
        .map_err(|err| format!("failed to read schema {schema_path:?}: {err}"))?;
    let schema: Value = serde_json::from_slice(&schema_bytes)
        .map_err(|err| format!("failed to parse schema {schema_path:?}: {err}"))?;

    let data_bytes =
        fs::read(data_path).map_err(|err| format!("failed to read data {data_path:?}: {err}"))?;
    let data: Value = serde_json::from_slice(&data_bytes)
        .map_err(|err| format!("failed to parse data {data_path:?}: {err}"))?;

    // スキーマをコンパイル
    let validator = jsonschema::validator_for(&schema)?;

    // 検証してエラーを表示
    let mut valid = true;
    let mut stdout = io::stdout().lock();

    for error in validator.iter_errors(&data) {
        match output {
            OutputFormat::Jsonl => {
                let record = serde_json::json!({
                    "instancePath": error.instance_path().to_string(),
                    "schemaPath": error.schema_path().to_string(),
                    "message": error.to_string(),
                });
                writeln!(stdout, "{record}")?;
            }
            OutputFormat::Text => eprintln!("{}: {}", error.instance_path(), error),
        }
        valid = false;
    }

    if matches!(output, OutputFormat::Jsonl) {
        stdout.flush()?;
    }

    Ok(valid)
}

fn main() {
    match run() {
        Ok(true) => process::exit(0),
        Ok(false) => process::exit(1),
        Err(err) => {
            eprintln!("Error: {err}");
            process::exit(2);
        }
    }
}
