# dynwinrt

Dynamic WinRT API invocation — call any Windows Runtime method at runtime without native code generation.

## Overview

`dynwinrt` is a Rust library that uses runtime metadata (.winmd files) and FFI (libffi) to call arbitrary WinRT methods dynamically. It provides a foundation for JavaScript and Python bindings that don't require MSVC compilation or version-specific generated code.

## Repository Structure

```
dynwinrt/
├── crates/dynwinrt/       # Core Rust library
├── bindings/
│   ├── js/                # JavaScript/TypeScript bindings (napi-rs)
│   └── py/                # Python bindings (PyO3)
└── tools/
    └── winrt-meta/        # → d:\work\winrt-meta (code generator)
```

## Build

```bash
# Build the core library
cargo build -p dynwinrt

# Run tests
cargo test -p dynwinrt

# Build JS bindings
cd bindings/js && npm install && npx napi build --no-const-enum --platform --release -o dist

# Build Python bindings
cd bindings/py && maturin develop
```

## Code Generation with winrt-meta

`winrt-meta` reads Windows metadata (.winmd) files and generates typed TypeScript bindings that use `dynwinrt-js` at runtime.

### Step 1: Build winrt-meta

```bash
cd d:\work\winrt-meta
cargo build --release
```

### Step 2: Generate Bindings

```bash
# Generate for a specific class
cargo run --release -- generate \
  --winmd "C:\Program Files (x86)\Windows Kits\10\UnionMetadata\10.0.26100.0\Windows.winmd" \
  --namespace "Windows.Foundation" \
  --class-name "Uri" \
  --lang ts \
  --output ./generated/Windows.Foundation

# Generate for an entire namespace
cargo run --release -- generate \
  --winmd "C:\Program Files (x86)\Windows Kits\10\UnionMetadata\10.0.26100.0\Windows.winmd" \
  --namespace "Windows.Web.Http" \
  --lang ts \
  --output ./generated/Windows.Web.Http
```

**Arguments:**

| Argument | Required | Description |
|---|---|---|
| `--winmd` | Yes | Path to .winmd file(s), separated by `;` |
| `--namespace` | Yes | WinRT namespace to generate |
| `--class-name` | No | Specific class (generates dependencies too) |
| `--lang` | No | Target language: `ts` (default) |
| `--output` | No | Output directory |

### Step 3: Fix Import Paths (local development)

Generated files import from `'dynwinrt-js'`. For local development, fix to relative path:

```bash
# Replace package import with relative path to built bindings
find generated -name "*.ts" -exec sed -i "s|from 'dynwinrt-js'|from '../../dist/index.js'|g" {} +
```

### Step 4: Use Generated Bindings

```typescript
import { roInitialize } from 'dynwinrt-js'
import { Uri } from './generated/Windows.Foundation/Uri'

roInitialize(1) // Initialize WinRT (MTA)

const uri = Uri.createUri('https://example.com/path?q=1')
console.log(uri.host)       // "example.com"
console.log(uri.port)       // 443
console.log(uri.schemeName) // "https"
```

### What Gets Generated

For each WinRT class, winrt-meta generates:

- **Interface registration** — `DynWinRtType.registerInterface()` with all methods and type signatures
- **Wrapper class** — TypeScript class with typed properties and methods
- **Factory methods** — Static methods for object creation (via activation factory)
- **Collection types** — `IVector<T>`, `IMap<K,V>`, etc. wrappers in `_collections.ts`
- **Enums** — TypeScript `enum` declarations

### Running Tests

```bash
# Core library tests
cargo test -p dynwinrt

# JS binding test (uses generated pattern)
cd bindings/js && npx tsx __test__/test_generated.ts

# Python binding tests
cd bindings/py && pytest
```

## Use WinAppSDK Bootstrap

The path to the WinAppSDK Bootstrap DLL is retrieved from the `WINAPPSDK_BOOTSTRAP_DLL_PATH` environment variable. Only needed for unpackaged apps using WinAppSDK APIs.

```typescript
import { initWinappsdk } from 'dynwinrt-js'
initWinappsdk(1, 8) // Initialize WinAppSDK 1.8
```

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `cargo build` fails with libffi errors | Ensure you have a C compiler (MSVC) and Windows SDK installed |
| WinAppSDK APIs fail at runtime | Set `WINAPPSDK_BOOTSTRAP_DLL_PATH` environment variable |
| `cargo test -p dynwinrt` fails | Ensure Windows SDK is installed at default path with `Windows.winmd` |
| JS bindings won't build | Run `npm install` first; requires Node.js 18+ |
| Python bindings won't build | Requires Python 3.8+ and `maturin` (`pip install maturin`) |
| winrt-meta snapshot tests fail | Line-ending differences — run `cargo test -p winrt-meta -- --include-ignored` to regenerate |

## Contributing

This project welcomes contributions and suggestions. Most contributions require you to agree to a
Contributor License Agreement (CLA) declaring that you have the right to, and actually do, grant us
the rights to use your contribution. For details, visit https://cla.opensource.microsoft.com.

When you submit a pull request, a CLA bot will automatically determine whether you need to provide
a CLA and decorate the PR appropriately (e.g., status check, comment). Simply follow the instructions
provided by the bot. You will only need to do this once across all repos using our CLA.

This project has adopted the [Microsoft Open Source Code of Conduct](https://opensource.microsoft.com/codeofconduct/).
For more information see the [Code of Conduct FAQ](https://opensource.microsoft.com/codeofconduct/faq/) or
contact [opencode@microsoft.com](mailto:opencode@microsoft.com) with any additional questions or comments.

## Trademarks

This project may contain trademarks or logos for projects, products, or services. Authorized use of Microsoft
trademarks or logos is subject to and must follow
[Microsoft's Trademark & Brand Guidelines](https://www.microsoft.com/en-us/legal/intellectualproperty/trademarks/usage/general).
Use of Microsoft trademarks or logos in modified versions of this project must not cause confusion or imply Microsoft sponsorship.
Any use of third-party trademarks or logos are subject to those third-party's policies.

## License

This project is licensed under the [MIT License](LICENSE).