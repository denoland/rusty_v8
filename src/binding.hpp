#include <v8-cppgc.h>
#include <v8-message.h>

/**
 * Types defined here will be compiled with bindgen
 * and made available in `crate::binding` in rust.
 */

namespace {

class RustObj;

}

static size_t RUST_v8__ScriptOrigin_SIZE = sizeof(v8::ScriptOrigin);

static size_t RUST_cppgc__Member_SIZE = sizeof(cppgc::Member<RustObj>);
static size_t RUST_cppgc__WeakMember_SIZE = sizeof(cppgc::WeakMember<RustObj>);

static size_t RUST_v8__TracedReference_SIZE =
    sizeof(v8::TracedReference<v8::Data>);
