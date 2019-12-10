#include "support.h"
#include "v8/include/v8.h"

using namespace support;

extern "C" {
v8::Promise::Resolver *v8__Promise__Resolver__New(v8::Local<v8::Context> context) {
    return maybe_local_to_ptr(v8::Promise::Resolver::New(context));
}
}
