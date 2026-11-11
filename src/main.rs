use std::{env, fs, process};
use serde_json::Value;

fn run() -> Result<bool, Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 3 {
        return Err(
            "Usage: jsonschema-check <schema.json> <data.json>".into()
        );
    }

    // ファイルの読み込みとJSONパース
    let schema: Value =
        serde_json::from_slice(&fs::read(&args[1])?)?;

    let data: Value =
        serde_json::from_slice(&fs::read(&args[2])?)?;

    // スキーマをコンパイル
    let validator = jsonschema::validator_for(&schema)?;

    // 検証してエラーを表示
    let mut valid = true;

    for error in validator.iter_errors(&data) {
        eprintln!(
            "{}: {}",
            error.instance_path(),
            error
        );
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
