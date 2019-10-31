#include "third_party/v8/include/v8-platform.h"
#include "../support.h"

#include <iostream>

using namespace v8;
using namespace support;

extern "C" {
void v8__Task__BASE__DELETE(Task& self);
void v8__Task__BASE__Run(Task& self);
}  // extern "C"

struct v8__Task__BASE : public Task {
  using Task::Task;
  void operator delete(void* ptr) noexcept {
    v8__Task__BASE__DELETE(*reinterpret_cast<Task*>(ptr));
  }
  void Run() override { v8__Task__BASE__Run(*this); }
};

extern "C" {
void v8__Task__BASE__CONSTRUCT(uninit_t<v8__Task__BASE>& buf) {
  construct_in_place<v8__Task__BASE>(buf);
}
void v8__Task__DELETE(Task& self) {
  delete &self;
}
void v8__Task__Run(Task& self) {
  self.Run();
}
}  // extern "C"
