#include "v8/include/v8.h"

using namespace v8;

extern "C" {
// This function consumes the Isolate::CreateParams object. The Isolate takes
// ownership of the ArrayBuffer::Allocator referenced by the params object.
Isolate* v8__Isolate__New(Isolate::CreateParams& params) {
  auto isolate = Isolate::New(params);
  delete &params;
  return isolate;
}

void v8__Isolate__Dispose(Isolate& isolate) {
  auto allocator = isolate.GetArrayBufferAllocator();
  isolate.Dispose();
  delete allocator;
}

void v8__Isolate__Enter(Isolate& isolate) {
  isolate.Enter();
}

void v8__Isolate__Exit(Isolate& isolate) {
  isolate.Exit();
}

void v8__Isolate__SetPromiseRejectCallback(Isolate& isolate,
                                           v8::PromiseRejectCallback callback) {
  isolate.SetPromiseRejectCallback(callback);
}

Isolate::CreateParams* v8__Isolate__CreateParams__NEW() {
  return new Isolate::CreateParams();
}

// This function is only called if the Isolate::CreateParams object is *not*
// consumed by Isolate::New().
void v8__Isolate__CreateParams__DELETE(Isolate::CreateParams& self) {
  delete self.array_buffer_allocator;
  delete &self;
}

// This function takes ownership of the ArrayBuffer::Allocator.
void v8__Isolate__CreateParams__SET__array_buffer_allocator(
    Isolate::CreateParams& self, ArrayBuffer::Allocator* value) {
  delete self.array_buffer_allocator;
  self.array_buffer_allocator = value;
}
}
