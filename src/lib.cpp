
#include <cstdint>
#include <iostream>
#include <memory>
#include <new>
#include <type_traits>
#include <utility>

namespace v8_inspector {
class StringView {
 public:
  StringView() : m_is8Bit(true), m_length(0), m_characters8(nullptr) {}

  StringView(const uint8_t* characters, size_t length)
      : m_is8Bit(true), m_length(length), m_characters8(characters) {}

  StringView(const uint16_t* characters, size_t length)
      : m_is8Bit(false), m_length(length), m_characters16(characters) {}

  bool is8Bit() const { return m_is8Bit; }
  size_t length() const { return m_length; }

  // TODO(dgozman): add DCHECK(m_is8Bit) to accessors once platform can be used
  // here.
  const uint8_t* characters8() const { return m_characters8; }
  const uint16_t* characters16() const { return m_characters16; }

 private:
  bool m_is8Bit;
  size_t m_length;
  union {
    const uint8_t* m_characters8;
    const uint16_t* m_characters16;
  };
};

class StringBuffer {
 public:
  virtual ~StringBuffer() = default;
  virtual const StringView& string() = 0;
  // This method copies contents.
  static std::unique_ptr<StringBuffer> create(const StringView&);
};

class Channel {
 public:
  Channel() {}
  virtual ~Channel() {}

  virtual void method1(int32_t arg) {
    std::cout << "default v8_inspector::Channel::method1(" << arg << ") called"
              << std::endl;
  }

  virtual int32_t method2() const = 0;
};
}  // namespace v8_inspector

#include "v8_inspector/string_buffer.h"

extern "C" {
void v8_inspector__Channel__EXTENDER__method1(v8_inspector::Channel& self,
                                              int32_t arg);
int32_t v8_inspector__Channel__EXTENDER__method2(
    const v8_inspector::Channel& self);
}

namespace extender {
template <class T>
using uninit_t = typename ::std::aligned_storage<sizeof(T), alignof(T)>::type;

namespace v8_inspector {
struct Channel : public ::v8_inspector::Channel {
  using ::v8_inspector::Channel::Channel;

  void method1(int32_t arg) override {
    v8_inspector__Channel__EXTENDER__method1(*this, arg);
  }

  int32_t method2() const override {
    return v8_inspector__Channel__EXTENDER__method2(*this);
  }
};
}  // namespace v8_inspector
}  // namespace extender

extern "C" {
void v8_inspector__Channel__DTOR(::v8_inspector::Channel& self) {
  self.~Channel();
}
void v8_inspector__Channel__method1(::v8_inspector::Channel& self,
                                    int32_t arg) {
  self.method1(arg);
}
void v8_inspector__Channel__Channel__method1(::v8_inspector::Channel& self,
                                             int32_t arg) {
  self.::v8_inspector::Channel::method1(arg);
}
int32_t v8_inspector__Channel__method2(const ::v8_inspector::Channel& self) {
  return self.method2();
}
void v8_inspector__Channel__EXTENDER__CTOR(
    ::extender::uninit_t<::extender::v8_inspector::Channel>& buf) {
  new (::std::launder(&buf))::extender::v8_inspector::Channel();
}
}  // extern "C"