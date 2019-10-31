#ifndef SUPPORT_H_
#define SUPPORT_H_

#include <cassert>
#include <memory>
#include <new>
#include <type_traits>
#include <utility>

// Check assumptions made in binding code.
// TODO(ry) re-enable the following
// static_assert(sizeof(bool) == sizeof(uint8_t));
// static_assert(sizeof(std::unique_ptr<void>) == sizeof(void*));

namespace support {
template <class T>
using uninit_t = typename std::aligned_storage<sizeof(T), alignof(T)>::type;

template <class T, class... Args>
void construct_in_place(uninit_t<T>& buf, Args... args) {
  new (&buf) T(std::forward<Args>(args)...);
}
}  // namespace support

#endif  // SUPPORT_H_