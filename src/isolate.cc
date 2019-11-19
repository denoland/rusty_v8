#include "v8/include/v8.h"

using namespace v8;

extern "C" {
Isolate* v8__Isolate__New() {
  Isolate::CreateParams params;
  return Isolate::New(params);
}

void v8__Isolate__Dispose(Isolate& isolate) {
  isolate.Dispose();
}
}