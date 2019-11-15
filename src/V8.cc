#include "third_party/v8/include/v8.h"

extern "C" {
void v8__V8__SetFlagsFromCommandLine(int *argc, char **argv) {
  v8::V8::SetFlagsFromCommandLine(argc, argv, true);
}

const char *v8__V8__GetVersion() { return v8::V8::GetVersion(); }

void v8__V8__Initialize() { v8::V8::Initialize(); }

bool v8__V8__Dispose() { return v8::V8::Dispose(); }
}
