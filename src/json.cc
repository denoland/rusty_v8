#include "support.h"
#include "v8/include/v8.h"

using namespace support;

extern "C" {
v8::Value *v8__JSON__Parse(v8::Local<v8::Context> context, v8::Local<v8::String> json_string) {
    return maybe_local_to_ptr(v8::JSON::Parse(context, json_string));
}

v8::String *v8__JSON__Stringify(v8::Local<v8::Context> context, v8::Local<v8::Value> json_object) {
    return maybe_local_to_ptr(v8::JSON::Stringify(context, json_object));
}
}