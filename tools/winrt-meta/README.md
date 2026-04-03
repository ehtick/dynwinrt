# winrt-meta

Read Windows metadata (`.winmd`) files and generate typed bindings that use `dynwinrt-js` at runtime.

## Install

```bash
npm install -D winrt-meta
```

## Usage

```bash
npx winrt-meta generate [OPTIONS]
```

### Arguments

| Argument | Required | Description |
|---|---|---|
| `--winmd` | No | Path to `.winmd` file(s), separated by `;`. Auto-detects Windows SDK if omitted |
| `--folder` | No | Directory containing `.winmd` files |
| `--namespace` | No | Generate only this namespace. If omitted, generates all non-Windows namespaces |
| `--class-name` | No | Class name(s) to generate, comma-separated (requires `--namespace`). E.g. `StorageFile` or `StorageFile,StorageFolder` |
| `--ref` | No | Additional `.winmd` files for type resolution only (no code generated). Paths separated by `;` |
| `--lang` | No | Target language: `ts` (default), `js` (ESM), `cjs` (CommonJS) |
| `--output` | No | Output directory (default: `./generated`) |
| `--dry-run` | No | Validate metadata and resolve dependencies without writing files |
| `--source-map` | No | Generate `.map` source map files alongside JS output (only with `--lang js` or `cjs`) |

When `--lang js` or `--lang cjs` is specified, TypeScript is generated internally and compiled to JavaScript via SWC. The intermediate `.ts` files are not written to the output directory.

### Examples

Generate JavaScript (ESM) bindings from a WinAppSDK metadata folder:

```bash
npx winrt-meta generate \
  --folder path/to/metadata \
  --output ./generated-js \
  --lang js
```

Generate TypeScript bindings for a specific class:

```bash
npx winrt-meta generate \
  --namespace Windows.Storage \
  --class-name StorageFile \
  --output ./generated-ts
```

Generate multiple classes in one pass (shares the winmd index):

```bash
npx winrt-meta generate \
  --namespace Windows.Storage \
  --class-name StorageFile,StorageFolder \
  --output ./generated-ts
```

Generate all namespaces from multiple `.winmd` files:

```bash
npx winrt-meta generate \
  --winmd "path/to/Windows.winmd;path/to/Microsoft.WindowsAppSDK.winmd" \
  --output ./generated-ts
```

Validate metadata without writing files:

```bash
npx winrt-meta generate \
  --folder path/to/metadata \
  --dry-run
```

Generate JS with source maps for debugging:

```bash
npx winrt-meta generate \
  --folder path/to/metadata \
  --output ./generated-js \
  --lang js \
  --source-map
```

## Output

For each WinRT class, the tool generates:

- **Interface registration** -- `DynWinRtType.registerInterface()` with all methods and type signatures
- **Wrapper class** -- typed class with properties and methods
- **Factory methods** -- static methods for object creation via activation factory
- **Enums** -- enum declarations
- **Collection types** -- `IVector<T>`, `IVectorView<T>`, `IMap<K,V>`, etc.
- **Index file** -- re-exporting all generated types

Dependencies are resolved automatically -- specifying `--class StorageFile` will also generate referenced types like `Uri`, enums, and interfaces.

## Build from Source

```bash
cargo build -p winrt-meta --release
```

The compiled executable needs to be copied into the npm package before publishing:

```bash
# x64
cargo build -p winrt-meta --release
cp target/release/winrt-meta.exe tools/winrt-meta/npm/bin/x64/

# arm64
cargo build -p winrt-meta --release --target aarch64-pc-windows-msvc
cp target/aarch64-pc-windows-msvc/release/winrt-meta.exe tools/winrt-meta/npm/bin/arm64/
```

Then publish:

```bash
cd tools/winrt-meta/npm
npm publish
```

In CI, this is handled automatically by the build workflow.

## Testing

```bash
cargo test -p winrt-meta
```

Tests include:
- Unit tests for type mapping, dependency resolution, and code generation helpers
- Snapshot test for `Windows.Foundation.Uri` (regenerate with `cargo run -- generate --namespace Windows.Foundation --class-name Uri --output tests/snapshots/uri`)
