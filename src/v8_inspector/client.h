#include <memory>
#include <utility>

using namespace v8_inspector;

extern "C" {
void v8_inspector__V8InspectorClient__BASE__runMessageLoopOnPause(
    V8InspectorClient& self,
    int contextGroupId);
void v8_inspector__V8InspectorClient__BASE__quitMessageLoopOnPause(
    V8InspectorClient& self);
void v8_inspector__V8InspectorClient__BASE__runIfWaitingForDebugger(
    V8InspectorClient& self,
    int contextGroupId);

}  // extern "C"

namespace v8_inspector {
struct Client__BASE : public V8InspectorClient {
  using V8InspectorClient::V8InspectorClient;

  void runMessageLoopOnPause(int contextGroupId) override {
    v8_inspector__V8InspectorClient__BASE__runMessageLoopOnPause(
        *this, contextGroupId);
  }
  void quitMessageLoopOnPause() override {
    v8_inspector__V8InspectorClient__BASE__quitMessageLoopOnPause(*this);
  }
  void runIfWaitingForDebugger(int contextGroupId) override {
    v8_inspector__V8InspectorClient__BASE__runIfWaitingForDebugger(
        *this, contextGroupId);
  }
};
}  // namespace v8_inspector

extern "C" {
void v8_inspector__V8InspectorClient__BASE__CTOR(uninit_t<Client__BASE>& buf) {
  new (launder(&buf)) Client__BASE();
}
void v8_inspector__V8InspectorClient__DTOR(V8InspectorClient& self) {
  self.~V8InspectorClient();
}

void v8_inspector__V8InspectorClient__runMessageLoopOnPause(
    V8InspectorClient& self,
    int contextGroupId) {
  self.runMessageLoopOnPause(contextGroupId);
}
void v8_inspector__V8InspectorClient__quitMessageLoopOnPause(
    V8InspectorClient& self) {
  self.quitMessageLoopOnPause();
}
void v8_inspector__V8InspectorClient__runIfWaitingForDebugger(
    V8InspectorClient& self,
    int contextGroupId) {
  self.runIfWaitingForDebugger(contextGroupId);
}
}  // extern "C"