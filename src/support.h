#ifndef SUPPORT_H_
#define SUPPORT_H_

#include <cassert>
#include <memory>
#include <new>
#include <type_traits>
#include <utility>

#include "v8/include/v8.h"

// Check assumptions made in binding code.
// TODO(ry) re-enable the following
// static_assert(sizeof(bool) == sizeof(uint8_t));
// static_assert(sizeof(std::unique_ptr<void>) == sizeof(void*));

namespace support {
template <class T>
using uninit_t = typename std::aligned_storage<sizeof(T), alignof(T)>::type;

template <class T, class... Args>
class construct_in_place_helper {
 public:
  construct_in_place_helper(uninit_t<T>& buf, Args... args)
      : inner_(std::forward<Args>(args)...) {}

 private:
  T inner_;
};

template <class T, class... Args>
void construct_in_place(uninit_t<T>& buf, Args... args) {
  new (&buf)
      construct_in_place_helper<T, Args...>(buf, std::forward<Args>(args)...);
}
}  // namespace support


template <class T>
inline static T* local_to_ptr(v8::Local<T> local) {
  return *local;
}

template <class T>
inline static v8::Local<T> ptr_to_local(T* ptr) {
  static_assert(sizeof(v8::Local<T>) == sizeof(T*), "");
  auto local = *reinterpret_cast<v8::Local<T>*>(&ptr);
  assert(*local == ptr);
  return local;
}

template <class T>
inline static T* maybe_local_to_ptr(v8::MaybeLocal<T> local) {
  return *local.FromMaybe(v8::Local<T>());
}

template <class T>
inline static v8::MaybeLocal<T> ptr_to_maybe_local(T* ptr) {
  static_assert(sizeof(v8::MaybeLocal<T>) == sizeof(T*), "");
  return *reinterpret_cast<v8::MaybeLocal<T>*>(&ptr);
}

#endif  // SUPPORT_H_
