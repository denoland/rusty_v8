#include <v8-message.h>

/**
 * Types defined here will be compiled with bindgen
 * and made available in `crate::binding` in rust.
 */

// TODO: In the immediate term, cppgc definitions will go here.
// In the future we should migrate over the rest of our SIZE definitions,
// and eventually entire structs and functions.

static size_t RUST_v8__ScriptOrigin_SIZE = sizeof(v8::ScriptOrigin);
