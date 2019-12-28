#ifndef SUPPORT_H_
#define SUPPORT_H_

#include <algorithm>
#include <array>
#include <cassert>
#include <memory>
#include <new>
#include <type_traits>
#include <utility>

#include "v8/include/v8.h"

// Check assumptions made in binding code.
static_assert(sizeof(bool) == sizeof(uint8_t), "");
static_assert(sizeof(std::unique_ptr<void>) == sizeof(void*), "");

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

template <class P>
struct make_pod {
  template <class V>
  inline make_pod(V&& value) : pod_(helper<V>(value)) {}
  template <class V>
  inline make_pod(const V& value) : pod_(helper<V>(value)) {}
  inline operator P() { return pod_; }

 private:
  P pod_;

  template <class V>
  union helper {
    static_assert(std::is_pod<P>::value, "type P must a pod type");
    static_assert(sizeof(V) <= sizeof(P),
                  "type P must be at least as big as type V");
    static_assert(alignof(V) <= alignof(P),
                  "alignment of type P must be compatible with that of type V");

    inline helper(V&& value) : value_(value), padding_() {}
    inline helper(const V& value) : value_(value), padding_() {}
    inline ~helper() {}

    inline operator P() {
      // Do a memcpy here avoid undefined behavior.
      P result;
      memcpy(&result, this, sizeof result);
      return result;
    }

   private:
    struct {
      V value_;
      char padding_[sizeof(P) - sizeof(V)];
    };
  };
};

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

// Because, for some reason, Clang complains that `std::aray<void*, 2` is an
// incomplete type, incompatible with C linkage.
struct two_pointers_t {
  void* a;
  void* b;
};

}  // namespace support

#endif  // SUPPORT_H_
