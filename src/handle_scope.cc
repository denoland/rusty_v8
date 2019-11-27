#include <cassert>
#include <cstdint>

#include "support.h"
#include "v8/include/v8.h"

using namespace v8;
using namespace support;

static_assert(sizeof(HandleScope) == sizeof(size_t) * 3,
              "HandleScope size mismatch");

extern "C" {
void v8__HandleScope__CONSTRUCT(uninit_t<HandleScope>& buf, Isolate* isolate) {
  construct_in_place<HandleScope>(buf, isolate);
}

void v8__HandleScope__DESTRUCT(HandleScope& self) {
  self.~HandleScope();
}
}