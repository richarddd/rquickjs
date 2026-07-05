// Aliases mapping quickjs-ng-style helper names onto the original-flavor
// compatibility shim (see compat_original.c / compat_original.h).
//
// The original QuickJS provides some helpers only as `static inline`, and
// others not at all (but emulatable from the public API). The shim exposes them
// under `rquickjs_compat_*` names; here we alias them back to the names
// quickjs-ng uses so that rquickjs-core can call them uniformly.

#[inline]
pub unsafe fn JS_GetProperty(ctx: *mut JSContext, this_obj: JSValue, prop: JSAtom) -> JSValue {
    rquickjs_compat_JS_GetProperty(ctx, this_obj, prop)
}

#[inline]
pub unsafe fn JS_SetProperty(
    ctx: *mut JSContext,
    this_obj: JSValue,
    prop: JSAtom,
    val: JSValue,
) -> ::core::ffi::c_int {
    rquickjs_compat_JS_SetProperty(ctx, this_obj, prop, val)
}

#[inline]
pub unsafe fn JS_FreeValue(ctx: *mut JSContext, v: JSValue) {
    rquickjs_compat_JS_FreeValue(ctx, v)
}

#[inline]
pub unsafe fn JS_FreeValueRT(rt: *mut JSRuntime, v: JSValue) {
    rquickjs_compat_JS_FreeValueRT(rt, v)
}

#[inline]
pub unsafe fn JS_DupValue(ctx: *mut JSContext, v: JSValue) -> JSValue {
    rquickjs_compat_JS_DupValue(ctx, v)
}

#[inline]
pub unsafe fn JS_DupValueRT(rt: *mut JSRuntime, v: JSValue) -> JSValue {
    rquickjs_compat_JS_DupValueRT(rt, v)
}

#[inline]
pub unsafe fn JS_GetFunctionProto(ctx: *mut JSContext) -> JSValue {
    rquickjs_compat_JS_GetFunctionProto(ctx)
}

#[inline]
pub unsafe fn JS_NewSymbol(
    ctx: *mut JSContext,
    description: *const ::core::ffi::c_char,
    is_global: bool,
) -> JSValue {
    rquickjs_compat_JS_NewSymbol(ctx, description, is_global as _)
}

#[inline]
pub unsafe fn JS_NewProxy(ctx: *mut JSContext, target: JSValue, handler: JSValue) -> JSValue {
    rquickjs_compat_JS_NewProxy(ctx, target, handler)
}

#[inline]
pub unsafe fn JS_IsProxy(ctx: *mut JSContext, val: JSValue) -> bool {
    rquickjs_compat_JS_IsProxy(ctx, val) != 0
}

#[inline]
pub unsafe fn JS_GetProxyTarget(ctx: *mut JSContext, proxy: JSValue) -> JSValue {
    rquickjs_compat_JS_GetProxyTarget(ctx, proxy)
}

#[inline]
pub unsafe fn JS_GetProxyHandler(ctx: *mut JSContext, proxy: JSValue) -> JSValue {
    rquickjs_compat_JS_GetProxyHandler(ctx, proxy)
}

#[inline]
pub unsafe fn JS_SetDumpFlags(rt: *mut JSRuntime, flags: u64) {
    rquickjs_compat_JS_SetDumpFlags(rt, flags)
}

#[inline]
pub unsafe fn JS_AddPerformance(ctx: *mut JSContext) -> ::core::ffi::c_int {
    rquickjs_compat_JS_AddPerformance(ctx)
}

/// Emulated typed-array-type query for the original flavor.
///
/// Unlike quickjs-ng's `JS_GetTypedArrayType` (which takes only the value), the
/// original has no public class-id API, so the emulation requires a context.
#[inline]
pub unsafe fn JS_GetTypedArrayType(ctx: *mut JSContext, obj: JSValue) -> ::core::ffi::c_int {
    rquickjs_compat_JS_GetTypedArrayType(ctx, obj)
}

/// Emulated uncatchable-error query for the original flavor (always `false`).
#[inline]
pub unsafe fn JS_IsUncatchableError(ctx: *mut JSContext, val: JSValue) -> bool {
    rquickjs_compat_JS_IsUncatchableError(ctx, val) != 0
}

/// Emulated immutable-array-buffer setter for the original flavor
/// (unsupported; returns an error code).
#[inline]
pub unsafe fn JS_SetImmutableArrayBuffer(obj: JSValue, immutable: bool) -> ::core::ffi::c_int {
    rquickjs_compat_JS_SetImmutableArrayBuffer(obj, immutable as _)
}

// ---- signature-normalizing aliases ----
//
// The original returns `int` (and `JS_IsArray`/`JS_IsError` take a context)
// where quickjs-ng returns `bool`; `JS_NewClassID` takes an extra runtime
// argument. These aliases present the quickjs-ng-compatible surface.

#[inline]
pub unsafe fn JS_IsArray(ctx: *mut JSContext, val: JSValue) -> bool {
    rquickjs_compat_JS_IsArray(ctx, val) != 0
}

#[inline]
pub unsafe fn JS_IsError(ctx: *mut JSContext, val: JSValue) -> bool {
    rquickjs_compat_JS_IsError(ctx, val) != 0
}

#[inline]
pub unsafe fn JS_IsFunction(ctx: *mut JSContext, val: JSValue) -> bool {
    rquickjs_compat_JS_IsFunction(ctx, val) != 0
}

#[inline]
pub unsafe fn JS_IsConstructor(ctx: *mut JSContext, val: JSValue) -> bool {
    rquickjs_compat_JS_IsConstructor(ctx, val) != 0
}

#[inline]
pub unsafe fn JS_NewClassID(rt: *mut JSRuntime, pclass_id: *mut JSClassID) -> JSClassID {
    rquickjs_compat_JS_NewClassID(rt, pclass_id)
}

#[inline]
pub unsafe fn JS_HasException(ctx: *mut JSContext) -> bool {
    rquickjs_compat_JS_HasException(ctx) != 0
}

#[inline]
pub unsafe fn JS_SetConstructorBit(
    ctx: *mut JSContext,
    func_obj: JSValue,
    val: bool,
) -> bool {
    rquickjs_compat_JS_SetConstructorBit(ctx, func_obj, val as _) != 0
}

// The original QuickJS has no bytecode source/debug stripping flags; define
// them as no-op (0) so `Module::write` options compile uniformly.
pub const JS_WRITE_OBJ_STRIP_SOURCE: u32 = 0;
pub const JS_WRITE_OBJ_STRIP_DEBUG: u32 = 0;
