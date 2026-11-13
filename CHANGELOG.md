# Changelog

## 0.2.0 (unreleased)

- Enable IDNA support for the `idn-email` and `idn-hostname` formats, including validation under Draft 2020-12 Format-Assertion.
- Recognized IDNA formats reject invalid values with exit code `1` when format validation is active, rather than being ignored or rejected as unknown formats.
- Add IDNA and ICU dependencies.

## 0.1.0 - 2026-11-13

Initial release.

- Validate a JSON data file against a JSON Schema file using Draft 4, Draft 6, Draft 7, Draft 2019-09, or Draft 2020-12.
- Detect the draft from `$schema`, defaulting to Draft 2020-12 when omitted.
- Preserve arbitrary-precision integers and decimals when parsing and validating schema and data files, including numeric bounds, equality constraints, and `multipleOf`.
- Resolve references within the schema document without fetching external schema files or HTTP/HTTPS resources.
- The `idn-hostname` and `idn-email` formats are unsupported: standard draft meta-schemas ignore them, while Format-Assertion rejects them as unknown formats.
- Report validation errors as text on standard error or, with `--output jsonl`, as JSON Lines on standard output with `instancePath`, `schemaPath`, and `message` attributes.
- Provide `--help` and `--version` when passed as the sole argument.
- Use exit code `0` for successful validation or help/version output, `1` for validation failure, and `2` for argument, file reading, JSON parsing, schema compilation, or output errors.
- Accept native operating system file paths, including non-UTF-8 paths, and include file context in reading and JSON parsing errors.
- Distribute source under the MIT license with Cargo installation instructions.
