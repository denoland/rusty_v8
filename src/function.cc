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
                              v8::Local<v8::Value> recv) {
  return maybe_local_to_ptr(self->Call(context, recv, 0, nullptr));
}

int v8__FunctionCallbackInfo__Length(v8::FunctionCallbackInfo<v8::Value> *self) {
    return self->Length();
}

v8::FunctionTemplate *v8__FunctionTemplate__New(v8::Isolate* isolate,
                                                v8::FunctionCallback callback = nullptr) {
    return local_to_ptr(v8::FunctionTemplate::New(isolate, callback));
}

v8::Function *v8__FunctionTemplate__GetFunction(v8::Local<v8::FunctionTemplate> self,
                                                v8::Local<v8::Context> context) {
    return maybe_local_to_ptr(self->GetFunction(context));
}
}
