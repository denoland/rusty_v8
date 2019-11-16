#include "third_party/v8/include/v8.h"

using namespace v8;

extern "C" {
void v8__V8__SetFlagsFromCommandLine(int* argc, char** argv) {
  V8::SetFlagsFromCommandLine(argc, argv, true);
}

const char* v8__V8__GetVersion() {
  return V8::GetVersion();
}

void v8__V8__InitializePlatform(Platform& platform) {
  V8::InitializePlatform(&platform);
}

void v8__V8__Initialize() {
  V8::Initialize();
}

bool v8__V8__Dispose() {
  return V8::Dispose();
}

void v8__V8__ShutdownPlatform() {
  V8::ShutdownPlatform();
}
}
