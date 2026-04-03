# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`dynwinrt` is a Rust-based runtime library that enables dynamic invocation of Windows Runtime (WinRT) APIs. Unlike static projections (PyWinRT, C++/WinRT), this library uses runtime metadata (.winmd files) and FFI (libffi) to call arbitrary WinRT methods without native code generation. The goal is to provide a foundation for JavaScript and Python bindings that don't require MSVC compilation or version-specific generated code.

## Build Commands

```bash
# Build the library
cargo build

# Run all tests
cargo test

# Run a specific test
cargo test test_name

# Build in release mode
cargo build --release
```

## Environment Setup

**Critical**: Set the `WINAPPSDK_BOOTSTRAP_DLL_PATH` environment variable to the path of the WinAppSDK Bootstrap DLL before running tests that use WinAppSDK APIs (e.g., FileOpenPicker). Without this, WinAppSDK initialization will fail.

## Architecture

### Core Type System

The library implements a three-layer type system for bridging Rust and WinRT ABIs:

1. **WinRTType** (`types.rs`): High-level type descriptors (I32, I64, Object, HString, HResult, Pointer)
2. **AbiType** (`types.rs`): ABI-level representations (I32, I64, Ptr) that match calling conventions
3. **WinRTValue** and **AbiValue** (`value.rs`): Runtime value containers that hold actual data

This separation allows the library to handle WinRT's type system dynamically while maintaining correct ABI compatibility.

### Dynamic Method Invocation

The dynamic call mechanism (in `call.rs` and `signature.rs`) works as follows:

1. **InterfaceSignature** describes a WinRT interface with its GUID and method list
2. **MethodSignature** describes each method's parameters (in/out) and types
3. At runtime, the library:
   - Extracts function pointers from COM vtables using `get_vtable_function_ptr`
   - Allocates stack space for out parameters
   - Marshals arguments using libffi
   - Invokes the method via `call_winrt_method_dynamic`
   - Converts out parameters back to WinRTValue types

**Example**: See `test_winrt_uri_interop_using_signature` in `lib.rs:157` for a complete example of dynamic method invocation.

### Key Modules

- **call.rs**: Core FFI invocation logic, vtable pointer extraction, and dynamic method calling
- **signature.rs**: Interface and method signature definitions that describe WinRT APIs at runtime
- **types.rs**: Type system mapping between WinRT types and ABI representations
- **value.rs**: Runtime value containers with conversion methods
- **interfaces.rs**: Hand-written interface signatures for Uri, FileOpenPicker (examples for future code generation)
- **roapi.rs**: WinRT activation factory access via RoGetActivationFactory
- **winapp.rs**: WinAppSDK Bootstrap initialization for unpackaged applications
- **dasync.rs**: Async trait implementation (copied from windows-future for reference)

## Design Philosophy

### Dynamic vs Static Projection

This library intentionally uses **runtime** rather than **compile-time** approach:

- Interface shapes are represented as data (`InterfaceSignature`) not generated code
- WinMD metadata can be read at runtime or pre-processed
- No version coupling between the runtime and specific WinAppSDK versions
- Trades compile-time type safety for flexibility and ease of distribution

### Out Parameter Handling

WinRT methods use out parameters (pointers) for return values. The library handles this by:

1. Pre-allocating `AbiValue` storage on the stack
2. Passing pointers to these allocations via libffi
3. Converting the populated `AbiValue` to `WinRTValue` after the call succeeds

See `call_winrt_method_dynamic` in `call.rs:59` for implementation details.

## Testing Strategy

Tests use real Windows APIs without mocking:

- **Basic WinRT**: Uri, XmlDocument (from Windows.winmd)
- **Windows Web**: HttpClient with async operations
- **WinAppSDK**: FileOpenPicker requires WinAppSDK Bootstrap initialization
- **Metadata Reading**: Direct windows-metadata crate tests to verify type information

All tests assume Windows 10/11 with SDK installed at `C:\Program Files (x86)\Windows Kits\10\UnionMetadata\10.0.26100.0\Windows.winmd`.

## Common Patterns

### Creating an Interface Signature

```rust
let mut vtable = InterfaceSignature::new("InterfaceName".to_string(), iid);
vtable
    .add_method(MethodSignature::new()) // Standard COM methods (QI, AddRef, Release)
    .add_method(MethodSignature::new().add_out(WinRTType::HString)) // Getter
    .add_method(MethodSignature::new().add(WinRTType::HString).add_out(WinRTType::Object)); // Factory method
```

### Calling a Dynamic Method

```rust
let result = method.call_dynamic(com_obj.as_raw(), &[WinRTValue::HString(hstring)])?;
let return_value = result[0].as_hstring().unwrap();
```

## Known Limitations

- Only supports a subset of WinRT types (I32, I64, Object, HString, Pointer)
- No support for structs/value types passed by value yet
- No generic type support (IVector&lt;T&gt;, IAsyncOperation&lt;T&gt;)
- Interface signatures must be hand-written (future: generate from WinMD)
- vtable index must be manually counted (future: derive from metadata)

## Related Projects

- [lazy-winrt](https://github.com/JesseCol/lazy-winrt): Original JavaScript prototype
- [JS Binding](https://github.com/Hong-Xiang/dynwinrt-js): JavaScript bindings using napi-rs
- [Python Binding](https://github.com/Hong-Xiang/dynwinrt-py): Python bindings using PyO3

## Implementation Notes

### Why libffi?

libffi provides portable FFI that can call functions with arbitrary signatures at runtime. This is essential because WinRT method signatures are only known after reading WinMD metadata.

### COM Object Lifetimes

The library uses `windows-core::IUnknown` smart pointers which automatically handle AddRef/Release. Raw pointers extracted via `as_raw()` are only used for the duration of a single call.

### Async Operations

The async trait (`dasync.rs`) is currently a reference implementation copied from windows-future. Future work will integrate this with dynamic method invocation to support `IAsyncOperation<T>` without static type parameters.
