#include <cstdint>

#include "v8/include/v8.h"

using namespace v8;

extern "C" {
Number* v8__Number__New(Isolate* isolate, double value) {
  return *Number::New(isolate, value);
}

double v8__Number__Value(const Number& self) {
  return self.Value();
}

Integer* v8__Integer__New(Isolate* isolate, int32_t value) {
  return *Integer::New(isolate, value);
}

Integer* v8__Integer__NewFromUnsigned(Isolate* isolate, uint32_t value) {
  return *Integer::NewFromUnsigned(isolate, value);
}

int64_t v8__Integer__Value(const Integer& self) {
  return self.Value();
}
}
