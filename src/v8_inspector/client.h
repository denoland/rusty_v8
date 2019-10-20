#include <memory>
#include <utility>

using namespace v8_inspector;
using Client = V8InspectorClient;

extern "C" {
void v8_inspector__Client__EXTENDER__runMessageLoopOnPause(Client& self,
                                                           int contextGroupId);
void v8_inspector__Client__EXTENDER__quitMessageLoopOnPause(Client& self);
void v8_inspector__Client__EXTENDER__runIfWaitingForDebugger(
    Client& self,
    int contextGroupId);

}  // extern "C"

namespace v8_inspector {
struct Client__EXTENDER : public Client {
  using Client::Client;

  void runMessageLoopOnPause(int contextGroupId) override {
    v8_inspector__Client__EXTENDER__runMessageLoopOnPause(*this,
                                                          contextGroupId);
  }
  void quitMessageLoopOnPause() override {
    v8_inspector__Client__EXTENDER__quitMessageLoopOnPause(*this);
  }
  void runIfWaitingForDebugger(int contextGroupId) override {
    v8_inspector__Client__EXTENDER__runIfWaitingForDebugger(*this,
                                                            contextGroupId);
  }
};
}  // namespace v8_inspector

extern "C" {
void v8_inspector__Client__EXTENDER__CTOR(uninit_t<Client__EXTENDER>& buf) {
  new (launder(&buf)) Client__EXTENDER();
}
void v8_inspector__Client__DTOR(Client& self) {
  self.~Client();
}

void v8_inspector__Client__runMessageLoopOnPause(Client& self,
                                                 int contextGroupId) {
  self.runMessageLoopOnPause(contextGroupId);
}
void v8_inspector__Client__quitMessageLoopOnPause(Client& self) {
  self.quitMessageLoopOnPause();
}
void v8_inspector__Client__runIfWaitingForDebugger(Client& self,
                                                   int contextGroupId) {
  self.runIfWaitingForDebugger(contextGroupId);
}
}  // extern "C"