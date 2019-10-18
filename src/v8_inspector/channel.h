#include <memory>
#include <utility>

extern "C" {
using namespace v8_inspector;

void v8_inspector__Channel__EXTENDER__sendResponse(Channel& self,
                                                   int callId,
                                                   StringBuffer* message);
void v8_inspector__Channel__EXTENDER__sendNotification(Channel& self,
                                                       StringBuffer* message);
void v8_inspector__Channel__EXTENDER__flushProtocolNotifications(Channel& self);
}  // extern "C"

namespace v8_inspector {
struct Channel__EXTENDER : public Channel {
  using Channel::Channel;
  static_assert(sizeof(std::unique_ptr<StringBuffer>) == sizeof(StringBuffer*),
                "sizeof(T*) != sizeof(unique_ptr<T>)");

  void sendResponse(int callId,
                    std::unique_ptr<StringBuffer> message) override {
    v8_inspector__Channel__EXTENDER__sendResponse(*this, callId,
                                                  message.release());
  }
  void sendNotification(std::unique_ptr<StringBuffer> message) override {
    v8_inspector__Channel__EXTENDER__sendNotification(*this, message.release());
  }
  void flushProtocolNotifications() override {
    v8_inspector__Channel__EXTENDER__flushProtocolNotifications(*this);
  }
};
}  // namespace v8_inspector

extern "C" {
using namespace v8_inspector;

void v8_inspector__Channel__EXTENDER__CTOR(uninit_t<Channel__EXTENDER>& buf) {
  new (launder(&buf)) Channel__EXTENDER();
}
void v8_inspector__Channel__DTOR(Channel& self) {
  self.~Channel();
}

void v8_inspector__Channel__sendResponse(Channel& self,
                                         int callId,
                                         StringBuffer* message) {
  self.sendResponse(callId,
                    static_cast<std::unique_ptr<StringBuffer>>(message));
}
void v8_inspector__Channel__sendNotification(Channel& self,
                                             StringBuffer* message) {
  self.sendNotification(static_cast<std::unique_ptr<StringBuffer>>(message));
}
void v8_inspector__Channel__flushProtocolNotifications(Channel& self) {
  self.flushProtocolNotifications();
}
}  // extern "C"