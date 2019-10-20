#include <memory>
#include <utility>

using namespace v8_inspector;

extern "C" {
void v8_inspector__V8Inspector__Channel__BASE__sendResponse(
    V8Inspector::Channel& self,
    int callId,
    StringBuffer* message);
void v8_inspector__V8Inspector__Channel__BASE__sendNotification(
    V8Inspector::Channel& self,
    StringBuffer* message);
void v8_inspector__V8Inspector__Channel__BASE__flushProtocolNotifications(
    V8Inspector::Channel& self);
}  // extern "C"

namespace v8_inspector {
struct V8Inspector__Channel__BASE : public V8Inspector::Channel {
  using V8Inspector::Channel::Channel;
  static_assert(sizeof(std::unique_ptr<StringBuffer>) == sizeof(StringBuffer*),
                "sizeof(T*) != sizeof(unique_ptr<T>)");

  void sendResponse(int callId,
                    std::unique_ptr<StringBuffer> message) override {
    v8_inspector__V8Inspector__Channel__BASE__sendResponse(*this, callId,
                                                           message.release());
  }
  void sendNotification(std::unique_ptr<StringBuffer> message) override {
    v8_inspector__V8Inspector__Channel__BASE__sendNotification(
        *this, message.release());
  }
  void flushProtocolNotifications() override {
    v8_inspector__V8Inspector__Channel__BASE__flushProtocolNotifications(*this);
  }
};
}  // namespace v8_inspector

extern "C" {
void v8_inspector__V8Inspector__Channel__BASE__CTOR(
    uninit_t<V8Inspector__Channel__BASE>& buf) {
  new (launder(&buf)) V8Inspector__Channel__BASE();
}
void v8_inspector__V8Inspector__Channel__DTOR(V8Inspector::Channel& self) {
  self.~Channel();
}

void v8_inspector__V8Inspector__Channel__sendResponse(
    V8Inspector::Channel& self,
    int callId,
    StringBuffer* message) {
  self.sendResponse(callId,
                    static_cast<std::unique_ptr<StringBuffer>>(message));
}
void v8_inspector__V8Inspector__Channel__sendNotification(
    V8Inspector::Channel& self,
    StringBuffer* message) {
  self.sendNotification(static_cast<std::unique_ptr<StringBuffer>>(message));
}
void v8_inspector__V8Inspector__Channel__flushProtocolNotifications(
    V8Inspector::Channel& self) {
  self.flushProtocolNotifications();
}
}  // extern "C"