#ifndef SUPPORT_H_
#define SUPPORT_H_

#include <algorithm>
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

// The C-ABI compatible equivalent of V8's Maybe<bool>.
enum class MaybeBool { JustFalse = 0, JustTrue = 1, Nothing = 2 };

inline static MaybeBool maybe_to_maybe_bool(v8::Maybe<bool> maybe) {
  if (maybe.IsNothing()) {
    return MaybeBool::Nothing;
  } else if (maybe.FromJust()) {
    return MaybeBool::JustTrue;
  } else {
    return MaybeBool::JustFalse;
  }
}

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

template <class T>
inline static T* global_to_ptr(v8::Global<T>& global) {
  static_assert(sizeof(v8::Global<T>) == sizeof(T*), "");
  T* ptr = nullptr;
  std::swap(ptr, reinterpret_cast<T*&>(global));
  return ptr;
}

template <class T>
inline static v8::Global<T> ptr_to_global(T* ptr) {
  v8::Global<T> global;
  std::swap(ptr, *reinterpret_cast<T**>(&global));
  return global;
}

}  // namespace support

#endif  // SUPPORT_H_
