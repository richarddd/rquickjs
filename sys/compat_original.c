/* Compatibility shim for the original (Fabrice Bellard) QuickJS.
 *
 * quickjs-ng exports a number of helpers as real (non-inline) functions that
 * the original QuickJS either provides only as `static inline`, or does not
 * provide at all but which can be emulated from the public API.
 *
 * bindgen does not emit bindings for `static inline` functions, so this file
 * re-exports/emulates the helpers under stable names so rquickjs-core can call
 * them uniformly across flavors.
 *
 * Compiled only for the `quickjs-original` flavor.
 *
 * NOTE: A handful of quickjs-ng APIs have no equivalent in the original and
 * cannot be emulated here (Proxy target/handler introspection, promise hooks).
 * Those are `#[cfg]`-gated out of rquickjs-core for the `quickjs-original`
 * flavor instead.
 */
#include "quickjs.h"
#include <string.h>

/* ---- static-inline helpers re-exported as real symbols ---- */

JSValue rquickjs_compat_JS_GetProperty(JSContext *ctx, JSValueConst this_obj, JSAtom prop) {
    return JS_GetProperty(ctx, this_obj, prop);
}

int rquickjs_compat_JS_SetProperty(JSContext *ctx, JSValueConst this_obj, JSAtom prop, JSValue val) {
    return JS_SetProperty(ctx, this_obj, prop, val);
}

void rquickjs_compat_JS_FreeValue(JSContext *ctx, JSValue v) {
    JS_FreeValue(ctx, v);
}

void rquickjs_compat_JS_FreeValueRT(JSRuntime *rt, JSValue v) {
    JS_FreeValueRT(rt, v);
}

JSValue rquickjs_compat_JS_DupValue(JSContext *ctx, JSValueConst v) {
    return JS_DupValue(ctx, v);
}

JSValue rquickjs_compat_JS_DupValueRT(JSRuntime *rt, JSValueConst v) {
    return JS_DupValueRT(rt, v);
}

/* ---- emulated helpers ---- */

/* `Function.prototype`, fetched via the global object. */
JSValue rquickjs_compat_JS_GetFunctionProto(JSContext *ctx) {
    JSValue global = JS_GetGlobalObject(ctx);
    if (JS_IsException(global))
        return global;
    JSValue func_ctor = JS_GetPropertyStr(ctx, global, "Function");
    JS_FreeValue(ctx, global);
    if (JS_IsException(func_ctor))
        return func_ctor;
    JSValue proto = JS_GetPropertyStr(ctx, func_ctor, "prototype");
    JS_FreeValue(ctx, func_ctor);
    return proto;
}

/* `Symbol(description)` / `Symbol.for(description)`. */
JSValue rquickjs_compat_JS_NewSymbol(JSContext *ctx, const char *description, JS_BOOL is_global) {
    JSValue global = JS_GetGlobalObject(ctx);
    if (JS_IsException(global))
        return global;
    JSValue symbol_ctor = JS_GetPropertyStr(ctx, global, "Symbol");
    JS_FreeValue(ctx, global);
    if (JS_IsException(symbol_ctor))
        return symbol_ctor;

    JSValue arg = description ? JS_NewString(ctx, description) : JS_UNDEFINED;
    JSValue result;
    if (is_global) {
        JSValue for_fn = JS_GetPropertyStr(ctx, symbol_ctor, "for");
        if (JS_IsException(for_fn)) {
            JS_FreeValue(ctx, arg);
            JS_FreeValue(ctx, symbol_ctor);
            return for_fn;
        }
        result = JS_Call(ctx, for_fn, symbol_ctor, 1, (JSValueConst *)&arg);
        JS_FreeValue(ctx, for_fn);
    } else {
        int argc = description ? 1 : 0;
        result = JS_Call(ctx, symbol_ctor, JS_UNDEFINED, argc, (JSValueConst *)&arg);
    }
    JS_FreeValue(ctx, arg);
    JS_FreeValue(ctx, symbol_ctor);
    return result;
}

/* Create a Proxy via the global `Proxy` constructor. */
JSValue rquickjs_compat_JS_NewProxy(JSContext *ctx, JSValueConst target, JSValueConst handler) {
    JSValue global = JS_GetGlobalObject(ctx);
    if (JS_IsException(global))
        return global;
    JSValue proxy_ctor = JS_GetPropertyStr(ctx, global, "Proxy");
    JS_FreeValue(ctx, global);
    if (JS_IsException(proxy_ctor))
        return proxy_ctor;
    JSValueConst args[2] = { target, handler };
    JSValue result = JS_CallConstructor(ctx, proxy_ctor, 2, args);
    JS_FreeValue(ctx, proxy_ctor);
    return result;
}

/* ---- Proxy introspection ----
 *
 * The original QuickJS keeps Proxy internals private (no JS_GetProxyTarget /
 * JS_GetProxyHandler / JS_IsProxy in quickjs.h), but the target and handler are
 * stored in the object's opaque data as a `JSProxyData`, reachable through the
 * public `JS_GetOpaque(obj, JS_CLASS_PROXY)`.
 *
 * `JS_CLASS_PROXY` is an internal, build-stable enum value that is not exported.
 * Rather than hardcode it, we discover it at runtime: construct a throwaway
 * Proxy and read its class id via the public `JS_GetClassID`. The result is
 * cached per runtime.
 *
 * This mirrors the layout of `JSProxyData` in quickjs.c. It is validated at
 * runtime (see below) so a layout change in a future QuickJS would be caught
 * rather than silently returning garbage.
 */
typedef struct RQuickJSProxyData {
    JSValue target;
    JSValue handler;
    uint8_t is_func;
    uint8_t is_revoked;
} RQuickJSProxyData;

/* Discovered proxy support state, resolved once per process.
 *
 * A single process only ever links one QuickJS build and built-in class ids are
 * stable for a given build, so a static cache is safe. */
static JSClassID rquickjs_proxy_class_id = 0; /* 0 = not yet resolved / unavailable */
static int rquickjs_proxy_layout_ok = 0;      /* 1 once the JSProxyData layout is validated */
static int rquickjs_proxy_resolved = 0;       /* 1 once discovery has run */

/* Discover the JS_CLASS_PROXY class id and validate that our `RQuickJSProxyData`
 * mirror matches the engine's actual layout, by constructing a proxy with a
 * distinctive target/handler and reading them back through the opaque pointer.
 *
 * If the layout does not round-trip (e.g. a future QuickJS reordered the
 * struct), we leave `rquickjs_proxy_layout_ok == 0` so the accessors error out
 * cleanly instead of returning garbage. */
static void rquickjs_compat_resolve_proxy(JSContext *ctx) {
    if (rquickjs_proxy_resolved)
        return;
    rquickjs_proxy_resolved = 1;

    JSValue target = JS_NewObject(ctx);
    JSValue handler = JS_NewObject(ctx);
    if (JS_IsException(target) || JS_IsException(handler)) {
        JS_FreeValue(ctx, target);
        JS_FreeValue(ctx, handler);
        return;
    }
    /* Tag target and handler with unique marker properties so we can confirm we
     * read the right fields in the right order. */
    JS_SetPropertyStr(ctx, target, "__rq_marker", JS_NewInt32(ctx, 0x7a11e5));
    JS_SetPropertyStr(ctx, handler, "__rq_marker", JS_NewInt32(ctx, 0x0dd));

    JSValue proxy = rquickjs_compat_JS_NewProxy(ctx, target, handler);
    if (JS_IsException(proxy)) {
        JS_FreeValue(ctx, JS_GetException(ctx));
        JS_FreeValue(ctx, target);
        JS_FreeValue(ctx, handler);
        return;
    }

    JSClassID id = JS_GetClassID(proxy);
    int ok = 0;
    if (id != 0) {
        RQuickJSProxyData *s = JS_GetOpaque(proxy, id);
        if (s) {
            JSValue t_marker = JS_GetPropertyStr(ctx, s->target, "__rq_marker");
            JSValue h_marker = JS_GetPropertyStr(ctx, s->handler, "__rq_marker");
            int32_t tv = -1, hv = -1;
            JS_ToInt32(ctx, &tv, t_marker);
            JS_ToInt32(ctx, &hv, h_marker);
            JS_FreeValue(ctx, t_marker);
            JS_FreeValue(ctx, h_marker);
            ok = (tv == 0x7a11e5) && (hv == 0x0dd);
        }
    }

    JS_FreeValue(ctx, proxy);
    JS_FreeValue(ctx, target);
    JS_FreeValue(ctx, handler);

    rquickjs_proxy_class_id = id;
    rquickjs_proxy_layout_ok = ok;
}

JS_BOOL rquickjs_compat_JS_IsProxy(JSContext *ctx, JSValueConst val) {
    rquickjs_compat_resolve_proxy(ctx);
    /* Class-id detection does not depend on the struct layout, so it is safe
     * even if layout validation failed. */
    if (rquickjs_proxy_class_id == 0)
        return 0;
    return JS_GetClassID(val) == rquickjs_proxy_class_id;
}

JSValue rquickjs_compat_JS_GetProxyTarget(JSContext *ctx, JSValueConst proxy) {
    rquickjs_compat_resolve_proxy(ctx);
    if (!rquickjs_proxy_layout_ok)
        return JS_ThrowTypeError(
            ctx, "Proxy target introspection is unavailable in this QuickJS build");
    /* JS_GetOpaque returns NULL if the class id does not match. */
    RQuickJSProxyData *s = JS_GetOpaque(proxy, rquickjs_proxy_class_id);
    if (!s)
        return JS_ThrowTypeError(ctx, "not a Proxy");
    return JS_DupValue(ctx, s->target);
}

JSValue rquickjs_compat_JS_GetProxyHandler(JSContext *ctx, JSValueConst proxy) {
    rquickjs_compat_resolve_proxy(ctx);
    if (!rquickjs_proxy_layout_ok)
        return JS_ThrowTypeError(
            ctx, "Proxy handler introspection is unavailable in this QuickJS build");
    RQuickJSProxyData *s = JS_GetOpaque(proxy, rquickjs_proxy_class_id);
    if (!s)
        return JS_ThrowTypeError(ctx, "not a Proxy");
    return JS_DupValue(ctx, s->handler);
}

/* Determine the typed-array element type by inspecting the constructor name.
 * Mirrors quickjs-ng's `JS_GetTypedArrayType` return values; returns -1 if the
 * value is not a typed array. */
int rquickjs_compat_JS_GetTypedArrayType(JSContext *ctx, JSValueConst obj) {
    JSValue ctor = JS_GetPropertyStr(ctx, obj, "constructor");
    if (JS_IsException(ctor)) {
        JS_FreeValue(ctx, JS_GetException(ctx));
        return -1;
    }
    JSValue name_val = JS_GetPropertyStr(ctx, ctor, "name");
    JS_FreeValue(ctx, ctor);
    if (JS_IsException(name_val)) {
        JS_FreeValue(ctx, JS_GetException(ctx));
        return -1;
    }
    const char *name = JS_ToCString(ctx, name_val);
    JS_FreeValue(ctx, name_val);
    if (!name)
        return -1;

    int type = -1;
    if (!strcmp(name, "Uint8ClampedArray")) type = JS_TYPED_ARRAY_UINT8C;
    else if (!strcmp(name, "Int8Array")) type = JS_TYPED_ARRAY_INT8;
    else if (!strcmp(name, "Uint8Array")) type = JS_TYPED_ARRAY_UINT8;
    else if (!strcmp(name, "Int16Array")) type = JS_TYPED_ARRAY_INT16;
    else if (!strcmp(name, "Uint16Array")) type = JS_TYPED_ARRAY_UINT16;
    else if (!strcmp(name, "Int32Array")) type = JS_TYPED_ARRAY_INT32;
    else if (!strcmp(name, "Uint32Array")) type = JS_TYPED_ARRAY_UINT32;
    else if (!strcmp(name, "BigInt64Array")) type = JS_TYPED_ARRAY_BIG_INT64;
    else if (!strcmp(name, "BigUint64Array")) type = JS_TYPED_ARRAY_BIG_UINT64;
    else if (!strcmp(name, "Float16Array")) type = JS_TYPED_ARRAY_FLOAT16;
    else if (!strcmp(name, "Float32Array")) type = JS_TYPED_ARRAY_FLOAT32;
    else if (!strcmp(name, "Float64Array")) type = JS_TYPED_ARRAY_FLOAT64;

    JS_FreeCString(ctx, name);
    return type;
}

/* The original does not track "uncatchable" errors separately. */
JS_BOOL rquickjs_compat_JS_IsUncatchableError(JSContext *ctx, JSValueConst val) {
    (void)ctx;
    (void)val;
    return 0;
}

/* Immutable ArrayBuffers are a quickjs-ng feature; treat as unsupported no-op. */
int rquickjs_compat_JS_SetImmutableArrayBuffer(JSValueConst obj, JS_BOOL immutable) {
    (void)obj;
    (void)immutable;
    return -1;
}

/* quickjs-ng runtime dump flags / performance hooks have no original
 * equivalent; provide no-op stubs so runtime setup can proceed uniformly. */
void rquickjs_compat_JS_SetDumpFlags(JSRuntime *rt, uint64_t flags) {
    (void)rt;
    (void)flags;
}

int rquickjs_compat_JS_AddPerformance(JSContext *ctx) {
    (void)ctx;
    return 0;
}

/* ---- signature-normalizing wrappers ----
 *
 * The original returns `int` (and sometimes takes a context) for several
 * predicates that quickjs-ng exposes as `bool`-returning (and context-less).
 * Re-export them with quickjs-ng-compatible signatures so rquickjs-core links
 * uniformly. bindgen is told to blocklist the original declarations. */

JS_BOOL rquickjs_compat_JS_IsArray(JSContext *ctx, JSValueConst val) {
    return JS_IsArray(ctx, val) != 0;
}

JS_BOOL rquickjs_compat_JS_IsError(JSContext *ctx, JSValueConst val) {
    return JS_IsError(ctx, val) != 0;
}

JS_BOOL rquickjs_compat_JS_IsFunction(JSContext *ctx, JSValueConst val) {
    return JS_IsFunction(ctx, val) != 0;
}

JS_BOOL rquickjs_compat_JS_IsConstructor(JSContext *ctx, JSValueConst val) {
    return JS_IsConstructor(ctx, val) != 0;
}

JSClassID rquickjs_compat_JS_NewClassID(JSRuntime *rt, JSClassID *pclass_id) {
    (void)rt;
    return JS_NewClassID(pclass_id);
}

JS_BOOL rquickjs_compat_JS_HasException(JSContext *ctx) {
    return JS_HasException(ctx) != 0;
}

JS_BOOL rquickjs_compat_JS_SetConstructorBit(JSContext *ctx, JSValueConst func_obj, JS_BOOL val) {
    return JS_SetConstructorBit(ctx, func_obj, val) != 0;
}
