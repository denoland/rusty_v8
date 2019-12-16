#include "support.h"
#include "v8/include/v8.h"

using namespace support;

extern "C" {
v8::Object *v8__Object__New(v8::Isolate *isolate,
                            v8::Local<v8::Value> prototype_or_null,
                            v8::Local<v8::Name>* names,
                            v8::Local<v8::Value>* values,
                            size_t length) {
  return local_to_ptr(v8::Object::New(isolate, prototype_or_null, names, values, length));
}

v8::Isolate *v8__Object__GetIsolate(v8::Object& self) {
  return self.GetIsolate();
}
}
