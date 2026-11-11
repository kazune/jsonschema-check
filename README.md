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

## Supported JSON Schema drafts

The supported drafts are Draft 4, Draft 6, Draft 7, Draft 2019-09, and Draft 2020-12.
The draft is detected automatically from the schema's `$schema` property.
If `$schema` is omitted, Draft 2020-12 is used.
There is no CLI option to select a draft.

For example, this schema explicitly selects Draft 2020-12:

```json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "string"
}
```

### Schema references

`$ref` can refer to subschemas within the same schema document, such as `#/definitions/item` or `#/$defs/item`.
External schema files are not loaded, and HTTP or HTTPS references are not fetched.
Relative references are not resolved against the schema file's directory.
If compiling a schema requires an unavailable external resource or an unresolved internal reference, the program reports an error on standard error and exits with code `2` in both output modes.
The standard meta-schemas for the supported drafts are bundled, so their `$schema` URIs do not require network access.

### Format validation

With the standard `$schema` URI for Draft 4, Draft 6, or Draft 7, recognized `format` values such as `email` are validated.
With the standard `$schema` URI for Draft 2019-09 or Draft 2020-12, `format` is treated as an annotation and does not affect whether data is valid.
This also applies when `$schema` is omitted, since the default draft is 2020-12.
Unknown format names are ignored with these standard drafts.
Other constraints, such as `type`, still apply.
There is no CLI option to override format validation.

## Usage

To validate data, pass the schema file path followed by the data file path, optionally preceded by `--output jsonl`.
Both files must contain valid JSON.
File paths use the operating system's native representation and do not need to be valid UTF-8.

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
Failures writing help, version information, or validation results produce exit code `2`.
If writing the error diagnostic to standard error also fails, the program exits with code `2` without further output attempts.
Missing or extra arguments produce the usage message with exit code `2`.
File reading and JSON parsing errors identify the operation, the file's role (`schema` or `data`), and the supplied file path, followed by the underlying error details.
Paths are enclosed in double quotes, with special characters such as quotes and newlines escaped.
Non-UTF-8 portions of paths are escaped in diagnostics.
For example, a JSON parsing error in the schema has the form `Error: failed to parse schema "schema.json": <details>`.
