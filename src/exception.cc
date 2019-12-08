#include "support.h"
#include "v8/include/v8.h"

using namespace support;

extern "C" {
v8::Value *v8__Exception__RangeError(v8::Local<v8::String> message) {
  return local_to_ptr(v8::Exception::RangeError(message));
}

v8::Value *v8__Exception__ReferenceError(v8::Local<v8::String> message) {
  return local_to_ptr(v8::Exception::ReferenceError(message));
}

v8::Value *v8__Exception__SyntaxError(v8::Local<v8::String> message) {
  return local_to_ptr(v8::Exception::SyntaxError(message));
}

v8::Value *v8__Exception__TypeError(v8::Local<v8::String> message) {
  return local_to_ptr(v8::Exception::TypeError(message));
}

v8::Value *v8__Exception__Error(v8::Local<v8::String> message) {
  return local_to_ptr(v8::Exception::Error(message));
}
}
