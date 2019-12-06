#include <cstdint>

#include "support.h"
#include "v8/include/v8.h"

using namespace v8;
using namespace support;

extern "C" {
bool v8__Value__IsUndefined(const Value &self) { return self.IsUndefined(); }

bool v8__Value__IsNull(const Value &self) { return self.IsNull(); }

bool v8__Value__IsNullOrUndefined(const Value &self) {
  return self.IsNullOrUndefined();
}
}
