#include <cstdint>

#include "support.h"
#include "v8/include/v8.h"

using namespace v8;
using namespace support;

static_assert(sizeof(ScriptOrigin) == sizeof(size_t) * 7,
              "ScriptOrigin size mismatch");

extern "C" {
Script *v8__Script__Compile(Context *context, String *source,
                            ScriptOrigin *origin) {
  return maybe_local_to_ptr(
      Script::Compile(ptr_to_local(context), ptr_to_local(source), origin));
}

Value *v8__Script__Run(Script &script, Context *context) {
  return maybe_local_to_ptr(script.Run(ptr_to_local(context)));
}

void v8__ScriptOrigin__CONSTRUCT(uninit_t<ScriptOrigin> &buf,
                                 Value *resource_name,
                                 Integer *resource_line_offset,
                                 Integer *resource_column_offset,
                                 Boolean *resource_is_shared_cross_origin,
                                 Integer *script_id, Value *source_map_url,
                                 Boolean *resource_is_opaque, Boolean *is_wasm,
                                 Boolean *is_module) {
  construct_in_place<ScriptOrigin>(
      buf, ptr_to_local(resource_name), ptr_to_local(resource_line_offset),
      ptr_to_local(resource_column_offset),
      ptr_to_local(resource_is_shared_cross_origin), ptr_to_local(script_id),
      ptr_to_local(source_map_url), ptr_to_local(resource_is_opaque),
      ptr_to_local(is_wasm), ptr_to_local(is_module));
}
}
