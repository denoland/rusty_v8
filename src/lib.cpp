
#include <cstdint>
#include <iostream>
#include <memory>
#include <new>
#include <type_traits>
#include <utility>

#include "../goog/v8/include/v8-inspector.h"

template <class T>
using uninit_t = typename std::aligned_storage<sizeof(T), alignof(T)>::type;

// In C++17, this should be backed by std::launder().
template <class T>
auto launder(T ptr) {
  return ptr;
}

#include "v8_inspector/channel.h"
#include "v8_inspector/client.h"
#include "v8_inspector/string_buffer.h"
