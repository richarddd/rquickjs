/* Declarations for the original-flavor compatibility shim (see
 * compat_original.c). Included by quickjs.bind.h only for the
 * `quickjs-original` flavor. */
#ifndef RQUICKJS_COMPAT_ORIGINAL_H
#define RQUICKJS_COMPAT_ORIGINAL_H

#include "quickjs.h"

/* static-inline helpers re-exported as real symbols */
JSValue rquickjs_compat_JS_GetProperty(JSContext *ctx, JSValueConst this_obj, JSAtom prop);
int rquickjs_compat_JS_SetProperty(JSContext *ctx, JSValueConst this_obj, JSAtom prop, JSValue val);
void rquickjs_compat_JS_FreeValue(JSContext *ctx, JSValue v);
void rquickjs_compat_JS_FreeValueRT(JSRuntime *rt, JSValue v);
JSValue rquickjs_compat_JS_DupValue(JSContext *ctx, JSValueConst v);
JSValue rquickjs_compat_JS_DupValueRT(JSRuntime *rt, JSValueConst v);

/* emulated helpers */
JSValue rquickjs_compat_JS_GetFunctionProto(JSContext *ctx);
JSValue rquickjs_compat_JS_NewSymbol(JSContext *ctx, const char *description, JS_BOOL is_global);
JSValue rquickjs_compat_JS_NewProxy(JSContext *ctx, JSValueConst target, JSValueConst handler);
JS_BOOL rquickjs_compat_JS_IsProxy(JSContext *ctx, JSValueConst val);
JSValue rquickjs_compat_JS_GetProxyTarget(JSContext *ctx, JSValueConst proxy);
JSValue rquickjs_compat_JS_GetProxyHandler(JSContext *ctx, JSValueConst proxy);
int rquickjs_compat_JS_GetTypedArrayType(JSContext *ctx, JSValueConst obj);
JS_BOOL rquickjs_compat_JS_IsUncatchableError(JSContext *ctx, JSValueConst val);
int rquickjs_compat_JS_SetImmutableArrayBuffer(JSValueConst obj, JS_BOOL immutable);
void rquickjs_compat_JS_SetDumpFlags(JSRuntime *rt, uint64_t flags);
int rquickjs_compat_JS_AddPerformance(JSContext *ctx);

/* signature-normalizing wrappers */
JS_BOOL rquickjs_compat_JS_IsArray(JSContext *ctx, JSValueConst val);
JS_BOOL rquickjs_compat_JS_IsError(JSContext *ctx, JSValueConst val);
JS_BOOL rquickjs_compat_JS_IsFunction(JSContext *ctx, JSValueConst val);
JS_BOOL rquickjs_compat_JS_IsConstructor(JSContext *ctx, JSValueConst val);
JSClassID rquickjs_compat_JS_NewClassID(JSRuntime *rt, JSClassID *pclass_id);
JS_BOOL rquickjs_compat_JS_HasException(JSContext *ctx);
JS_BOOL rquickjs_compat_JS_SetConstructorBit(JSContext *ctx, JSValueConst func_obj, JS_BOOL val);

#endif /* RQUICKJS_COMPAT_ORIGINAL_H */
