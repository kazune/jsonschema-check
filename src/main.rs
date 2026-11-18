use serde_json::Value;
use std::{
    env,
    ffi::{OsStr, OsString},
    fs,
    io::{self, Write},
    path::Path,
    process,
};

const USAGE: &str = "Usage: jsonschema-check [--output jsonl] <schema.json> <data.json>";

#[derive(Clone, Copy)]
enum OutputFormat {
    Text,
    Jsonl,
}

enum ExitCode {
    Ok = 0,
    ValidationFailed = 1,
    InvalidDataJson = 2,
    InvalidSchema = 3,
    Other = 4,
}

struct RunError {
    code: ExitCode,
    message: String,
}

impl RunError {
    fn new(code: ExitCode, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl From<io::Error> for RunError {
    fn from(error: io::Error) -> Self {
        Self::new(ExitCode::Other, error.to_string())
    }
}

fn is_file_arg(arg: &OsStr) -> bool {
    arg != "--output" && !arg.as_encoded_bytes().starts_with(b"--output=")
}

fn run() -> Result<ExitCode, RunError> {
    let args: Vec<OsString> = env::args_os().skip(1).collect();

    match args.as_slice() {
        [arg] if arg == "--help" => {
            print_help()?;
            Ok(ExitCode::Ok)
        }
        [arg] if arg == "--version" => {
            print_version()?;
            Ok(ExitCode::Ok)
        }
        [option, format, schema, data]
            if option == "--output"
                && format == "jsonl"
                && is_file_arg(schema)
                && is_file_arg(data) =>
        {
            validate(Path::new(schema), Path::new(data), OutputFormat::Jsonl)
        }
        [schema, data] if is_file_arg(schema) && is_file_arg(data) => {
            validate(Path::new(schema), Path::new(data), OutputFormat::Text)
        }
        _ => Err(RunError::new(ExitCode::Other, USAGE)),
    }
}

fn print_help() -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    writeln!(
        stdout,
        "A minimal JSON Schema validation CLI.\n\n\
         {USAGE}\n\
         Usage: jsonschema-check --help\n\
         Usage: jsonschema-check --version\n\n\
         Arguments:\n  <schema.json>  Path to the JSON Schema file\n  <data.json>    Path to the JSON data file\n\n\
         Options:\n  --help         Print help\n  --version      Print version\n  --output jsonl Write validation errors as JSON Lines to stdout\n\n\
         Exit codes:\n  0  Validation succeeded, or help/version was printed\n  1  Validation failed\n  2  Invalid data JSON\n  3  Invalid schema JSON or schema compilation error\n  4  Argument, file reading, or output error"
    )?;
    stdout.flush()
}

fn print_version() -> io::Result<()> {
    let mut stdout = io::stdout().lock();
    writeln!(stdout, "jsonschema-check {}", env!("CARGO_PKG_VERSION"))?;
    stdout.flush()
}

#[allow(
    clippy::unnecessary_debug_formatting,
    reason = "Diagnostics require quoted paths with special characters and non-UTF-8 bytes escaped."
)]
fn validate(
    schema_path: &Path,
    data_path: &Path,
    output: OutputFormat,
) -> Result<ExitCode, RunError> {
    // ファイルの読み込みとJSONパース
    let schema_bytes = fs::read(schema_path).map_err(|err| {
        RunError::new(
            ExitCode::Other,
            format!("failed to read schema {schema_path:?}: {err}"),
        )
    })?;
    let schema: Value = serde_json::from_slice(&schema_bytes).map_err(|err| {
        RunError::new(
            ExitCode::InvalidSchema,
            format!("failed to parse schema {schema_path:?}: {err}"),
        )
    })?;

    let data_bytes = fs::read(data_path).map_err(|err| {
        RunError::new(
            ExitCode::Other,
            format!("failed to read data {data_path:?}: {err}"),
        )
    })?;
    let data: Value = serde_json::from_slice(&data_bytes).map_err(|err| {
        RunError::new(
            ExitCode::InvalidDataJson,
            format!("failed to parse data {data_path:?}: {err}"),
        )
    })?;

    // スキーマをコンパイル
    let validator = jsonschema::validator_for(&schema)
        .map_err(|err| RunError::new(ExitCode::InvalidSchema, err.to_string()))?;

    // 検証してエラーを表示
    let mut code = ExitCode::Ok;
    let mut stdout = io::stdout().lock();
    let mut stderr = io::stderr().lock();

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
            OutputFormat::Text => writeln!(stderr, "{}: {}", error.instance_path(), error)?,
        }
        code = ExitCode::ValidationFailed;
    }

    match output {
        OutputFormat::Jsonl => stdout.flush()?,
        OutputFormat::Text => stderr.flush()?,
    }

    Ok(code)
}

fn main() {
    let code = match run() {
        Ok(code) => code,
        Err(err) => {
            let mut stderr = io::stderr().lock();
            match writeln!(stderr, "Error: {}", err.message).and_then(|()| stderr.flush()) {
                Ok(()) => err.code,
                Err(_) => ExitCode::Other,
            }
        }
    };
    process::exit(code as i32);
}
