#include "support.h"
#include "v8/include/v8.h"

using namespace support;

extern "C" {
  v8::Isolate *v8__PropertyCallbackInfo__GetIsolate(v8::PropertyCallbackInfo<v8::Value> *self) {
    return self->GetIsolate();
  }
  
  v8::Object *v8__PropertyCallbackInfo__This(v8::PropertyCallbackInfo<v8::Value> *self) {
    return local_to_ptr(self->This());
  }

  v8::ReturnValue<v8::Value> *v8__PropertyCallbackInfo__GetReturnValue(v8::PropertyCallbackInfo<v8::Value> *self) {
    v8::ReturnValue<v8::Value> *rv = new v8::ReturnValue<v8::Value>(self->GetReturnValue());
    return rv;
  }
}
