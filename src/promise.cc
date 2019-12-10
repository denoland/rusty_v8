#include "support.h"
#include "v8/include/v8.h"

using namespace support;

extern "C" {
v8::Promise::Resolver *v8__Promise__Resolver__New(v8::Local<v8::Context> context) {
  return maybe_local_to_ptr(v8::Promise::Resolver::New(context));
}

v8::Promise *v8__Promise__Resolver__GetPromise(v8::Promise::Resolver *self) {
  return local_to_ptr(self->GetPromise());
}

bool *v8__Promise__Resolver__Resolve(v8::Promise::Resolver *self,
                                     v8::Local<v8::Context> context,
                                     v8::Local<v8::Value> value) {
  return maybe_to_ptr(self->Resolve(context, value));
}

bool *v8__Promise__Resolver__Reject(v8::Promise::Resolver *self,
                                    v8::Local<v8::Context> context,
                                    v8::Local<v8::Value> value) {
  return maybe_to_ptr(self->Reject(context, value));
}

v8::Promise::PromiseState v8__Promise__State(v8::Promise *self) {
  return self->State();
}

bool v8__Promise__HasHandler(v8::Promise *self) {
  return self->HasHandler();
}

v8::Value *v8__Promise__Result(v8::Promise *self) {
  return local_to_ptr(self->Result());
}
}
