#include "support.h"
#include "v8/include/v8.h"

using namespace v8;
using namespace support;

extern "C" {
void v8__Locker__CONSTRUCT(uninit_t<Locker>& buf, Isolate* isolate) {
  construct_in_place<Locker>(buf, isolate);
}

void v8__Locker__DESTRUCT(Locker& self) {
  self.~Locker();
}
}