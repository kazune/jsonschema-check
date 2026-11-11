use serde_json::Value;
use std::{
    env, fs,
    io::{self, Write},
    process,
};

const USAGE: &str = "Usage: jsonschema-check [--output jsonl] <schema.json> <data.json>";

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
                 Options:\n  --help         Print help\n  --version      Print version\n  --output jsonl Write validation errors as JSON Lines to stdout\n\n\
                 Exit codes:\n  0  Validation succeeded, or help/version was printed\n  1  Validation failed\n  2  Argument, file reading, JSON parsing, schema compilation, or output error"
            );
            return Ok(true);
        }

        if args[1] == "--version" {
            println!("jsonschema-check {}", env!("CARGO_PKG_VERSION"));
            return Ok(true);
        }
    }

    let jsonl = args.get(1).is_some_and(|arg| arg == "--output");
    let files = if jsonl {
        if args.len() != 5 || args[2] != "jsonl" {
            return Err(USAGE.into());
        }
        &args[3..]
    } else {
        &args[1..]
    };

    if files.len() != 2
        || files
            .iter()
            .any(|arg| arg == "--output" || arg.starts_with("--output="))
    {
        return Err(USAGE.into());
    }

    let schema_path = &files[0];
    let data_path = &files[1];

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
        if jsonl {
            let record = serde_json::json!({
                "instancePath": error.instance_path().to_string(),
                "schemaPath": error.schema_path().to_string(),
                "message": error.to_string(),
            });
            writeln!(stdout, "{record}")?;
        } else {
            eprintln!("{}: {}", error.instance_path(), error);
        }
        valid = false;
    }

    if jsonl {
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
