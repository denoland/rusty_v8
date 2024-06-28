#include <v8-cppgc.h>
#include <v8-isolate.h>
#include <v8-message.h>

/**
 * Types defined here will be compiled with bindgen
 * and made available in `crate::binding` in rust.
 */

extern "C" {
class RUST_ExternalOneByteString
    : public v8::String::ExternalOneByteStringResource {
 public:
  using RustDestroyFn = void (*)(char*, size_t);
  // bindgen doesn't support generating bindings for inline
  // constructors/functions
  RUST_ExternalOneByteString(char* data, int length, RustDestroyFn rustDestroy,
                             v8::Isolate* isolate);

  ~RUST_ExternalOneByteString() {
    (*_rustDestroy)(_data, _length);
    _isolate->AdjustAmountOfExternalAllocatedMemory(
        -static_cast<int64_t>(-_length));
  }

  const char* data() const { return _data; }

  size_t length() const { return static_cast<size_t>(_length); }

 private:
  char* const _data;
  const size_t _length;
  RustDestroyFn _rustDestroy;
  v8::Isolate* _isolate;
};

class RustObj final : public cppgc::GarbageCollected<RustObj> {
 public:
  using RustTraceFn = void (*)(const RustObj* obj, cppgc::Visitor*);
  using RustDestroyFn = void (*)(const RustObj* obj);
  explicit RustObj(RustTraceFn trace, RustDestroyFn destroy)
      : trace_(trace), destroy_(destroy) {}

  ~RustObj() { destroy_(this); }

  void Trace(cppgc::Visitor* visitor) const { trace_(this, visitor); }

 private:
  RustTraceFn trace_;
  RustDestroyFn destroy_;
};
}

// Allocate memory using C++'s `new` operator
void* RUST_new(size_t size) { return operator new(size); }

static size_t RUST_v8__ScriptOrigin_SIZE = sizeof(v8::ScriptOrigin);

static size_t RUST_cppgc__Member_SIZE = sizeof(cppgc::Member<RustObj>);
static size_t RUST_cppgc__WeakMember_SIZE = sizeof(cppgc::WeakMember<RustObj>);

static size_t RUST_v8__TracedReference_SIZE =
    sizeof(v8::TracedReference<v8::Data>);
