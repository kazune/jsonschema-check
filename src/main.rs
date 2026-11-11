use serde_json::Value;
use std::{env, fs, process};

const USAGE: &str = "Usage: jsonschema-check <schema.json> <data.json>";

fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() == 2 {
        if args[1] == "--help" {
            println!(
                "A minimal JSON Schema validation CLI.\n\n\
                 {USAGE}\n\
                 Usage: jsonschema-check --help\n\
                 Usage: jsonschema-check --version\n\n\
                 Arguments:\n  <schema.json>  Path to the JSON Schema file\n  <data.json>    Path to the JSON data file\n\n\
                 Options:\n  --help     Print help\n  --version  Print version\n\n\
                 Exit codes:\n  0  Validation succeeded, or help/version was printed\n  1  Validation failed\n  2  Argument, file reading, JSON parsing, or schema compilation error"
            );
            return Ok(true);
        }

        if args[1] == "--version" {
            println!("jsonschema-check {}", env!("CARGO_PKG_VERSION"));
            return Ok(true);
        }
    }

    if args.len() != 3
        || args[1..]
            .iter()
            .any(|arg| arg == "--help" || arg == "--version")
    {
        return Err(USAGE.into());
    }

    // ファイルの読み込みとJSONパース
    let schema_bytes =
        fs::read(&args[1]).map_err(|err| format!("failed to read schema {:?}: {err}", args[1]))?;
    let schema: Value = serde_json::from_slice(&schema_bytes)
        .map_err(|err| format!("failed to parse schema {:?}: {err}", args[1]))?;

    let data_bytes =
        fs::read(&args[2]).map_err(|err| format!("failed to read data {:?}: {err}", args[2]))?;
    let data: Value = serde_json::from_slice(&data_bytes)
        .map_err(|err| format!("failed to parse data {:?}: {err}", args[2]))?;

    // スキーマをコンパイル
    let validator = jsonschema::validator_for(&schema)?;

    // 検証してエラーを表示
    let mut valid = true;

    for error in validator.iter_errors(&data) {
        eprintln!("{}: {}", error.instance_path(), error);
        valid = false;
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
