#include <v8-cppgc.h>
#include <v8-message.h>
#include <v8-typed-array.h>

#include "support.h"

/**
 * Types defined here will be compiled with bindgen
 * and made available in `crate::binding` in rust.
 */

class RustObj;

static size_t v8__ScriptOrigin_SIZE = sizeof(v8::ScriptOrigin);

static size_t cppgc__Member_SIZE = sizeof(cppgc::Member<RustObj>);
static size_t cppgc__WeakMember_SIZE = sizeof(cppgc::WeakMember<RustObj>);

static size_t v8__TracedReference_SIZE = sizeof(v8::TracedReference<v8::Data>);

static size_t v8__String__ValueView_SIZE = sizeof(v8::String::ValueView);

static int v8__String__kMaxLength = v8::String::kMaxLength;

static size_t v8__TypedArray__kMaxByteLength = v8::TypedArray::kMaxByteLength;

#define TYPED_ARRAY_MAX_LENGTH(name) \
  static size_t v8__##name##__kMaxLength = v8::name::kMaxLength;
EACH_TYPED_ARRAY(TYPED_ARRAY_MAX_LENGTH)
#undef TYPED_ARRAY_MAX_LENGTH
