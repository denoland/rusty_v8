#include "third_party/v8/include/v8-inspector.h"
#include "../support.h"

using namespace v8_inspector;
using namespace support;

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

struct v8_inspector__V8Inspector__Channel__BASE : public V8Inspector::Channel {
  using V8Inspector::Channel::Channel;

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

extern "C" {
void v8_inspector__V8Inspector__Channel__BASE__CONSTRUCT(
    uninit_t<v8_inspector__V8Inspector__Channel__BASE>& buf) {
  construct_in_place<v8_inspector__V8Inspector__Channel__BASE>(buf);
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