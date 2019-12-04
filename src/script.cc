#include <cstdint>

#include "support.h"
#include "v8/include/v8.h"

using namespace v8;
using namespace support;

extern "C" {
Script* v8__Script__Compile(Context* context,
                            String* source,
                            ScriptOrigin* origin) {
  return maybe_local_to_ptr(
      Script::Compile(ptr_to_local(context), ptr_to_local(source), origin));
}

Value* v8__Script__Run(Script& script, Context* context) {
  return maybe_local_to_ptr(script.Run(ptr_to_local(context)));
}
}
