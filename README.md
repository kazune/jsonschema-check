# jsonschema-check

A minimal JSON Schema validation CLI with a small dependency footprint.

`jsonschema-check` validates a JSON file against a JSON Schema and reports validation errors to standard error.
It uses the `jsonschema` library with default features disabled to keep dependencies small.

## Build

A Rust toolchain with support for edition 2024 is required.

```sh
cargo build --release --locked
```

The executable is available at `target/release/jsonschema-check`.

## Usage

To validate data, pass exactly two arguments: the schema file path followed by the data file path.
Both files must contain valid JSON.

```sh
jsonschema-check <schema.json> <data.json>
```

Use `--help` to display usage information or `--version` to display the program version.
Each option must be used alone and writes to standard output with exit code `0` without reading any files.

```sh
jsonschema-check --help
jsonschema-check --version
```

Combining either option with other arguments produces the usage message on standard error with exit code `2`.

For example, save the following schema as `schema.json`:

```json
{
  "type": "object",
  "properties": {
    "name": { "type": "string" },
    "age": { "type": "integer", "minimum": 0 }
  },
  "required": ["name", "age"],
  "additionalProperties": false
}
```

Save the following data as `data.json`:

```json
{
  "name": "Alice",
  "age": 30
}
```

Run the executable:

```sh
./target/release/jsonschema-check schema.json data.json
```

Successful validation produces no output and exits with code `0`.
Changing `age` to `-1` produces a validation error on standard error and exits with code `1`:

```text
/age: -1 is less than the minimum of 0
```

Each reported validation error appears on its own line as `<instance path>: <message>`.
The instance path is a JSON Pointer into the data and is empty for errors at the document root.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | The data satisfies the schema, or help or version information was displayed. |
| `1` | The data does not satisfy the schema. |
| `2` | An argument, file reading, JSON parsing, or schema compilation error occurred. |

Errors with exit code `2` are written to standard error with an `Error:` prefix.
Missing or extra arguments produce the usage message with exit code `2`.
File reading and JSON parsing errors identify the operation, the file's role (`schema` or `data`), and the supplied file path, followed by the underlying error details.
Paths are enclosed in double quotes, with special characters such as quotes and newlines escaped.
For example, a JSON parsing error in the schema has the form `Error: failed to parse schema "schema.json": <details>`.
