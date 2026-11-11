# jsonschema-check

A minimal JSON Schema validation CLI with a small dependency footprint.

`jsonschema-check` validates a JSON file against a JSON Schema and reports validation errors as text to standard error or, with `--output jsonl`, as JSON Lines to standard output.
It uses the `jsonschema` library with default features disabled to keep dependencies small.

## Build

A Rust toolchain with support for edition 2024 is required.

```sh
cargo build --release --locked
```

The executable is available at `target/release/jsonschema-check`.

## Usage

To validate data, pass the schema file path followed by the data file path, optionally preceded by `--output jsonl`.
Both files must contain valid JSON.

```sh
jsonschema-check [--output jsonl] <schema.json> <data.json>
```

Use `--help` to display usage information or `--version` to display the program version.
Each is recognized as an option only when it is the sole argument, and writes to standard output with exit code `0` without reading any files.

```sh
jsonschema-check --help
jsonschema-check --version
```

In file argument positions, `--help` and `--version` are treated as literal file paths in both text and JSON Lines modes.
If such a file does not exist, the program reports a file reading error with exit code `2`.

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
With the default text output, changing `age` to `-1` produces a validation error on standard error and exits with code `1`:

```text
/age: -1 is less than the minimum of 0
```

Each reported validation error appears on its own line as `<instance path>: <message>`.
The instance path is a JSON Pointer into the data and is empty for errors at the document root.

## JSON Lines output

Place `--output jsonl` before the two file paths to write validation errors to standard output as JSON Lines.

```sh
./target/release/jsonschema-check --output jsonl schema.json data.json
```

Each validation error is a single JSON object on its own line, with exactly three string attributes.

| Attribute | Meaning |
| --- | --- |
| `instancePath` | JSON Pointer into the data, or an empty string for the document root. |
| `schemaPath` | JSON Pointer to the violated constraint in the schema, or an empty string for the schema root. |
| `message` | A human-readable explanation from the validation library. |

For example:

```jsonl
{"instancePath":"/age","schemaPath":"/properties/age/minimum","message":"-1 is less than the minimum of 0"}
```

Attribute order is unspecified, and `message` text should not be used for machine classification.
JSON Pointer tokens escape `~` as `~0` and `/` as `~1`, and array elements use numeric indices.
Quotes, newlines, and other special characters in attribute values are escaped as JSON.
For a missing required property, `instancePath` identifies the object that should contain the property.
All reported validation errors are emitted without headings, counts, or summary records.
Successful validation produces no output and exits with code `0`; validation errors produce exit code `1` and no standard error output.
Argument, file reading, JSON parsing, schema compilation, and output errors are reported as text on standard error with exit code `2`.
Check the exit code as well as the output, since output errors can leave incomplete JSON Lines.
Only the form `--output jsonl` before both file paths is supported; missing or unsupported values, repeated options, and other placements are argument errors.
`--output=jsonl` is not supported.

## Exit codes

| Code | Meaning |
| --- | --- |
| `0` | The data satisfies the schema, or help or version information was displayed. |
| `1` | The data does not satisfy the schema. |
| `2` | An argument, file reading, JSON parsing, schema compilation, or output error occurred. |

Errors with exit code `2` are written to standard error with an `Error:` prefix.
Missing or extra arguments produce the usage message with exit code `2`.
File reading and JSON parsing errors identify the operation, the file's role (`schema` or `data`), and the supplied file path, followed by the underlying error details.
Paths are enclosed in double quotes, with special characters such as quotes and newlines escaped.
For example, a JSON parsing error in the schema has the form `Error: failed to parse schema "schema.json": <details>`.
