#include "third_party/v8/include/v8.h"

extern "C" {
void v8__V8__SetFlagsFromCommandLine(int *argc, char **argv) {
  v8::V8::SetFlagsFromCommandLine(argc, argv, true);
}
}
