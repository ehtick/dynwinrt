use std::collections::HashSet;
use std::sync::LazyLock;

use crate::meta::{ClassMeta, InterfaceMeta, MethodMeta, ParamDirection};
use crate::types::{TypeKind, TypeMeta, TypeRef};

/// Empty set passed as `deferred` for codegen (no circular dep handling needed).
pub(crate) static NO_DEFERRED: LazyLock<HashSet<String>> = LazyLock::new(HashSet::new);

// ======================================================================
// Struct collection helpers
// ======================================================================

/// Recursively collect non-HResult struct types from a type tree.
fn collect_used_structs_from_type(typ: &TypeMeta, seen: &mut HashSet<String>, result: &mut Vec<TypeMeta>) {
    match typ {
        TypeMeta::Struct { namespace, name, fields } => {
            if name != "HResult" {
                let full = format!("{}.{}", namespace, name);
                if !seen.insert(full) {
                    return; // already collected
                }
            }
            // Recurse into fields FIRST so nested structs appear before this one
            for f in fields { collect_used_structs_from_type(&f.typ, seen, result); }
            if name != "HResult" {
                result.push(typ.clone());
            }
        }
        TypeMeta::AsyncOperation(inner) | TypeMeta::AsyncActionWithProgress(inner) => {
            collect_used_structs_from_type(inner, seen, result);
        }
        TypeMeta::AsyncOperationWithProgress(r, p) => {
            collect_used_structs_from_type(r, seen, result);
            collect_used_structs_from_type(p, seen, result);
        }
        TypeMeta::Array(inner) => collect_used_structs_from_type(inner, seen, result),
        TypeMeta::Parameterized { args, .. } => {
            for arg in args { collect_used_structs_from_type(arg, seen, result); }
        }
        _ => {}
    }
}

pub(crate) fn collect_used_structs_from_class(class: &ClassMeta) -> Vec<TypeMeta> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for iface in class.all_interfaces() {
        for m in &iface.methods {
            for p in &m.params { collect_used_structs_from_type(&p.typ, &mut seen, &mut result); }
            if let Some(ref rt) = m.return_type { collect_used_structs_from_type(rt, &mut seen, &mut result); }
        }
    }
    result
}

pub(crate) fn collect_used_structs_from_iface(iface: &InterfaceMeta) -> Vec<TypeMeta> {
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for m in &iface.methods {
        for p in &m.params { collect_used_structs_from_type(&p.typ, &mut seen, &mut result); }
        if let Some(ref rt) = m.return_type { collect_used_structs_from_type(rt, &mut seen, &mut result); }
    }
    result
}

// ======================================================================
// Struct field type helpers
// ======================================================================

/// Map a struct field type to its TypeScript type annotation.
pub(crate) fn ts_struct_field_type(typ: &TypeMeta) -> String {
    match typ {
        TypeMeta::Bool => "boolean".to_string(),
        TypeMeta::String => "string".to_string(),
        TypeMeta::Guid => "string".to_string(),
        TypeMeta::I8 | TypeMeta::U8 | TypeMeta::I16 | TypeMeta::U16 | TypeMeta::Char16
        | TypeMeta::I32 | TypeMeta::U32 | TypeMeta::I64 | TypeMeta::U64
        | TypeMeta::F32 | TypeMeta::F64 | TypeMeta::Enum { .. } => "number".to_string(),
        TypeMeta::Struct { name, .. } if name == "HResult" => "number".to_string(),
        TypeMeta::Struct { name, .. } => name.clone(),
        _ => "DynWinRtValue".to_string(),
    }
}

/// Generate a `DynWinRtStruct.getXxx(index)` expression for a struct field.
pub(crate) fn struct_field_getter(typ: &TypeMeta, index: usize) -> String {
    match typ {
        TypeMeta::Bool => format!("s.getU8({}) !== 0", index),
        TypeMeta::I8 => format!("s.getI8({})", index),
        TypeMeta::U8 => format!("s.getU8({})", index),
        TypeMeta::I16 => format!("s.getI16({})", index),
        TypeMeta::U16 | TypeMeta::Char16 => format!("s.getU16({})", index),
        TypeMeta::I32 | TypeMeta::Enum { .. } => format!("s.getI32({})", index),
        TypeMeta::U32 => format!("s.getU32({})", index),
        TypeMeta::I64 => format!("s.getI64({})", index),
        TypeMeta::U64 => format!("s.getU64({})", index),
        TypeMeta::F32 => format!("s.getF32({})", index),
        TypeMeta::F64 => format!("s.getF64({})", index),
        TypeMeta::String => format!("s.getHstring({})", index),
        TypeMeta::Guid => format!("s.getGuid({}).toString()", index),
        TypeMeta::Struct { name, .. } if name == "HResult" => format!("s.getI32({})", index),
        TypeMeta::Struct { name, .. } => format!("_unpack{}(s.getStruct({}).toValue())", name, index),
        _ => format!("s.getObject({})", index), // IReference<T> etc.
    }
}

/// Generate a `s.setXxx(index, expr)` statement for a struct field.
pub(crate) fn struct_field_setter(typ: &TypeMeta, index: usize, value_expr: &str) -> String {
    match typ {
        TypeMeta::Bool => format!("s.setU8({}, {} ? 1 : 0)", index, value_expr),
        TypeMeta::I8 => format!("s.setI8({}, {})", index, value_expr),
        TypeMeta::U8 => format!("s.setU8({}, {})", index, value_expr),
        TypeMeta::I16 => format!("s.setI16({}, {})", index, value_expr),
        TypeMeta::U16 | TypeMeta::Char16 => format!("s.setU16({}, {})", index, value_expr),
        TypeMeta::I32 | TypeMeta::Enum { .. } => format!("s.setI32({}, {})", index, value_expr),
        TypeMeta::U32 => format!("s.setU32({}, {})", index, value_expr),
        TypeMeta::I64 => format!("s.setI64({}, {})", index, value_expr),
        TypeMeta::U64 => format!("s.setU64({}, {})", index, value_expr),
        TypeMeta::F32 => format!("s.setF32({}, {})", index, value_expr),
        TypeMeta::F64 => format!("s.setF64({}, {})", index, value_expr),
        TypeMeta::String => format!("s.setHstring({}, {})", index, value_expr),
        TypeMeta::Guid => format!("s.setGuid({}, WinGuid.parse({}))", index, value_expr),
        TypeMeta::Struct { name, .. } if name == "HResult" => format!("s.setI32({}, {})", index, value_expr),
        TypeMeta::Struct { name, .. } => format!("s.setStruct({}, _pack{}({}))", index, name, value_expr),
        _ => format!("s.setObject({}, {})", index, value_expr), // IReference<T> etc.
    }
}

// ======================================================================
// Method signature builder
// ======================================================================

/// Build a `new DynWinRtMethodSig().addIn(...)...addOut(...)` expression.
pub(crate) fn build_method_sig(method: &MethodMeta) -> String {
    let mut parts = Vec::new();

    // In params
    for param in &method.params {
        if param.direction == ParamDirection::In {
            parts.push(format!(".addIn({})", ts_dynwinrt_type(&param.typ)));
        }
    }

    // Out params (explicit [out] parameters in method signature)
    for param in &method.params {
        if param.direction == ParamDirection::Out {
            parts.push(format!(".addOut({})", ts_dynwinrt_type(&param.typ)));
        } else if param.direction == ParamDirection::OutFill {
            parts.push(format!(".addOutFill({})", ts_dynwinrt_type(&param.typ)));
        }
    }

    // Return type (WinRT return value = [out, retval])
    if let Some(ref return_type) = method.return_type {
        parts.push(format!(".addOut({})", ts_dynwinrt_type(return_type)));
    }

    if parts.is_empty() {
        "new DynWinRtMethodSig()".to_string()
    } else {
        format!("new DynWinRtMethodSig(){}", parts.join(""))
    }
}

// ======================================================================
// Type expression: recursive expansion
// ======================================================================

/// Map a TypeMeta to a fully-expanded `DynWinRtType.*()` expression.
/// Recursively expands all compound types to leaf primitives.
pub(crate) fn ts_dynwinrt_type(typ: &TypeMeta) -> String {
    match typ {
        // Primitives
        TypeMeta::Bool => "DynWinRtType.boolType()".to_string(),
        TypeMeta::I8 => "DynWinRtType.i8Type()".to_string(),
        TypeMeta::I16 => "DynWinRtType.i16()".to_string(),
        TypeMeta::Char16 => "DynWinRtType.u16()".to_string(),
        TypeMeta::I32 => "DynWinRtType.i32()".to_string(),
        TypeMeta::U8 => "DynWinRtType.u8()".to_string(),
        TypeMeta::U16 => "DynWinRtType.u16()".to_string(),
        TypeMeta::U32 => "DynWinRtType.u32()".to_string(),
        TypeMeta::I64 => "DynWinRtType.i64()".to_string(),
        TypeMeta::U64 => "DynWinRtType.u64()".to_string(),
        TypeMeta::F32 => "DynWinRtType.f32()".to_string(),
        TypeMeta::F64 => "DynWinRtType.f64()".to_string(),

        // Strings
        TypeMeta::String => "DynWinRtType.hstring()".to_string(),

        // GUID — native type in dynwinrt
        TypeMeta::Guid => "DynWinRtType.guidType()".to_string(),

        // Generic object
        TypeMeta::Object => "DynWinRtType.object()".to_string(),

        // Interface — use interface(IID) if available
        TypeMeta::Interface { iid, .. } if !iid.is_empty() => {
            format!("DynWinRtType.interface(WinGuid.parse('{}'))", iid)
        }
        TypeMeta::Interface { .. } => "DynWinRtType.object()".to_string(),

        // RuntimeClass — runtimeClass(fullName, defaultIID)
        TypeMeta::RuntimeClass { namespace, name, default_iid } => {
            let full_name = format!("{}.{}", namespace, name);
            if !default_iid.is_empty() {
                format!(
                    "DynWinRtType.runtimeClass('{}', WinGuid.parse('{}'))",
                    full_name, default_iid
                )
            } else {
                "DynWinRtType.object()".to_string()
            }
        }

        // Delegate — COM pointer
        TypeMeta::Delegate { .. } => "DynWinRtType.object()".to_string(),

        // Async patterns — recursively expand inner types
        TypeMeta::AsyncOperation(inner) => {
            format!("DynWinRtType.iAsyncOperation({})", ts_dynwinrt_type(inner))
        }
        TypeMeta::AsyncOperationWithProgress(result, progress) => {
            format!("DynWinRtType.iAsyncOperationWithProgress({}, {})",
                ts_dynwinrt_type(result), ts_dynwinrt_type(progress))
        }
        TypeMeta::AsyncAction => {
            "DynWinRtType.iAsyncAction()".to_string()
        }
        TypeMeta::AsyncActionWithProgress(progress) => {
            format!("DynWinRtType.iAsyncActionWithProgress({})", ts_dynwinrt_type(progress))
        }

        // Struct — named for correct IID signature, recursively expand fields
        TypeMeta::Struct { namespace, name, fields } => {
            let full_name = format!("{}.{}", namespace, name);
            let field_types: Vec<String> = fields.iter()
                .map(|f| ts_dynwinrt_type(&f.typ))
                .collect();
            format!("DynWinRtType.structType('{}', [{}])", full_name, field_types.join(", "))
        }

        // Array — recursively expand element type
        TypeMeta::Array(inner) => {
            format!("DynWinRtType.arrayType({})", ts_dynwinrt_type(inner))
        }

        // Enum — named for correct IID signature, with member values
        TypeMeta::Enum { namespace, name, members, .. } => {
            let full_name = format!("{}.{}", namespace, name);
            if members.is_empty() {
                format!("DynWinRtType.enumType('{}')", full_name)
            } else {
                let names: Vec<String> = members.iter().map(|m| format!("'{}'", m.name)).collect();
                let values: Vec<String> = members.iter().map(|m| m.value.to_string()).collect();
                format!("DynWinRtType.enumType('{}', [{}], [{}])",
                    full_name, names.join(", "), values.join(", "))
            }
        }

        // Parameterized — preserve generic type info for IID computation
        TypeMeta::Parameterized { piid, args, .. } => {
            if piid.is_empty() {
                "DynWinRtType.object()".to_string()
            } else {
                let arg_types: Vec<String> = args.iter().map(|a| ts_dynwinrt_type(a)).collect();
                format!("DynWinRtType.parameterized(WinGuid.parse('{}'), [{}])", piid, arg_types.join(", "))
            }
        }
    }
}

// ======================================================================
// Argument wrapping
// ======================================================================

pub(crate) fn build_args_expr(in_params: &[&crate::meta::ParamMeta]) -> String {
    in_params.iter()
        .map(|p| wrap_arg(&to_camel_case(&p.name), &p.typ))
        .collect::<Vec<_>>()
        .join(", ")
}

pub(crate) fn wrap_arg(name: &str, typ: &TypeMeta) -> String {
    match typ {
        TypeMeta::String => format!("DynWinRtValue.hstring({})", name),
        TypeMeta::Bool => format!("DynWinRtValue.boolValue({})", name),
        TypeMeta::I32 | TypeMeta::U32 | TypeMeta::Enum { .. }
        | TypeMeta::I8 | TypeMeta::U8 | TypeMeta::I16 | TypeMeta::U16
        | TypeMeta::Char16 => {
            format!("DynWinRtValue.i32({})", name)
        }
        TypeMeta::I64 | TypeMeta::U64 => format!("DynWinRtValue.i64({})", name),
        TypeMeta::F32 => format!("DynWinRtValue.f32({})", name),
        TypeMeta::F64 => format!("DynWinRtValue.f64({})", name),
        TypeMeta::Guid => format!("DynWinRtValue.guid(WinGuid.parse({}))", name),
        TypeMeta::RuntimeClass { .. } | TypeMeta::Object | TypeMeta::Interface { .. }
        | TypeMeta::Parameterized { .. } | TypeMeta::Delegate { .. } => {
            format!("({} as any)._obj ?? {}", name, name)
        }
        TypeMeta::Array(_) => format!("{}.toValue()", name),
        TypeMeta::Struct { name: struct_name, .. } if struct_name == "HResult" => {
            format!("DynWinRtValue.i32({})", name)
        }
        TypeMeta::Struct { name: struct_name, .. } => {
            format!("_pack{}({}).toValue()", struct_name, name)
        }
        _ => name.to_string(),
    }
}

// ======================================================================
// Return conversion
// ======================================================================

/// Resolve a type name, using `_m_X.X` for deferred (lazy module ref) imports.
pub(crate) fn resolve_type_name(name: &str, deferred: &HashSet<String>) -> String {
    if deferred.contains(name) {
        format!("_m_{0}.{0}", name)
    } else {
        name.to_string()
    }
}

/// Convert an array return expression to the appropriate JS array type.
pub(crate) fn convert_array_return(arr_expr: &str, inner: &TypeMeta, known_types: &HashSet<String>, deferred: &HashSet<String>) -> String {
    match inner {
        TypeMeta::I8 => format!("{}.toI8Vec()", arr_expr),
        TypeMeta::U8 => format!("{}.toU8Vec()", arr_expr),
        TypeMeta::I16 => format!("{}.toI16Vec()", arr_expr),
        TypeMeta::U16 | TypeMeta::Char16 => format!("{}.toU16Vec()", arr_expr),
        TypeMeta::I32 | TypeMeta::Enum { .. } => format!("{}.toI32Vec()", arr_expr),
        TypeMeta::U32 => format!("{}.toU32Vec()", arr_expr),
        TypeMeta::I64 => format!("{}.toI64Vec()", arr_expr),
        TypeMeta::U64 => format!("{}.toU64Vec()", arr_expr),
        TypeMeta::F32 => format!("{}.toF32Vec()", arr_expr),
        TypeMeta::F64 => format!("{}.toF64Vec()", arr_expr),
        TypeMeta::Bool => format!("{}.toValues().map(v => v.toBool())", arr_expr),
        TypeMeta::String => format!("{}.toStringVec()", arr_expr),
        TypeMeta::Guid => format!("{}.toValues().map(v => v.toString())", arr_expr),
        TypeMeta::Struct { name, .. } if name == "HResult" => format!("{}.toI32Vec()", arr_expr),
        TypeMeta::Struct { name, .. } => format!("{}.toValues().map(v => _unpack{}(v))", arr_expr, name),
        TypeMeta::RuntimeClass { name, .. } if known_types.contains(name) => {
            let r = resolve_type_name(name, deferred);
            format!("{}.toValues().map(v => new {}(v))", arr_expr, r)
        }
        TypeMeta::Interface { name, .. } if known_types.contains(name) => {
            let r = resolve_type_name(name, deferred);
            format!("{}.toValues().map(v => new {}(v))", arr_expr, r)
        }
        _ => format!("{}.toValues()", arr_expr),
    }
}

pub(crate) fn convert_return(expr: &str, return_type: Option<&TypeMeta>, is_async: bool, known_types: &HashSet<String>, deferred: &HashSet<String>) -> String {
    if is_async {
        let inner_type = match return_type {
            Some(TypeMeta::AsyncOperation(inner)) => Some(inner.as_ref()),
            Some(TypeMeta::AsyncOperationWithProgress(inner, _)) => Some(inner.as_ref()),
            _ => None,
        };
        let awaited = format!("(await {}.toPromise())", expr);
        return convert_return(&awaited, inner_type, false, known_types, deferred);
    }
    match return_type {
        Some(TypeMeta::String) | Some(TypeMeta::Guid) => format!("{}.toString()", expr),
        Some(TypeMeta::I8 | TypeMeta::U8 | TypeMeta::I16 | TypeMeta::U16 | TypeMeta::Char16
            | TypeMeta::I32 | TypeMeta::U32) => format!("{}.toNumber()", expr),
        Some(TypeMeta::I64 | TypeMeta::U64) => format!("{}.toI64()", expr),
        Some(TypeMeta::F32 | TypeMeta::F64) => format!("{}.toF64()", expr),
        Some(TypeMeta::Bool) => format!("{}.toBool()", expr),
        Some(TypeMeta::Enum { .. }) => format!("{}.toNumber()", expr),
        Some(TypeMeta::RuntimeClass { name, .. }) if known_types.contains(name) => {
            let r = resolve_type_name(name, deferred);
            format!("new {}({})", r, expr)
        }
        Some(TypeMeta::Struct { name, .. }) if name == "HResult" => format!("{}.toNumber()", expr),
        Some(TypeMeta::Struct { name, .. }) => format!("_unpack{}({})", name, expr),
        Some(TypeMeta::Delegate { .. }) => expr.to_string(),
        Some(TypeMeta::Interface { name, .. }) if known_types.contains(name) => {
            let r = resolve_type_name(name, deferred);
            format!("new {}({})", r, expr)
        }
        Some(TypeMeta::Parameterized { name, args, .. }) => {
            let concrete = crate::meta::make_parameterized_name(name, args);
            if known_types.contains(&concrete) {
                let r = resolve_type_name(&concrete, deferred);
                format!("new {}({})", r, expr)
            } else {
                expr.to_string()
            }
        }
        Some(TypeMeta::Array(inner)) => {
            let arr_expr = format!("{}.asArray()", expr);
            convert_array_return(&arr_expr, inner, known_types, deferred)
        }
        _ => expr.to_string(),
    }
}

// ======================================================================
// Interface registration helper
// ======================================================================

/// Generate a `const <var_name> = DynWinRtType.registerInterface(...)` block.
/// `var_name` controls the JS variable name (e.g. `"_IFoo"` for class-internal use).
pub(crate) fn generate_interface_registration(iface: &InterfaceMeta, var_name: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("const {} = DynWinRtType.registerInterface(\n", var_name));
    out.push_str(&format!("    \"{}\", IID_{})\n", iface.name, iface.name));
    for method in &iface.methods {
        out.push_str(&format!(
            "    .addMethod(\"{}\", {})\n",
            method.name,
            build_method_sig(method)
        ));
    }
    trim_trailing_newline_add_semicolon(&mut out);
    out
}

pub(crate) fn trim_trailing_newline_add_semicolon(out: &mut String) {
    if out.ends_with(")\n") {
        out.truncate(out.len() - 1);
        out.push_str(";\n");
    }
}

// ======================================================================
// Generic collection helpers
// ======================================================================

/// Collect the set of known generic collection names used in method signatures.
/// Returns e.g. ["IVectorView", "IMap"] for import generation.
pub(crate) fn collect_used_generics_from_methods(methods: &[MethodMeta]) -> Vec<String> {
    let refs: Vec<&MethodMeta> = methods.iter().collect();
    collect_used_generics_from_methods_inner(&refs)
}

/// Shared implementation for collecting generic names from method references.
fn collect_used_generics_from_methods_inner(methods: &[&MethodMeta]) -> Vec<String> {
    let mut names: HashSet<String> = HashSet::new();
    fn visit(typ: &TypeMeta, names: &mut HashSet<String>) {
        match typ {
            TypeMeta::Parameterized { name, args, .. } => {
                names.insert(crate::meta::make_parameterized_name(name, args));
                for arg in args { visit(arg, names); }
            }
            TypeMeta::AsyncOperation(inner) | TypeMeta::AsyncActionWithProgress(inner) => visit(inner, names),
            TypeMeta::AsyncOperationWithProgress(r, p) => { visit(r, names); visit(p, names); }
            TypeMeta::Array(inner) => visit(inner, names),
            _ => {}
        }
    }
    for m in methods {
        for p in &m.params { visit(&p.typ, &mut names); }
        if let Some(ref rt) = m.return_type { visit(rt, &mut names); }
    }
    let mut sorted: Vec<String> = names.into_iter().collect();
    sorted.sort();
    sorted
}

/// Collect all used generic names from a class (all its interfaces).
pub(crate) fn collect_used_generics_from_class(class: &ClassMeta) -> Vec<String> {
    let all_methods: Vec<&MethodMeta> = class.all_interfaces()
        .flat_map(|iface| &iface.methods)
        .collect();
    // Reuse the same visitor logic as collect_used_generics_from_methods
    collect_used_generics_from_methods_inner(&all_methods)
}

// ======================================================================
// Import collection helpers
// ======================================================================

/// Recursively collect named type references from a TypeMeta tree.
/// `self_name` is excluded from results (the type being generated).
/// `include_self_interfaces` controls whether Interface types with the same name
/// as self_name are included (needed for class imports, not for interface imports).
fn visit_type_for_imports(
    typ: &TypeMeta,
    self_name: &str,
    include_self_interfaces: bool,
    imports: &mut HashSet<TypeRef>,
) {
    match typ {
        TypeMeta::RuntimeClass { namespace, name, .. } if name != self_name => {
            imports.insert(TypeRef { namespace: namespace.clone(), name: name.clone(), kind: TypeKind::Class });
        }
        TypeMeta::Interface { namespace, name, .. } => {
            if name != self_name || include_self_interfaces {
                imports.insert(TypeRef { namespace: namespace.clone(), name: name.clone(), kind: TypeKind::Interface });
            }
        }
        TypeMeta::Enum { namespace, name, .. } => {
            imports.insert(TypeRef { namespace: namespace.clone(), name: name.clone(), kind: TypeKind::Enum });
        }
        TypeMeta::AsyncOperation(inner) | TypeMeta::AsyncActionWithProgress(inner) => {
            visit_type_for_imports(inner, self_name, include_self_interfaces, imports);
        }
        TypeMeta::AsyncOperationWithProgress(result, progress) => {
            visit_type_for_imports(result, self_name, include_self_interfaces, imports);
            visit_type_for_imports(progress, self_name, include_self_interfaces, imports);
        }
        TypeMeta::Struct { fields, .. } => {
            for f in fields { visit_type_for_imports(&f.typ, self_name, include_self_interfaces, imports); }
        }
        TypeMeta::Array(inner) => visit_type_for_imports(inner, self_name, include_self_interfaces, imports),
        TypeMeta::Parameterized { args, .. } => {
            for arg in args { visit_type_for_imports(arg, self_name, include_self_interfaces, imports); }
        }
        _ => {}
    }
}

/// Collect type imports from methods using the unified visitor.
fn collect_methods_type_imports(methods: &[MethodMeta], self_name: &str, include_self_interfaces: bool, imports: &mut HashSet<TypeRef>) {
    for m in methods {
        for p in &m.params { visit_type_for_imports(&p.typ, self_name, include_self_interfaces, imports); }
        if let Some(ref rt) = m.return_type { visit_type_for_imports(rt, self_name, include_self_interfaces, imports); }
    }
}

/// Collect type references from an interface for import generation.
pub(crate) fn collect_iface_type_imports(iface: &InterfaceMeta) -> HashSet<TypeRef> {
    let mut imports = HashSet::new();
    collect_methods_type_imports(&iface.methods, &iface.name, false, &mut imports);
    imports
}

/// Collect type references from a class for import generation.
pub(crate) fn collect_type_imports(class: &ClassMeta) -> HashSet<TypeRef> {
    let mut imports = HashSet::new();
    for iface in class.all_interfaces() {
        collect_methods_type_imports(&iface.methods, &class.name, true, &mut imports);
    }
    imports
}

// ======================================================================
// Parameter and naming helpers
// ======================================================================

pub(crate) fn get_in_params(method: &MethodMeta) -> Vec<&crate::meta::ParamMeta> {
    // Include OutFill params as "in" — FillArray requires caller to provide the buffer
    method.params.iter().filter(|p| p.direction == ParamDirection::In || p.direction == ParamDirection::OutFill).collect()
}

pub(crate) fn to_camel_case(s: &str) -> String {
    if s.is_empty() {
        return String::new();
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap().to_lowercase().to_string();
    let result = format!("{}{}", first, chars.collect::<String>());
    // Avoid JS reserved words / strict-mode restricted identifiers
    if is_js_reserved(&result) {
        format!("{}_", result)
    } else {
        result
    }
}

fn is_js_reserved(s: &str) -> bool {
    matches!(s,
        // Keywords & strict-mode restricted identifiers
        "arguments" | "eval" | "break" | "case" | "catch" | "class" | "const"
        | "continue" | "debugger" | "default" | "delete" | "do" | "else"
        | "enum" | "export" | "extends" | "false" | "finally" | "for"
        | "function" | "if" | "import" | "in" | "instanceof" | "let"
        | "new" | "null" | "return" | "super" | "switch" | "this"
        | "throw" | "true" | "try" | "typeof" | "undefined" | "var"
        | "void" | "while" | "with" | "yield"
        // Strict-mode future reserved words
        | "implements" | "interface" | "package" | "private" | "protected"
        | "public" | "static"
    )
}

pub(crate) fn capitalize(s: &str) -> String {
    if s.is_empty() { return String::new(); }
    let mut chars = s.chars();
    let first = chars.next().unwrap().to_uppercase().to_string();
    format!("{}{}", first, chars.collect::<String>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::meta::{MethodMeta, ParamMeta, ParamDirection};

    #[test]
    fn to_camel_case_basic() {
        assert_eq!(to_camel_case("GetValue"), "getValue");
        assert_eq!(to_camel_case("X"), "x");
        assert_eq!(to_camel_case("already"), "already");
        assert_eq!(to_camel_case(""), "");
    }

    #[test]
    fn capitalize_basic() {
        assert_eq!(capitalize("hello"), "Hello");
        assert_eq!(capitalize("H"), "H");
        assert_eq!(capitalize(""), "");
        assert_eq!(capitalize("already"), "Already");
    }

    #[test]
    fn ts_struct_field_type_mappings() {
        assert_eq!(ts_struct_field_type(&TypeMeta::Bool), "boolean");
        assert_eq!(ts_struct_field_type(&TypeMeta::String), "string");
        assert_eq!(ts_struct_field_type(&TypeMeta::Guid), "string");
        assert_eq!(ts_struct_field_type(&TypeMeta::I32), "number");
        assert_eq!(ts_struct_field_type(&TypeMeta::F64), "number");
        assert_eq!(ts_struct_field_type(&TypeMeta::Struct {
            namespace: "N".into(), name: "MyStruct".into(), fields: vec![],
        }), "MyStruct");
        assert_eq!(ts_struct_field_type(&TypeMeta::Struct {
            namespace: "N".into(), name: "HResult".into(), fields: vec![],
        }), "number");
    }

    #[test]
    fn struct_field_getter_expressions() {
        assert_eq!(struct_field_getter(&TypeMeta::Bool, 0), "s.getU8(0) !== 0");
        assert_eq!(struct_field_getter(&TypeMeta::I32, 2), "s.getI32(2)");
        assert_eq!(struct_field_getter(&TypeMeta::String, 1), "s.getHstring(1)");
        assert_eq!(struct_field_getter(&TypeMeta::F64, 3), "s.getF64(3)");
    }

    #[test]
    fn struct_field_setter_expressions() {
        assert_eq!(struct_field_setter(&TypeMeta::Bool, 0, "v"), "s.setU8(0, v ? 1 : 0)");
        assert_eq!(struct_field_setter(&TypeMeta::I32, 1, "x"), "s.setI32(1, x)");
        assert_eq!(struct_field_setter(&TypeMeta::String, 2, "s"), "s.setHstring(2, s)");
    }

    #[test]
    fn ts_dynwinrt_type_primitives() {
        assert_eq!(ts_dynwinrt_type(&TypeMeta::Bool), "DynWinRtType.boolType()");
        assert_eq!(ts_dynwinrt_type(&TypeMeta::I32), "DynWinRtType.i32()");
        assert_eq!(ts_dynwinrt_type(&TypeMeta::String), "DynWinRtType.hstring()");
        assert_eq!(ts_dynwinrt_type(&TypeMeta::Guid), "DynWinRtType.guidType()");
        assert_eq!(ts_dynwinrt_type(&TypeMeta::F64), "DynWinRtType.f64()");
        assert_eq!(ts_dynwinrt_type(&TypeMeta::Object), "DynWinRtType.object()");
    }

    #[test]
    fn ts_dynwinrt_type_async() {
        assert_eq!(ts_dynwinrt_type(&TypeMeta::AsyncAction), "DynWinRtType.iAsyncAction()");
        assert_eq!(
            ts_dynwinrt_type(&TypeMeta::AsyncOperation(Box::new(TypeMeta::String))),
            "DynWinRtType.iAsyncOperation(DynWinRtType.hstring())"
        );
    }

    #[test]
    fn ts_dynwinrt_type_array() {
        assert_eq!(
            ts_dynwinrt_type(&TypeMeta::Array(Box::new(TypeMeta::I32))),
            "DynWinRtType.arrayType(DynWinRtType.i32())"
        );
    }

    #[test]
    fn ts_dynwinrt_type_struct() {
        let s = TypeMeta::Struct {
            namespace: "N".into(),
            name: "Rect".into(),
            fields: vec![
                crate::types::FieldMeta { name: "X".into(), typ: TypeMeta::F32 },
                crate::types::FieldMeta { name: "Y".into(), typ: TypeMeta::F32 },
            ],
        };
        assert_eq!(
            ts_dynwinrt_type(&s),
            "DynWinRtType.structType('N.Rect', [DynWinRtType.f32(), DynWinRtType.f32()])"
        );
    }

    #[test]
    fn build_method_sig_empty() {
        let m = MethodMeta {
            name: "DoSomething".into(),
            vtable_index: 6,
            params: vec![],
            return_type: None,
            is_property_getter: false,
            is_property_setter: false,
            is_event_add: false,
            is_event_remove: false,
        };
        assert_eq!(build_method_sig(&m), "new DynWinRtMethodSig()");
    }

    #[test]
    fn build_method_sig_with_params_and_return() {
        let m = MethodMeta {
            name: "GetValue".into(),
            vtable_index: 7,
            params: vec![
                ParamMeta { name: "key".into(), typ: TypeMeta::String, direction: ParamDirection::In },
            ],
            return_type: Some(TypeMeta::I32),
            is_property_getter: false,
            is_property_setter: false,
            is_event_add: false,
            is_event_remove: false,
        };
        let sig = build_method_sig(&m);
        assert!(sig.contains(".addIn(DynWinRtType.hstring())"));
        assert!(sig.contains(".addOut(DynWinRtType.i32())"));
    }

    #[test]
    fn wrap_arg_types() {
        assert_eq!(wrap_arg("s", &TypeMeta::String), "DynWinRtValue.hstring(s)");
        assert_eq!(wrap_arg("b", &TypeMeta::Bool), "DynWinRtValue.boolValue(b)");
        assert_eq!(wrap_arg("n", &TypeMeta::I32), "DynWinRtValue.i32(n)");
        assert_eq!(wrap_arg("n", &TypeMeta::I64), "DynWinRtValue.i64(n)");
        assert_eq!(wrap_arg("f", &TypeMeta::F64), "DynWinRtValue.f64(f)");
    }

    #[test]
    fn convert_return_basic() {
        let known = HashSet::new();
        let deferred = HashSet::new();
        assert_eq!(convert_return("r", Some(&TypeMeta::String), false, &known, &deferred), "r.toString()");
        assert_eq!(convert_return("r", Some(&TypeMeta::I32), false, &known, &deferred), "r.toNumber()");
        assert_eq!(convert_return("r", Some(&TypeMeta::Bool), false, &known, &deferred), "r.toBool()");
        assert_eq!(convert_return("r", None, false, &known, &deferred), "r");
    }

    #[test]
    fn convert_return_with_known_class() {
        let mut known = HashSet::new();
        known.insert("Uri".to_string());
        let deferred = HashSet::new();
        let rt = TypeMeta::RuntimeClass {
            namespace: "Windows.Foundation".into(), name: "Uri".into(), default_iid: "abc".into(),
        };
        assert_eq!(convert_return("r", Some(&rt), false, &known, &deferred), "new Uri(r)");
    }

    #[test]
    fn get_in_params_filters_correctly() {
        let m = MethodMeta {
            name: "Test".into(),
            vtable_index: 6,
            params: vec![
                ParamMeta { name: "a".into(), typ: TypeMeta::I32, direction: ParamDirection::In },
                ParamMeta { name: "b".into(), typ: TypeMeta::I32, direction: ParamDirection::Out },
                ParamMeta { name: "c".into(), typ: TypeMeta::Array(Box::new(TypeMeta::U8)), direction: ParamDirection::OutFill },
            ],
            return_type: None,
            is_property_getter: false,
            is_property_setter: false,
            is_event_add: false,
            is_event_remove: false,
        };
        let in_params = get_in_params(&m);
        assert_eq!(in_params.len(), 2); // In + OutFill
        assert_eq!(in_params[0].name, "a");
        assert_eq!(in_params[1].name, "c");
    }

    #[test]
    fn collect_type_imports_skips_self() {
        let class = crate::meta::ClassMeta {
            name: "MyClass".into(),
            namespace: "N".into(),
            full_name: "N.MyClass".into(),
            default_interface: Some(crate::meta::InterfaceMeta {
                name: "IMyClass".into(),
                namespace: "N".into(),
                iid: "abc".into(),
                methods: vec![MethodMeta {
                    name: "GetSelf".into(),
                    vtable_index: 6,
                    params: vec![],
                    return_type: Some(TypeMeta::RuntimeClass {
                        namespace: "N".into(), name: "MyClass".into(), default_iid: "def".into(),
                    }),
                    is_property_getter: false,
                    is_property_setter: false,
                    is_event_add: false,
                    is_event_remove: false,
                }],
                generic_piid: None,
                generic_args: vec![],
            }),
            required_interfaces: vec![],
            factory_interfaces: vec![],
            static_interfaces: vec![],
            has_default_constructor: false,
        };
        let imports = collect_type_imports(&class);
        // Should not include MyClass itself
        assert!(!imports.iter().any(|r| r.name == "MyClass"));
    }
}
