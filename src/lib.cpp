
#include <cstdint>
#include <iostream>
#include <memory>
#include <new>
#include <type_traits>
#include <utility>

#include "../v8/include/v8-inspector.h"
namespace v8_inspector {
using Channel = V8Inspector::Channel;
}

template <class T>
using uninit_t = typename std::aligned_storage<sizeof(T), alignof(T)>::type;

#include "v8_inspector/channel.h"
#include "v8_inspector/string_buffer.h"
