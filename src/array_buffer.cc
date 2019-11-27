#include "v8/include/v8.h"

using namespace v8;

extern "C" {
ArrayBuffer::Allocator* v8__ArrayBuffer__Allocator__NewDefaultAllocator() {
  return ArrayBuffer::Allocator::NewDefaultAllocator();
}

void v8__ArrayBuffer__Allocator__DELETE(ArrayBuffer::Allocator& self) {
  delete &self;
}
}  // extern "C"
