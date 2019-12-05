#include "support.h"
#include "v8/include/v8.h"

using namespace support;

extern "C" {
v8::Primitive *v8__Null(v8::Isolate *isolate) {
  return local_to_ptr(v8::Null(isolate));
}

v8::Primitive *v8__Undefined(v8::Isolate *isolate) {
  return local_to_ptr(v8::Undefined(isolate));
}

v8::Boolean *v8__True(v8::Isolate *isolate) {
  return local_to_ptr(v8::True(isolate));
}

v8::Boolean *v8__False(v8::Isolate *isolate) {
  return local_to_ptr(v8::False(isolate));
}
}
