#include "support.h"
#include "v8/include/v8.h"

using namespace support;

extern "C" {
v8::Function *v8__Function__New(v8::Local<v8::Context> context,
                                v8::FunctionCallback callback) {
  return maybe_local_to_ptr(v8::Function::New(context, callback));
}

v8::Value *v8__Function__Call(v8::Function *self,
                              v8::Local<v8::Context> context,
                              v8::Local<v8::Value> recv,
                              int argc,
                              v8::Local<v8::Value> argv[]) {
  return maybe_local_to_ptr(self->Call(context, recv, argc, argv));
}
}
