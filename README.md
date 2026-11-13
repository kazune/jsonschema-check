# jsonschema-check

A minimal JSON Schema validation CLI with a small dependency footprint.

`jsonschema-check` validates a JSON file against a JSON Schema and reports validation errors as text to standard error or, with `--output jsonl`, as JSON Lines to standard output.
It uses the `jsonschema` library with default features disabled to keep dependencies small.

## Installation

A Rust toolchain with support for edition 2024 is required.

To install from a local clone:

```sh
git clone https://github.com/kazune/jsonschema-check.git
cd jsonschema-check
cargo install --path . --locked
```

To install directly from a specific GitHub commit, replace `COMMIT` with the full commit hash:

```sh
cargo install --git https://github.com/kazune/jsonschema-check.git --rev COMMIT --locked
```

Both methods build from source and use the dependencies recorded in `Cargo.lock`.
With Cargo's default installation settings, the executable is installed in `$HOME/.cargo/bin`.
Ensure Cargo's installation `bin` directory is on your `PATH`, then check the installation:

```sh
jsonschema-check --version
```

## Build

To build without installing:

```sh
cargo build --release --locked
```

The executable is available at `target/release/jsonschema-check`.

## Supported JSON Schema drafts

The supported drafts are Draft 4, Draft 6, Draft 7, Draft 2019-09, and Draft 2020-12.
The draft is detected automatically from the schema's `$schema` property.
If `$schema` is omitted, Draft 2020-12 is used.
There is no CLI option to select a draft.
See the [validation behavior appendix](#appendix-validation-behavior) for format validation and implementation details.

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

## Appendix: Validation behavior

### Format validation

With the standard `$schema` URI for Draft 4, Draft 6, or Draft 7, recognized `format` values such as `email` are validated.
With the standard `$schema` URI for Draft 2019-09 or Draft 2020-12, `format` is treated as an annotation and does not affect whether data is valid.
This also applies when `$schema` is omitted, since the default draft is 2020-12.
Unknown format names are ignored with these standard drafts.
Other constraints, such as `type`, still apply.
There is no CLI option to override format validation.

Draft 2020-12 also supports the [Format-Assertion vocabulary](https://json-schema.org/draft/2020-12/draft-bhutton-json-schema-validation-01#section-7.2.2), which requires format validation when selected through a meta-schema declaring that vocabulary.
A custom meta-schema can be embedded in the same document under `$defs`, identified by `$id`, and selected through `$schema` without external retrieval.
Adding `$vocabulary` to a schema that still selects the standard Draft 2020-12 meta-schema does not enable format assertions.
With Format-Assertion active, unknown format names cause a schema compilation error with exit code `2`.

The `idna` feature enables the internationalized `idn-email` and `idn-hostname` formats.
`idn-hostname` is recognized for Draft 7 and later; Draft 4 and Draft 6 treat it as an unknown format.
Recognized IDNA formats follow the same format validation rules described above: standard Draft 2019-09 and Draft 2020-12 treat them as annotations, while selecting Format-Assertion validates their values.
IDNA processing is local and does not perform network lookups.

### Numeric precision

JSON numbers in both schema and data files are parsed and validated with arbitrary precision.
Large integers and decimal values retain their precision without rounding to `f64`.
This applies to numeric comparisons, equality constraints, and `multipleOf`, including values written with exponents such as `1e400` or `1e-400`.
Processing time and memory use depend on the size of the numbers and the validation constraints.

### Further details

For regular expression handling and other validation details, consult the [`jsonschema` library documentation](https://docs.rs/jsonschema) for the version specified in `Cargo.toml`.
The JSON Schema documentation describes [regular expression syntax and portability](https://json-schema.org/understanding-json-schema/reference/regular_expressions).
