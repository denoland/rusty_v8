
#include <cstdint>
#include <iostream>
#include <new>
#include <type_traits>
#include <utility>

namespace v8 {
class Channel {
 public:
  Channel() {}
  virtual ~Channel() {}

  virtual void method1(int32_t arg) {
    std::cout << "default v8::Channel::method1(" << arg << ") called"
              << std::endl;
  }

  virtual int32_t method2() const = 0;
};
}  // namespace v8

extern "C" {
void v8_inspector__Channel__EXTENDER__method1(v8::Channel& self, int32_t arg);
int32_t v8_inspector__Channel__EXTENDER__method2(const v8::Channel& self);
}

namespace extender {
template <class T>
using uninit_t = typename std::aligned_storage<sizeof(T), alignof(T)>::type;

namespace v8 {
struct Channel : public ::v8::Channel {
  using ::v8::Channel::Channel;

  void method1(int32_t arg) override {
    v8_inspector__Channel__EXTENDER__method1(*this, arg);
  }

  int32_t method2() const override {
    return v8_inspector__Channel__EXTENDER__method2(*this);
  }
};
}  // namespace v8
}  // namespace extender

extern "C" {
void v8_inspector__Channel__DTOR(v8::Channel& self) {
  self.~Channel();
}
void v8_inspector__Channel__method1(v8::Channel& self, int32_t arg) {
  self.method1(arg);
}
void v8_inspector__Channel__Channel__method1(v8::Channel& self, int32_t arg) {
  self.::v8::Channel::method1(arg);
}
int32_t v8_inspector__Channel__method2(const v8::Channel& self) {
  return self.method2();
}
void v8_inspector__Channel__EXTENDER__CTOR(
    extender::uninit_t<extender::v8::Channel>& buf) {
  new (std::launder(&buf)) extender::v8::Channel();
}
}  // extern "C"