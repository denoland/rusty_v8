#include "../support.h"
#include "v8/include/libplatform/libplatform.h"
#include "v8/include/v8-platform.h"

#include <iostream>

using namespace v8;
using namespace v8::platform;
using namespace support;

extern "C" {
Platform* v8__platform__NewDefaultPlatform() {
  // TODO: support optional arguments.
  return NewDefaultPlatform().release();
}

void v8__Platform__DELETE(Platform& self) {
  delete &self;
}
}  // extern "C"
