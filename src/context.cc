#include <cstdint>

#include "v8/include/v8.h"

using namespace v8;

extern "C" {
Context* v8__Context__New(Isolate* isolate) {
  // TODO: optional arguments.
  return *Context::New(isolate);
}

void v8__Context__Enter(Context& self) {
  self.Enter();
}

void v8__Context__Exit(Context& self) {
  self.Exit();
}

Isolate* v8__Context__GetIsolate(Context& self) {
  return self.GetIsolate();
}
}
