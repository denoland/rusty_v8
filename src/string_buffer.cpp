#include "third_party/v8/include/v8-inspector.h"
#include "support.h"

using namespace v8_inspector;

extern "C" {
void v8_inspector__StringBuffer__DELETE(StringBuffer& self) {
  delete &self;
}

const StringView* v8_inspector__StringBuffer__string(StringBuffer& self) {
  return nullptr; // TODO(ry) self.string();
}

StringBuffer* v8_inspector__StringBuffer__create(const StringView& source) {
  return StringBuffer::create(source).release();
}
}