#include <cstdint>

#include "support.h"
#include "v8/include/v8.h"

using namespace v8;
using namespace support;

extern "C" {
String* v8__String__NewFromUtf8(Isolate* isolate,
                                const char* data,
                                NewStringType type,
                                int length) {
  return maybe_local_to_ptr(String::NewFromUtf8(isolate, data, type, length));
}

int v8__String__Utf8Length(const String& self, Isolate* isolate) {
  return self.Utf8Length(isolate);
}

int v8__String__WriteUtf8(const String& self,
                          Isolate* isolate,
                          char* buffer,
                          int length,
                          int* nchars_ref,
                          int options) {
  return self.WriteUtf8(isolate, buffer, length, nchars_ref, options);
}
}
