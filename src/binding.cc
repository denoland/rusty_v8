#include <cassert>
#include <cstdint>
#include <iostream>

#include "support.h"
#include "v8/include/libplatform/libplatform.h"
#include "v8/include/v8-inspector.h"
#include "v8/include/v8-platform.h"
#include "v8/include/v8-profiler.h"
#include "v8/include/v8.h"
#include "v8/src/execution/isolate-utils-inl.h"
#include "v8/src/execution/isolate-utils.h"
#include "v8/src/objects/maybe-object.h"

using namespace support;

static_assert(sizeof(two_pointers_t) ==
                  sizeof(std::shared_ptr<v8::BackingStore>),
              "std::shared_ptr<v8::BackingStore> size mismatch");

static_assert(sizeof(v8::ScriptOrigin) == sizeof(size_t) * 7,
              "ScriptOrigin size mismatch");

static_assert(sizeof(v8::HandleScope) == sizeof(size_t) * 3,
              "HandleScope size mismatch");

static_assert(sizeof(v8::EscapableHandleScope) == sizeof(size_t) * 4,
              "EscapableHandleScope size mismatch");

static_assert(sizeof(v8::PromiseRejectMessage) == sizeof(size_t) * 3,
              "PromiseRejectMessage size mismatch");

static_assert(sizeof(v8::Locker) == sizeof(size_t) * 2, "Locker size mismatch");

static_assert(sizeof(v8::ScriptCompiler::Source) == sizeof(size_t) * 8,
              "Source size mismatch");

static_assert(sizeof(v8::FunctionCallbackInfo<v8::Value>) == sizeof(size_t) * 3,
              "FunctionCallbackInfo size mismatch");

static_assert(sizeof(v8::PropertyCallbackInfo<v8::Value>) == sizeof(size_t) * 1,
              "PropertyCallbackInfo size mismatch");

static_assert(sizeof(v8::ReturnValue<v8::Value>) == sizeof(size_t) * 1,
              "ReturnValue size mismatch");

static_assert(sizeof(v8::TryCatch) == sizeof(size_t) * 6,
              "TryCatch size mismatch");

static_assert(sizeof(v8::Location) == sizeof(size_t) * 1,
              "Location size mismatch");

static_assert(sizeof(v8::SnapshotCreator) == sizeof(size_t) * 1,
              "SnapshotCreator size mismatch");

static_assert(sizeof(three_pointers_t) == sizeof(v8_inspector::StringView),
              "StringView size mismatch");

enum InternalSlots {
  kSlotDynamicImport = 0,
  kNumInternalSlots,
};
#define SLOT_NUM_EXTERNAL(isolate) \
  (isolate->GetNumberOfDataSlots() - kNumInternalSlots)
#define SLOT_INTERNAL(isolate, slot) \
  (isolate->GetNumberOfDataSlots() - 1 - slot)

// This is an extern C calling convention compatible version of
// v8::HostImportModuleDynamicallyCallback
typedef v8::Promise* (*v8__HostImportModuleDynamicallyCallback)(
    v8::Local<v8::Context> context, v8::Local<v8::ScriptOrModule> referrer,
    v8::Local<v8::String> specifier);

v8::MaybeLocal<v8::Promise> HostImportModuleDynamicallyCallback(
    v8::Local<v8::Context> context, v8::Local<v8::ScriptOrModule> referrer,
    v8::Local<v8::String> specifier) {
  auto* isolate = context->GetIsolate();
  void* d = isolate->GetData(SLOT_INTERNAL(isolate, kSlotDynamicImport));
  auto* callback = reinterpret_cast<v8__HostImportModuleDynamicallyCallback>(d);
  assert(callback != nullptr);
  auto* promise_ptr = callback(context, referrer, specifier);
  if (promise_ptr == nullptr) {
    return v8::MaybeLocal<v8::Promise>();
  } else {
    return v8::MaybeLocal<v8::Promise>(ptr_to_local(promise_ptr));
  }
}

extern "C" {
void v8__V8__SetFlagsFromCommandLine(int* argc, char** argv) {
  v8::V8::SetFlagsFromCommandLine(argc, argv, true);
}

const char* v8__V8__GetVersion() { return v8::V8::GetVersion(); }

void v8__V8__InitializePlatform(v8::Platform* platform) {
  v8::V8::InitializePlatform(platform);
}

void v8__V8__Initialize() { v8::V8::Initialize(); }

bool v8__V8__Dispose() { return v8::V8::Dispose(); }

void v8__V8__ShutdownPlatform() { v8::V8::ShutdownPlatform(); }

v8::Isolate* v8__Isolate__New(const v8::Isolate::CreateParams& params) {
  return v8::Isolate::New(params);
}

void v8__Isolate__Dispose(v8::Isolate* isolate) { isolate->Dispose(); }

void v8__Isolate__Enter(v8::Isolate* isolate) { isolate->Enter(); }

void v8__Isolate__Exit(v8::Isolate* isolate) { isolate->Exit(); }

const v8::Context* v8__Isolate__GetCurrentContext(v8::Isolate* isolate) {
  return local_to_ptr(isolate->GetCurrentContext());
}

const v8::Context* v8__Isolate__GetEnteredOrMicrotaskContext(
    v8::Isolate* isolate) {
  return local_to_ptr(isolate->GetEnteredOrMicrotaskContext());
}

void v8__Isolate__SetData(v8::Isolate* isolate, uint32_t slot, void* data) {
  isolate->SetData(slot, data);
}

void* v8__Isolate__GetData(v8::Isolate* isolate, uint32_t slot) {
  return isolate->GetData(slot);
}

uint32_t v8__Isolate__GetNumberOfDataSlots(v8::Isolate* isolate) {
  return SLOT_NUM_EXTERNAL(isolate);
}

void v8__Isolate__RunMicrotasks(v8::Isolate* isolate) {
  isolate->RunMicrotasks();
}

void v8__Isolate__EnqueueMicrotask(v8::Isolate* isolate,
                                   const v8::Function& function) {
  isolate->EnqueueMicrotask(ptr_to_local(&function));
}

void v8__Isolate__RequestInterrupt(v8::Isolate* isolate,
                                   v8::InterruptCallback callback, void* data) {
  isolate->RequestInterrupt(callback, data);
}

void v8__Isolate__SetPromiseRejectCallback(v8::Isolate* isolate,
                                           v8::PromiseRejectCallback callback) {
  isolate->SetPromiseRejectCallback(callback);
}

void v8__Isolate__SetCaptureStackTraceForUncaughtExceptions(
    v8::Isolate* isolate, bool capture, int frame_limit) {
  isolate->SetCaptureStackTraceForUncaughtExceptions(capture, frame_limit);
}

void v8__Isolate__SetHostInitializeImportMetaObjectCallback(
    v8::Isolate* isolate, v8::HostInitializeImportMetaObjectCallback callback) {
  isolate->SetHostInitializeImportMetaObjectCallback(callback);
}

void v8__Isolate__SetHostImportModuleDynamicallyCallback(
    v8::Isolate* isolate, v8__HostImportModuleDynamicallyCallback callback) {
  isolate->SetData(SLOT_INTERNAL(isolate, kSlotDynamicImport),
                   reinterpret_cast<void*>(callback));
  isolate->SetHostImportModuleDynamicallyCallback(
      HostImportModuleDynamicallyCallback);
}

bool v8__Isolate__AddMessageListener(v8::Isolate* isolate,
                                     v8::MessageCallback callback) {
  return isolate->AddMessageListener(callback);
}

const v8::Value* v8__Isolate__ThrowException(v8::Isolate* isolate,
                                             const v8::Value& exception) {
  return local_to_ptr(isolate->ThrowException(ptr_to_local(&exception)));
}

void v8__Isolate__TerminateExecution(v8::Isolate* isolate) {
  isolate->TerminateExecution();
}

bool v8__Isolate__IsExecutionTerminating(v8::Isolate* isolate) {
  return isolate->IsExecutionTerminating();
}

void v8__Isolate__CancelTerminateExecution(v8::Isolate* isolate) {
  isolate->CancelTerminateExecution();
}

void v8__Isolate__CreateParams__CONSTRUCT(
    uninit_t<v8::Isolate::CreateParams>* buf) {
  construct_in_place<v8::Isolate::CreateParams>(buf);
}

size_t v8__Isolate__CreateParams__SIZEOF() {
  return sizeof(v8::Isolate::CreateParams);
}

void v8__HandleScope__CONSTRUCT(uninit_t<v8::HandleScope>* buf,
                                v8::Isolate* isolate) {
  construct_in_place<v8::HandleScope>(buf, isolate);
}

void v8__HandleScope__DESTRUCT(v8::HandleScope* self) { self->~HandleScope(); }

v8::Isolate* v8__HandleScope__GetIsolate(const v8::HandleScope& self) {
  return self.GetIsolate();
}

void v8__EscapableHandleScope__CONSTRUCT(
    uninit_t<v8::EscapableHandleScope>* buf, v8::Isolate* isolate) {
  construct_in_place<v8::EscapableHandleScope>(buf, isolate);
}

void v8__EscapableHandleScope__DESTRUCT(v8::EscapableHandleScope* self) {
  self->~EscapableHandleScope();
}

const v8::Data* v8__EscapableHandleScope__Escape(v8::EscapableHandleScope* self,
                                                 const v8::Data& value) {
  return local_to_ptr(self->Escape(ptr_to_local(&value)));
}

v8::Isolate* v8__EscapableHandleScope__GetIsolate(
    const v8::EscapableHandleScope& self) {
  return self.GetIsolate();
}

void v8__Locker__CONSTRUCT(uninit_t<v8::Locker>* buf, v8::Isolate* isolate) {
  construct_in_place<v8::Locker>(buf, isolate);
}

void v8__Locker__DESTRUCT(v8::Locker* self) { self->~Locker(); }

const v8::Data* v8__Local__New(v8::Isolate* isolate, const v8::Data& other) {
  return local_to_ptr(v8::Local<v8::Data>::New(isolate, ptr_to_local(&other)));
}

const v8::Data* v8__Global__New(v8::Isolate* isolate, const v8::Data& other) {
  // We have to use `std::move()` here because v8 disables the copy constructor
  // for class `v8::Global`.
  auto global = v8::Global<v8::Data>(isolate, ptr_to_local(&other));
  return make_pod<v8::Data*>(std::move(global));
}

void v8__Global__Reset__0(const v8::Data*& self) {
  auto global = ptr_to_global(self);
  global.Reset();
  self = make_pod<v8::Data*>(std::move(global));
}

void v8__Global__Reset__2(const v8::Data*& self, v8::Isolate* isolate,
                          const v8::Data* const& other) {
  auto global = ptr_to_global(self);
  global.Reset(isolate, ptr_to_local(other));
  self = make_pod<v8::Data*>(std::move(global));
}

void v8__ScriptCompiler__Source__CONSTRUCT(
    uninit_t<v8::ScriptCompiler::Source>* buf, const v8::String& source_string,
    const v8::ScriptOrigin& origin) {
  construct_in_place<v8::ScriptCompiler::Source>(
      buf, ptr_to_local(&source_string), origin);
}

void v8__ScriptCompiler__Source__DESTRUCT(v8::ScriptCompiler::Source* self) {
  self->~Source();
}

const v8::Module* v8__ScriptCompiler__CompileModule(
    v8::Isolate* isolate, v8::ScriptCompiler::Source* source,
    v8::ScriptCompiler::CompileOptions options,
    v8::ScriptCompiler::NoCacheReason no_cache_reason) {
  v8::MaybeLocal<v8::Module> maybe_local = v8::ScriptCompiler::CompileModule(
      isolate, source, options, no_cache_reason);
  if (maybe_local.IsEmpty()) {
    return nullptr;
  } else {
    return local_to_ptr(maybe_local.ToLocalChecked());
  }
}

bool v8__Data__EQ(const v8::Data& self, const v8::Data& other) {
  return ptr_to_local(&self) == ptr_to_local(&other);
}

bool v8__Value__IsUndefined(const v8::Value& self) {
  return self.IsUndefined();
}

bool v8__Value__IsNull(const v8::Value& self) { return self.IsNull(); }

bool v8__Value__IsNullOrUndefined(const v8::Value& self) {
  return self.IsNullOrUndefined();
}

bool v8__Value__IsTrue(const v8::Value& self) { return self.IsTrue(); }

bool v8__Value__IsFalse(const v8::Value& self) { return self.IsFalse(); }

bool v8__Value__IsName(const v8::Value& self) { return self.IsName(); }

bool v8__Value__IsString(const v8::Value& self) { return self.IsString(); }

bool v8__Value__IsSymbol(const v8::Value& self) { return self.IsSymbol(); }

bool v8__Value__IsFunction(const v8::Value& self) { return self.IsFunction(); }

bool v8__Value__IsArray(const v8::Value& self) { return self.IsArray(); }

bool v8__Value__IsObject(const v8::Value& self) { return self.IsObject(); }

bool v8__Value__IsBigInt(const v8::Value& self) { return self.IsBigInt(); }

bool v8__Value__IsBoolean(const v8::Value& self) { return self.IsBoolean(); }

bool v8__Value__IsNumber(const v8::Value& self) { return self.IsNumber(); }

bool v8__Value__IsExternal(const v8::Value& self) { return self.IsExternal(); }

bool v8__Value__IsInt32(const v8::Value& self) { return self.IsInt32(); }

bool v8__Value__IsUint32(const v8::Value& self) { return self.IsUint32(); }

bool v8__Value__IsDate(const v8::Value& self) { return self.IsDate(); }

bool v8__Value__IsArgumentsObject(const v8::Value& self) {
  return self.IsArgumentsObject();
}

bool v8__Value__IsBigIntObject(const v8::Value& self) {
  return self.IsBigIntObject();
}

bool v8__Value__IsBooleanObject(const v8::Value& self) {
  return self.IsBooleanObject();
}

bool v8__Value__IsNumberObject(const v8::Value& self) {
  return self.IsNumberObject();
}

bool v8__Value__IsStringObject(const v8::Value& self) {
  return self.IsStringObject();
}

bool v8__Value__IsSymbolObject(const v8::Value& self) {
  return self.IsSymbolObject();
}

bool v8__Value__IsNativeError(const v8::Value& self) {
  return self.IsNativeError();
}

bool v8__Value__IsRegExp(const v8::Value& self) { return self.IsRegExp(); }

bool v8__Value__IsAsyncFunction(const v8::Value& self) {
  return self.IsAsyncFunction();
}

bool v8__Value__IsGeneratorFunction(const v8::Value& self) {
  return self.IsGeneratorFunction();
}

bool v8__Value__IsGeneratorObject(const v8::Value& self) {
  return self.IsGeneratorObject();
}

bool v8__Value__IsPromise(const v8::Value& self) { return self.IsPromise(); }

bool v8__Value__IsMap(const v8::Value& self) { return self.IsMap(); }

bool v8__Value__IsSet(const v8::Value& self) { return self.IsSet(); }

bool v8__Value__IsMapIterator(const v8::Value& self) {
  return self.IsMapIterator();
}

bool v8__Value__IsSetIterator(const v8::Value& self) {
  return self.IsSetIterator();
}

bool v8__Value__IsWeakMap(const v8::Value& self) { return self.IsWeakMap(); }

bool v8__Value__IsWeakSet(const v8::Value& self) { return self.IsWeakSet(); }

bool v8__Value__IsArrayBuffer(const v8::Value& self) {
  return self.IsArrayBuffer();
}

bool v8__Value__IsArrayBufferView(const v8::Value& self) {
  return self.IsArrayBufferView();
}

bool v8__Value__IsTypedArray(const v8::Value& self) {
  return self.IsTypedArray();
}

bool v8__Value__IsUint8Array(const v8::Value& self) {
  return self.IsUint8Array();
}

bool v8__Value__IsUint8ClampedArray(const v8::Value& self) {
  return self.IsUint8ClampedArray();
}

bool v8__Value__IsInt8Array(const v8::Value& self) {
  return self.IsInt8Array();
}

bool v8__Value__IsUint16Array(const v8::Value& self) {
  return self.IsUint16Array();
}

bool v8__Value__IsInt16Array(const v8::Value& self) {
  return self.IsInt16Array();
}

bool v8__Value__IsUint32Array(const v8::Value& self) {
  return self.IsUint32Array();
}

bool v8__Value__IsInt32Array(const v8::Value& self) {
  return self.IsInt32Array();
}

bool v8__Value__IsFloat32Array(const v8::Value& self) {
  return self.IsFloat32Array();
}

bool v8__Value__IsFloat64Array(const v8::Value& self) {
  return self.IsFloat64Array();
}

bool v8__Value__IsBigInt64Array(const v8::Value& self) {
  return self.IsBigInt64Array();
}

bool v8__Value__IsBigUint64Array(const v8::Value& self) {
  return self.IsBigUint64Array();
}

bool v8__Value__IsDataView(const v8::Value& self) { return self.IsDataView(); }

bool v8__Value__IsSharedArrayBuffer(const v8::Value& self) {
  return self.IsSharedArrayBuffer();
}

bool v8__Value__IsProxy(const v8::Value& self) { return self.IsProxy(); }

bool v8__Value__IsWasmModuleObject(const v8::Value& self) {
  return self.IsWasmModuleObject();
}

bool v8__Value__IsModuleNamespaceObject(const v8::Value& self) {
  return self.IsModuleNamespaceObject();
}

bool v8__Value__StrictEquals(const v8::Value& self, const v8::Value& that) {
  return self.StrictEquals(ptr_to_local(&that));
}

bool v8__Value__SameValue(const v8::Value& self, const v8::Value& that) {
  return self.SameValue(ptr_to_local(&that));
}

const v8::Uint32* v8__Value__ToUint32(const v8::Value& self,
                                      const v8::Context& context) {
  return maybe_local_to_ptr(self.ToUint32(ptr_to_local(&context)));
}

const v8::Int32* v8__Value__ToInt32(const v8::Value& self,
                                    const v8::Context& context) {
  return maybe_local_to_ptr(self.ToInt32(ptr_to_local(&context)));
}

const v8::Integer* v8__Value__ToInteger(const v8::Value& self,
                                        const v8::Context& context) {
  return maybe_local_to_ptr(self.ToInteger(ptr_to_local(&context)));
}

const v8::BigInt* v8__Value__ToBigInt(const v8::Value& self,
                                      const v8::Context& context) {
  return maybe_local_to_ptr(self.ToBigInt(ptr_to_local(&context)));
}

const v8::String* v8__Value__ToString(const v8::Value& self,
                                      const v8::Context& context) {
  return maybe_local_to_ptr(self.ToString(ptr_to_local(&context)));
}

const v8::String* v8__Value__ToDetailString(const v8::Value& self,
                                            const v8::Context& context) {
  return maybe_local_to_ptr(self.ToDetailString(ptr_to_local(&context)));
}

const v8::Number* v8__Value__ToNumber(const v8::Value& self,
                                      const v8::Context& context) {
  return maybe_local_to_ptr(self.ToNumber(ptr_to_local(&context)));
}

const v8::Object* v8__Value__ToObject(const v8::Value& self,
                                      const v8::Context& context) {
  return maybe_local_to_ptr(self.ToObject(ptr_to_local(&context)));
}

void v8__Value__NumberValue(const v8::Value& self, const v8::Context& context,
                            v8::Maybe<double>* out) {
  *out = self.NumberValue(ptr_to_local(&context));
}

void v8__Value__IntegerValue(const v8::Value& self, const v8::Context& context,
                             v8::Maybe<int64_t>* out) {
  *out = self.IntegerValue(ptr_to_local(&context));
}

void v8__Value__Uint32Value(const v8::Value& self, const v8::Context& context,
                            v8::Maybe<uint32_t>* out) {
  *out = self.Uint32Value(ptr_to_local(&context));
}

void v8__Value__Int32Value(const v8::Value& self, const v8::Context& context,
                           v8::Maybe<int32_t>* out) {
  *out = self.Int32Value(ptr_to_local(&context));
}

const v8::Primitive* v8__Null(v8::Isolate* isolate) {
  return local_to_ptr(v8::Null(isolate));
}

const v8::Primitive* v8__Undefined(v8::Isolate* isolate) {
  return local_to_ptr(v8::Undefined(isolate));
}

const v8::Boolean* v8__Boolean__New(v8::Isolate* isolate, bool value) {
  return local_to_ptr(v8::Boolean::New(isolate, value));
}

const v8::PrimitiveArray* v8__PrimitiveArray__New(v8::Isolate* isolate,
                                                  int length) {
  return local_to_ptr(v8::PrimitiveArray::New(isolate, length));
}

int v8__PrimitiveArray__Length(const v8::PrimitiveArray& self) {
  return self.Length();
}

void v8__PrimitiveArray__Set(const v8::PrimitiveArray& self,
                             v8::Isolate* isolate, int index,
                             const v8::Primitive& item) {
  ptr_to_local(&self)->Set(isolate, index, ptr_to_local(&item));
}

const v8::Primitive* v8__PrimitiveArray__Get(const v8::PrimitiveArray& self,
                                             v8::Isolate* isolate, int index) {
  return local_to_ptr(ptr_to_local(&self)->Get(isolate, index));
}

v8::BackingStore* v8__ArrayBuffer__NewBackingStore__with_byte_length(
    v8::Isolate* isolate, size_t byte_length) {
  std::unique_ptr<v8::BackingStore> u =
      v8::ArrayBuffer::NewBackingStore(isolate, byte_length);
  return u.release();
}

v8::BackingStore* v8__ArrayBuffer__NewBackingStore__with_data(
    void* data, size_t byte_length, v8::BackingStoreDeleterCallback deleter,
    void* deleter_data) {
  std::unique_ptr<v8::BackingStore> u = v8::ArrayBuffer::NewBackingStore(
      data, byte_length, deleter, deleter_data);
  return u.release();
}

two_pointers_t v8__ArrayBuffer__GetBackingStore(const v8::ArrayBuffer& self) {
  return make_pod<two_pointers_t>(ptr_to_local(&self)->GetBackingStore());
}

void* v8__BackingStore__Data(const v8::BackingStore& self) {
  return self.Data();
}

size_t v8__BackingStore__ByteLength(const v8::BackingStore& self) {
  return self.ByteLength();
}

bool v8__BackingStore__IsShared(const v8::BackingStore& self) {
  return self.IsShared();
}

void v8__BackingStore__DELETE(v8::BackingStore* self) { delete self; }

two_pointers_t std__shared_ptr__v8__BackingStore__COPY(
    const std::shared_ptr<v8::BackingStore>& ptr) {
  return make_pod<two_pointers_t>(ptr);
}

two_pointers_t std__shared_ptr__v8__BackingStore__CONVERT__std__unique_ptr(
    v8::BackingStore* unique_ptr) {
  return make_pod<two_pointers_t>(
      std::shared_ptr<v8::BackingStore>(unique_ptr));
}

v8::BackingStore* std__shared_ptr__v8__BackingStore__get(
    const std::shared_ptr<v8::BackingStore>& ptr) {
  return ptr.get();
}

void std__shared_ptr__v8__BackingStore__reset(
    std::shared_ptr<v8::BackingStore>* ptr) {
  ptr->reset();
}

long std__shared_ptr__v8__BackingStore__use_count(
    const std::shared_ptr<v8::BackingStore>& ptr) {
  return ptr.use_count();
}

two_pointers_t std__shared_ptr__v8__ArrayBuffer__Allocator__COPY(
    const std::shared_ptr<v8::ArrayBuffer::Allocator>& ptr) {
  return make_pod<two_pointers_t>(ptr);
}

two_pointers_t
std__shared_ptr__v8__ArrayBuffer__Allocator__CONVERT__std__unique_ptr(
    v8::ArrayBuffer::Allocator* unique_ptr) {
  return make_pod<two_pointers_t>(
      std::shared_ptr<v8::ArrayBuffer::Allocator>(unique_ptr));
}

v8::ArrayBuffer::Allocator* std__shared_ptr__v8__ArrayBuffer__Allocator__get(
    const std::shared_ptr<v8::ArrayBuffer::Allocator>& ptr) {
  return ptr.get();
}

void std__shared_ptr__v8__ArrayBuffer__Allocator__reset(
    std::shared_ptr<v8::ArrayBuffer::Allocator>* ptr) {
  ptr->reset();
}

long std__shared_ptr__v8__ArrayBuffer__Allocator__use_count(
    const std::shared_ptr<v8::ArrayBuffer::Allocator>& ptr) {
  return ptr.use_count();
}

const v8::String* v8__String__Empty(v8::Isolate* isolate) {
  return local_to_ptr(v8::String::Empty(isolate));
}

const v8::String* v8__String__NewFromUtf8(v8::Isolate* isolate,
                                          const char* data,
                                          v8::NewStringType new_type,
                                          int length) {
  return maybe_local_to_ptr(
      v8::String::NewFromUtf8(isolate, data, new_type, length));
}

int v8__String__Length(const v8::String& self) { return self.Length(); }

int v8__String__Utf8Length(const v8::String& self, v8::Isolate* isolate) {
  return self.Utf8Length(isolate);
}

int v8__String__WriteUtf8(const v8::String& self, v8::Isolate* isolate,
                          char* buffer, int length, int* nchars_ref,
                          int options) {
  return self.WriteUtf8(isolate, buffer, length, nchars_ref, options);
}

void v8__Template__Set(const v8::Template& self, const v8::Name& key,
                       const v8::Data& value, v8::PropertyAttribute attr) {
  ptr_to_local(&self)->Set(ptr_to_local(&key), ptr_to_local(&value), attr);
}

const v8::ObjectTemplate* v8__ObjectTemplate__New(
    v8::Isolate* isolate, const v8::FunctionTemplate& templ) {
  return local_to_ptr(v8::ObjectTemplate::New(isolate, ptr_to_local(&templ)));
}

const v8::Object* v8__ObjectTemplate__NewInstance(
    const v8::ObjectTemplate& self, const v8::Context& context) {
  return maybe_local_to_ptr(
      ptr_to_local(&self)->NewInstance(ptr_to_local(&context)));
}

const v8::Object* v8__Object__New(v8::Isolate* isolate) {
  return local_to_ptr(v8::Object::New(isolate));
}

const v8::Object* v8__Object__New__with_prototype_and_properties(
    v8::Isolate* isolate, const v8::Value& prototype_or_null,
    const v8::Name* const names[], const v8::Value* const values[],
    size_t length) {
  return local_to_ptr(v8::Object::New(isolate, ptr_to_local(&prototype_or_null),
                                      const_ptr_array_to_local_array(names),
                                      const_ptr_array_to_local_array(values),
                                      length));
}

const v8::Value* v8__Object__Get(const v8::Object& self,
                                 const v8::Context& context,
                                 const v8::Value& key) {
  return maybe_local_to_ptr(
      ptr_to_local(&self)->Get(ptr_to_local(&context), ptr_to_local(&key)));
}

const v8::Value* v8__Object__GetIndex(const v8::Object& self,
                                      const v8::Context& context,
                                      uint32_t index) {
  return maybe_local_to_ptr(
      ptr_to_local(&self)->Get(ptr_to_local(&context), index));
}

const v8::Value* v8__Object__GetPrototype(const v8::Object& self) {
  return local_to_ptr(ptr_to_local(&self)->GetPrototype());
}

MaybeBool v8__Object__Set(const v8::Object& self, const v8::Context& context,
                          const v8::Value& key, const v8::Value& value) {
  return maybe_to_maybe_bool(ptr_to_local(&self)->Set(
      ptr_to_local(&context), ptr_to_local(&key), ptr_to_local(&value)));
}

MaybeBool v8__Object__SetIndex(const v8::Object& self,
                               const v8::Context& context, uint32_t index,
                               const v8::Value& value) {
  return maybe_to_maybe_bool(ptr_to_local(&self)->Set(
      ptr_to_local(&context), index, ptr_to_local(&value)));
}

MaybeBool v8__Object__SetPrototype(const v8::Object& self,
                                   const v8::Context& context,
                                   const v8::Value& prototype) {
  return maybe_to_maybe_bool(ptr_to_local(&self)->SetPrototype(
      ptr_to_local(&context), ptr_to_local(&prototype)));
}

MaybeBool v8__Object__CreateDataProperty(const v8::Object& self,
                                         const v8::Context& context,
                                         const v8::Name& key,
                                         const v8::Value& value) {
  return maybe_to_maybe_bool(ptr_to_local(&self)->CreateDataProperty(
      ptr_to_local(&context), ptr_to_local(&key), ptr_to_local(&value)));
}

MaybeBool v8__Object__DefineOwnProperty(const v8::Object& self,
                                        const v8::Context& context,
                                        const v8::Name& key,
                                        const v8::Value& value,
                                        v8::PropertyAttribute attr) {
  return maybe_to_maybe_bool(ptr_to_local(&self)->DefineOwnProperty(
      ptr_to_local(&context), ptr_to_local(&key), ptr_to_local(&value), attr));
}

MaybeBool v8__Object__SetAccessor(const v8::Object& self,
                                  const v8::Context& context,
                                  const v8::Name& key,
                                  v8::AccessorNameGetterCallback getter) {
  return maybe_to_maybe_bool(ptr_to_local(&self)->SetAccessor(
      ptr_to_local(&context), ptr_to_local(&key), getter));
}

v8::Isolate* v8__Object__GetIsolate(const v8::Object& self) {
  return ptr_to_local(&self)->GetIsolate();
}

int v8__Object__GetIdentityHash(const v8::Object& self) {
  return ptr_to_local(&self)->GetIdentityHash();
}

const v8::Context* v8__Object__CreationContext(const v8::Object& self) {
  return local_to_ptr(ptr_to_local(&self)->CreationContext());
}

const v8::Array* v8__Object__GetOwnPropertyNames(const v8::Object* self,
                                                 const v8::Context* context) {
  return maybe_local_to_ptr(
      ptr_to_local(self)->GetOwnPropertyNames(ptr_to_local(context)));
}

const v8::Array* v8__Object__GetPropertyNames(const v8::Object* self,
                                              const v8::Context* context) {
  return maybe_local_to_ptr(
      ptr_to_local(self)->GetPropertyNames(ptr_to_local(context)));
}

const v8::Array* v8__Array__New(v8::Isolate* isolate, int length) {
  return local_to_ptr(v8::Array::New(isolate, length));
}

const v8::Array* v8__Array__New_with_elements(v8::Isolate* isolate,
                                              const v8::Value* const elements[],
                                              size_t length) {
  return local_to_ptr(v8::Array::New(
      isolate, const_ptr_array_to_local_array(elements), length));
}

uint32_t v8__Array__Length(const v8::Array& self) { return self.Length(); }

size_t v8__Map__Size(const v8::Map& self) { return self.Size(); }

const v8::Array* v8__Map__As__Array(const v8::Map& self) {
  return local_to_ptr(self.AsArray());
}

const v8::Number* v8__Number__New(v8::Isolate* isolate, double value) {
  return *v8::Number::New(isolate, value);
}

double v8__Number__Value(const v8::Number& self) { return self.Value(); }

const v8::Integer* v8__Integer__New(v8::Isolate* isolate, int32_t value) {
  return *v8::Integer::New(isolate, value);
}

const v8::Integer* v8__Integer__NewFromUnsigned(v8::Isolate* isolate,
                                                uint32_t value) {
  return *v8::Integer::NewFromUnsigned(isolate, value);
}

int64_t v8__Integer__Value(const v8::Integer& self) { return self.Value(); }

const v8::ArrayBuffer* v8__ArrayBufferView__Buffer(
    const v8::ArrayBufferView& self) {
  return local_to_ptr(ptr_to_local(&self)->Buffer());
}

size_t v8__ArrayBufferView__ByteLength(const v8::ArrayBufferView& self) {
  return ptr_to_local(&self)->ByteLength();
}

size_t v8__ArrayBufferView__ByteOffset(const v8::ArrayBufferView& self) {
  return ptr_to_local(&self)->ByteOffset();
}

size_t v8__ArrayBufferView__CopyContents(const v8::ArrayBufferView& self,
                                         void* dest, int byte_length) {
  return ptr_to_local(&self)->CopyContents(dest, byte_length);
}

v8::ArrayBuffer::Allocator* v8__ArrayBuffer__Allocator__NewDefaultAllocator() {
  return v8::ArrayBuffer::Allocator::NewDefaultAllocator();
}

void v8__ArrayBuffer__Allocator__DELETE(v8::ArrayBuffer::Allocator* self) {
  delete self;
}

const v8::ArrayBuffer* v8__ArrayBuffer__New__with_byte_length(
    v8::Isolate* isolate, size_t byte_length) {
  return local_to_ptr(v8::ArrayBuffer::New(isolate, byte_length));
}

const v8::ArrayBuffer* v8__ArrayBuffer__New__with_backing_store(
    v8::Isolate* isolate,
    const std::shared_ptr<v8::BackingStore>& backing_store) {
  return local_to_ptr(v8::ArrayBuffer::New(isolate, backing_store));
}

size_t v8__ArrayBuffer__ByteLength(const v8::ArrayBuffer& self) {
  return self.ByteLength();
}

struct InternalFieldData {
  uint32_t data;
};

std::vector<InternalFieldData*> deserialized_data;

void DeserializeInternalFields(v8::Local<v8::Object> holder, int index,
                               v8::StartupData payload, void* data) {
  assert(data == nullptr);
  if (payload.raw_size == 0) {
    holder->SetAlignedPointerInInternalField(index, nullptr);
    return;
  }
  InternalFieldData* embedder_field = new InternalFieldData{0};
  memcpy(embedder_field, payload.data, payload.raw_size);
  holder->SetAlignedPointerInInternalField(index, embedder_field);
  deserialized_data.push_back(embedder_field);
}

const v8::Context* v8__Context__New(v8::Isolate* isolate,
                                    const v8::ObjectTemplate* templ,
                                    const v8::Value* global_object) {
  return local_to_ptr(
      v8::Context::New(isolate, nullptr, ptr_to_maybe_local(templ),
                       ptr_to_maybe_local(global_object),
                       v8::DeserializeInternalFieldsCallback(
                           DeserializeInternalFields, nullptr)));
}

void v8__Context__Enter(const v8::Context& self) {
  ptr_to_local(&self)->Enter();
}

void v8__Context__Exit(const v8::Context& self) { ptr_to_local(&self)->Exit(); }

v8::Isolate* v8__Context__GetIsolate(const v8::Context& self) {
  return ptr_to_local(&self)->GetIsolate();
}

const v8::Object* v8__Context__Global(const v8::Context& self) {
  return local_to_ptr(ptr_to_local(&self)->Global());
}

const v8::String* v8__Message__Get(const v8::Message& self) {
  return local_to_ptr(self.Get());
}

const v8::String* v8__Message__GetSourceLine(const v8::Message& self,
                                             const v8::Context& context) {
  return maybe_local_to_ptr(self.GetSourceLine(ptr_to_local(&context)));
}

const v8::Value* v8__Message__GetScriptResourceName(const v8::Message& self) {
  return local_to_ptr(self.GetScriptResourceName());
}

int v8__Message__GetLineNumber(const v8::Message& self,
                               const v8::Context& context) {
  v8::Maybe<int> maybe = self.GetLineNumber(ptr_to_local(&context));
  if (maybe.IsJust()) {
    return maybe.ToChecked();
  } else {
    return -1;
  }
}

const v8::StackTrace* v8__Message__GetStackTrace(const v8::Message& self) {
  return local_to_ptr(self.GetStackTrace());
}

int v8__Message__GetStartPosition(const v8::Message& self) {
  return self.GetStartPosition();
}

int v8__Message__GetEndPosition(const v8::Message& self) {
  return self.GetEndPosition();
}

int v8__Message__GetWasmFunctionIndex(const v8::Message& self) {
  return self.GetWasmFunctionIndex();
}

int v8__Message__ErrorLevel(const v8::Message& self) {
  return self.ErrorLevel();
}

int v8__Message__GetStartColumn(const v8::Message& self) {
  return self.GetStartColumn();
}

int v8__Message__GetEndColumn(const v8::Message& self) {
  return self.GetEndColumn();
}

bool v8__Message__IsSharedCrossOrigin(const v8::Message& self) {
  return self.IsSharedCrossOrigin();
}

bool v8__Message__IsOpaque(const v8::Message& self) { return self.IsOpaque(); }

v8::Isolate* v8__Message__GetIsolate(const v8::Message& self) {
  return self.GetIsolate();
}

const v8::Value* v8__Exception__RangeError(const v8::String& message) {
  return local_to_ptr(v8::Exception::RangeError(ptr_to_local(&message)));
}

const v8::Value* v8__Exception__ReferenceError(const v8::String& message) {
  return local_to_ptr(v8::Exception::ReferenceError(ptr_to_local(&message)));
}

const v8::Value* v8__Exception__SyntaxError(const v8::String& message) {
  return local_to_ptr(v8::Exception::SyntaxError(ptr_to_local(&message)));
}

const v8::Value* v8__Exception__TypeError(const v8::String& message) {
  return local_to_ptr(v8::Exception::TypeError(ptr_to_local(&message)));
}

const v8::Value* v8__Exception__Error(const v8::String& message) {
  return local_to_ptr(v8::Exception::Error(ptr_to_local(&message)));
}

const v8::Message* v8__Exception__CreateMessage(v8::Isolate* isolate,
                                                const v8::Value& exception) {
  return local_to_ptr(
      v8::Exception::CreateMessage(isolate, ptr_to_local(&exception)));
}

const v8::StackTrace* v8__Exception__GetStackTrace(const v8::Value& exception) {
  return local_to_ptr(v8::Exception::GetStackTrace(ptr_to_local(&exception)));
}

const v8::Function* v8__Function__New(const v8::Context& context,
                                      v8::FunctionCallback callback) {
  return maybe_local_to_ptr(
      v8::Function::New(ptr_to_local(&context), callback));
}

const v8::Function* v8__Function__NewWithData(const v8::Context& context,
                                              v8::FunctionCallback callback,
                                              const v8::Value& data) {
  return maybe_local_to_ptr(
      v8::Function::New(ptr_to_local(&context), callback, ptr_to_local(&data)));
}

const v8::Value* v8__Function__Call(const v8::Function& self,
                                    const v8::Context& context,
                                    const v8::Value& recv, int argc,
                                    const v8::Value* const argv[]) {
  return maybe_local_to_ptr(
      ptr_to_local(&self)->Call(ptr_to_local(&context), ptr_to_local(&recv),
                                argc, const_ptr_array_to_local_array(argv)));
}

const v8::FunctionTemplate* v8__FunctionTemplate__New(
    v8::Isolate* isolate, v8::FunctionCallback callback = nullptr) {
  return local_to_ptr(v8::FunctionTemplate::New(isolate, callback));
}

const v8::Function* v8__FunctionTemplate__GetFunction(
    const v8::FunctionTemplate& self, const v8::Context& context) {
  return maybe_local_to_ptr(
      ptr_to_local(&self)->GetFunction(ptr_to_local(&context)));
}

void v8__FunctionTemplate__SetClassName(const v8::FunctionTemplate& self,
                                        const v8::String& name) {
  ptr_to_local(&self)->SetClassName(ptr_to_local(&name));
}

v8::Isolate* v8__FunctionCallbackInfo__GetIsolate(
    const v8::FunctionCallbackInfo<v8::Value>& self) {
  return self.GetIsolate();
}

v8::Value* v8__FunctionCallbackInfo__GetReturnValue(
    const v8::FunctionCallbackInfo<v8::Value>& self) {
  return make_pod<v8::Value*>(self.GetReturnValue());
}

const v8::Object* v8__FunctionCallbackInfo__This(
    const v8::FunctionCallbackInfo<v8::Value>& self) {
  return local_to_ptr(self.This());
}

int v8__FunctionCallbackInfo__Length(
    const v8::FunctionCallbackInfo<v8::Value>& self) {
  return self.Length();
}

const v8::Value* v8__FunctionCallbackInfo__GetArgument(
    const v8::FunctionCallbackInfo<v8::Value>& self, int i) {
  return local_to_ptr(self[i]);
}

const v8::Value* v8__FunctionCallbackInfo__Data(
    const v8::FunctionCallbackInfo<v8::Value>& self) {
  return local_to_ptr(self.Data());
}

void v8__ReturnValue__Set(v8::ReturnValue<v8::Value>* self,
                          const v8::Value& value) {
  self->Set(ptr_to_local(&value));
}

const v8::Value* v8__ReturnValue__Get(const v8::ReturnValue<v8::Value>& self) {
  return local_to_ptr(self.Get());
}

int v8__StackTrace__GetFrameCount(const v8::StackTrace& self) {
  return self.GetFrameCount();
}

const v8::StackFrame* v8__StackTrace__GetFrame(const v8::StackTrace& self,
                                               v8::Isolate* isolate,
                                               uint32_t index) {
  return local_to_ptr(self.GetFrame(isolate, index));
}

int v8__StackFrame__GetLineNumber(const v8::StackFrame& self) {
  return self.GetLineNumber();
}

int v8__StackFrame__GetColumn(const v8::StackFrame& self) {
  return self.GetColumn();
}

int v8__StackFrame__GetScriptId(const v8::StackFrame& self) {
  return self.GetScriptId();
}

const v8::String* v8__StackFrame__GetScriptName(const v8::StackFrame& self) {
  return local_to_ptr(self.GetScriptName());
}

const v8::String* v8__StackFrame__GetScriptNameOrSourceURL(
    const v8::StackFrame& self) {
  return local_to_ptr(self.GetScriptNameOrSourceURL());
}

const v8::String* v8__StackFrame__GetFunctionName(const v8::StackFrame& self) {
  return local_to_ptr(self.GetFunctionName());
}

bool v8__StackFrame__IsEval(const v8::StackFrame& self) {
  return self.IsEval();
}

bool v8__StackFrame__IsConstructor(const v8::StackFrame& self) {
  return self.IsConstructor();
}

bool v8__StackFrame__IsWasm(const v8::StackFrame& self) {
  return self.IsWasm();
}

bool v8__StackFrame__IsUserJavaScript(const v8::StackFrame& self) {
  return self.IsUserJavaScript();
}

void v8__TryCatch__CONSTRUCT(uninit_t<v8::TryCatch>* buf,
                             v8::Isolate* isolate) {
  construct_in_place<v8::TryCatch>(buf, isolate);
}

void v8__TryCatch__DESTRUCT(v8::TryCatch* self) { self->~TryCatch(); }

bool v8__TryCatch__HasCaught(const v8::TryCatch& self) {
  return self.HasCaught();
}

bool v8__TryCatch__CanContinue(const v8::TryCatch& self) {
  return self.CanContinue();
}

bool v8__TryCatch__HasTerminated(const v8::TryCatch& self) {
  return self.HasTerminated();
}

const v8::Value* v8__TryCatch__Exception(const v8::TryCatch& self) {
  return local_to_ptr(self.Exception());
}

const v8::Value* v8__TryCatch__StackTrace(const v8::TryCatch& self,
                                          const v8::Context& context) {
  return maybe_local_to_ptr(self.StackTrace(ptr_to_local(&context)));
}

const v8::Message* v8__TryCatch__Message(const v8::TryCatch& self) {
  return local_to_ptr(self.Message());
}

void v8__TryCatch__Reset(v8::TryCatch* self) { self->Reset(); }

const v8::Value* v8__TryCatch__ReThrow(v8::TryCatch* self) {
  return local_to_ptr(self->ReThrow());
}

bool v8__TryCatch__IsVerbose(const v8::TryCatch& self) {
  return self.IsVerbose();
}

void v8__TryCatch__SetVerbose(v8::TryCatch* self, bool value) {
  self->SetVerbose(value);
}

void v8__TryCatch__SetCaptureMessage(v8::TryCatch* self, bool value) {
  self->SetCaptureMessage(value);
}

const v8::Uint8Array* v8__Uint8Array__New(const v8::ArrayBuffer& buf_ptr,
                                          size_t byte_offset, size_t length) {
  return local_to_ptr(
      v8::Uint8Array::New(ptr_to_local(&buf_ptr), byte_offset, length));
}

const v8::Script* v8__Script__Compile(const v8::Context& context,
                                      const v8::String& source,
                                      const v8::ScriptOrigin& origin) {
  return maybe_local_to_ptr(
      v8::Script::Compile(ptr_to_local(&context), ptr_to_local(&source),
                          const_cast<v8::ScriptOrigin*>(&origin)));
}

const v8::Value* v8__Script__Run(const v8::Script& script,
                                 const v8::Context& context) {
  return maybe_local_to_ptr(ptr_to_local(&script)->Run(ptr_to_local(&context)));
}

void v8__ScriptOrigin__CONSTRUCT(
    uninit_t<v8::ScriptOrigin>* buf, const v8::Value& resource_name,
    const v8::Integer& resource_line_offset,
    const v8::Integer& resource_column_offset,
    const v8::Boolean& resource_is_shared_cross_origin,
    const v8::Integer& script_id, const v8::Value& source_map_url,
    const v8::Boolean& resource_is_opaque, const v8::Boolean& is_wasm,
    const v8::Boolean& is_module) {
  construct_in_place<v8::ScriptOrigin>(
      buf, ptr_to_local(&resource_name), ptr_to_local(&resource_line_offset),
      ptr_to_local(&resource_column_offset),
      ptr_to_local(&resource_is_shared_cross_origin), ptr_to_local(&script_id),
      ptr_to_local(&source_map_url), ptr_to_local(&resource_is_opaque),
      ptr_to_local(&is_wasm), ptr_to_local(&is_module));
}

const v8::Value* v8__ScriptOrModule__GetResourceName(
    const v8::ScriptOrModule& self) {
  return local_to_ptr(ptr_to_local(&self)->GetResourceName());
}

const v8::PrimitiveArray* v8__ScriptOrModule__GetHostDefinedOptions(
    const v8::ScriptOrModule& self) {
  return local_to_ptr(ptr_to_local(&self)->GetHostDefinedOptions());
}

const v8::SharedArrayBuffer* v8__SharedArrayBuffer__New__with_byte_length(
    v8::Isolate* isolate, size_t byte_length) {
  return local_to_ptr(v8::SharedArrayBuffer::New(isolate, byte_length));
}

const v8::SharedArrayBuffer* v8__SharedArrayBuffer__New__with_backing_store(
    v8::Isolate* isolate,
    const std::shared_ptr<v8::BackingStore>& backing_store) {
  return local_to_ptr(v8::SharedArrayBuffer::New(isolate, backing_store));
}

size_t v8__SharedArrayBuffer__ByteLength(const v8::SharedArrayBuffer& self) {
  return self.ByteLength();
}

two_pointers_t v8__SharedArrayBuffer__GetBackingStore(
    const v8::SharedArrayBuffer& self) {
  return make_pod<two_pointers_t>(ptr_to_local(&self)->GetBackingStore());
}

v8::BackingStore* v8__SharedArrayBuffer__NewBackingStore__with_byte_length(
    v8::Isolate* isolate, size_t byte_length) {
  std::unique_ptr<v8::BackingStore> u =
      v8::SharedArrayBuffer::NewBackingStore(isolate, byte_length);
  return u.release();
}

v8::BackingStore* v8__SharedArrayBuffer__NewBackingStore__with_data(
    void* data, size_t byte_length, v8::BackingStoreDeleterCallback deleter,
    void* deleter_data) {
  std::unique_ptr<v8::BackingStore> u = v8::SharedArrayBuffer::NewBackingStore(
      data, byte_length, deleter, deleter_data);
  return u.release();
}

const v8::Value* v8__JSON__Parse(const v8::Context& context,
                                 const v8::String& json_string) {
  return maybe_local_to_ptr(
      v8::JSON::Parse(ptr_to_local(&context), ptr_to_local(&json_string)));
}

const v8::String* v8__JSON__Stringify(const v8::Context& context,
                                      const v8::Value& json_object) {
  return maybe_local_to_ptr(
      v8::JSON::Stringify(ptr_to_local(&context), ptr_to_local(&json_object)));
}

const v8::Promise::Resolver* v8__Promise__Resolver__New(
    const v8::Context& context) {
  return maybe_local_to_ptr(v8::Promise::Resolver::New(ptr_to_local(&context)));
}

const v8::Promise* v8__Promise__Resolver__GetPromise(
    const v8::Promise::Resolver& self) {
  return local_to_ptr(ptr_to_local(&self)->GetPromise());
}

MaybeBool v8__Promise__Resolver__Resolve(const v8::Promise::Resolver& self,
                                         const v8::Context& context,
                                         const v8::Value& value) {
  return maybe_to_maybe_bool(ptr_to_local(&self)->Resolve(
      ptr_to_local(&context), ptr_to_local(&value)));
}

MaybeBool v8__Promise__Resolver__Reject(const v8::Promise::Resolver& self,
                                        const v8::Context& context,
                                        const v8::Value& value) {
  return maybe_to_maybe_bool(ptr_to_local(&self)->Reject(ptr_to_local(&context),
                                                         ptr_to_local(&value)));
}

v8::Promise::PromiseState v8__Promise__State(const v8::Promise& self) {
  return ptr_to_local(&self)->State();
}

bool v8__Promise__HasHandler(const v8::Promise& self) {
  return ptr_to_local(&self)->HasHandler();
}

const v8::Value* v8__Promise__Result(const v8::Promise& self) {
  return local_to_ptr(ptr_to_local(&self)->Result());
}

const v8::Promise* v8__Promise__Catch(const v8::Promise& self,
                                      const v8::Context& context,
                                      const v8::Function& handler) {
  return maybe_local_to_ptr(ptr_to_local(&self)->Catch(ptr_to_local(&context),
                                                       ptr_to_local(&handler)));
}

const v8::Promise* v8__Promise__Then(const v8::Promise& self,
                                     const v8::Context& context,
                                     const v8::Function& handler) {
  return maybe_local_to_ptr(ptr_to_local(&self)->Then(ptr_to_local(&context),
                                                      ptr_to_local(&handler)));
}

const v8::Promise* v8__Promise__Then2(const v8::Promise& self,
                                      const v8::Context& context,
                                      const v8::Function& on_fulfilled,
                                      const v8::Function& on_rejected) {
  return maybe_local_to_ptr(ptr_to_local(&self)->Then(
      ptr_to_local(&context), ptr_to_local(&on_fulfilled),
      ptr_to_local(&on_rejected)));
}

v8::PromiseRejectEvent v8__PromiseRejectMessage__GetEvent(
    const v8::PromiseRejectMessage& self) {
  return self.GetEvent();
}

const v8::Promise* v8__PromiseRejectMessage__GetPromise(
    const v8::PromiseRejectMessage& self) {
  return local_to_ptr(self.GetPromise());
}

const v8::Value* v8__PromiseRejectMessage__GetValue(
    const v8::PromiseRejectMessage& self) {
  return local_to_ptr(self.GetValue());
}

v8::Isolate* v8__PropertyCallbackInfo__GetIsolate(
    const v8::PropertyCallbackInfo<v8::Value>& self) {
  return self.GetIsolate();
}

v8::Value* v8__PropertyCallbackInfo__GetReturnValue(
    const v8::PropertyCallbackInfo<v8::Value>& self) {
  return make_pod<v8::Value*>(self.GetReturnValue());
}

const v8::Object* v8__PropertyCallbackInfo__This(
    const v8::PropertyCallbackInfo<v8::Value>& self) {
  return local_to_ptr(self.This());
}

const v8::Proxy* v8__Proxy__New(const v8::Context& context,
                                const v8::Object& target,
                                const v8::Object& handler) {
  return maybe_local_to_ptr(v8::Proxy::New(
      ptr_to_local(&context), ptr_to_local(&target), ptr_to_local(&handler)));
}

const v8::Value* v8__Proxy__GetHandler(const v8::Proxy& self) {
  return local_to_ptr(ptr_to_local(&self)->GetHandler());
}

const v8::Value* v8__Proxy__GetTarget(const v8::Proxy& self) {
  return local_to_ptr(ptr_to_local(&self)->GetTarget());
}

bool v8__Proxy__IsRevoked(const v8::Proxy& self) {
  return ptr_to_local(&self)->IsRevoked();
}

void v8__Proxy__Revoke(const v8::Proxy& self) { ptr_to_local(&self)->Revoke(); }

void v8__SnapshotCreator__CONSTRUCT(uninit_t<v8::SnapshotCreator>* buf,
                                    const intptr_t* external_references) {
  construct_in_place<v8::SnapshotCreator>(buf, external_references);
}

void v8__SnapshotCreator__DESTRUCT(v8::SnapshotCreator* self) {
  self->~SnapshotCreator();
}

void v8__StartupData__DESTRUCT(v8::StartupData* self) { delete[] self->data; }

v8::Isolate* v8__SnapshotCreator__GetIsolate(v8::SnapshotCreator* self) {
  return self->GetIsolate();
}

v8::StartupData SerializeInternalFields(v8::Local<v8::Object> holder, int index,
                                        void* data) {
  assert(data == nullptr);
  InternalFieldData* embedder_field = static_cast<InternalFieldData*>(
      holder->GetAlignedPointerFromInternalField(index));
  if (embedder_field == nullptr) return {nullptr, 0};
  int size = sizeof(*embedder_field);
  char* payload = new char[size];
  // We simply use memcpy to serialize the content.
  memcpy(payload, embedder_field, size);
  return {payload, size};
}

void v8__SnapshotCreator__SetDefaultContext(v8::SnapshotCreator* self,
                                            const v8::Context& context) {
  self->SetDefaultContext(ptr_to_local(&context), SerializeInternalFields);
}

v8::StartupData v8__SnapshotCreator__CreateBlob(
    v8::SnapshotCreator* self,
    v8::SnapshotCreator::FunctionCodeHandling function_code_handling) {
  return self->CreateBlob(function_code_handling);
}

v8::Platform* v8__platform__NewDefaultPlatform() {
  // TODO: support optional arguments.
  return v8::platform::NewDefaultPlatform().release();
}

void v8__Platform__DELETE(v8::Platform* self) { delete self; }
void v8__Task__BASE__DELETE(v8::Task* self);
void v8__Task__BASE__Run(v8::Task* self);

struct v8__Task__BASE : public v8::Task {
  using Task::Task;
  void operator delete(void* ptr) noexcept {
    v8__Task__BASE__DELETE(reinterpret_cast<v8::Task*>(ptr));
  }
  void Run() override { v8__Task__BASE__Run(this); }
};

void v8__Task__BASE__CONSTRUCT(uninit_t<v8__Task__BASE>* buf) {
  construct_in_place<v8__Task__BASE>(buf);
}
void v8__Task__DELETE(v8::Task* self) { delete self; }
void v8__Task__Run(v8::Task* self) { self->Run(); }

void v8_inspector__V8Inspector__Channel__BASE__sendResponse(
    v8_inspector::V8Inspector::Channel* self, int callId,
    v8_inspector::StringBuffer* message);
void v8_inspector__V8Inspector__Channel__BASE__sendNotification(
    v8_inspector::V8Inspector::Channel* self,
    v8_inspector::StringBuffer* message);
void v8_inspector__V8Inspector__Channel__BASE__flushProtocolNotifications(
    v8_inspector::V8Inspector::Channel* self);

void v8_inspector__V8Inspector__DELETE(v8_inspector::V8Inspector* self) {
  delete self;
}

v8_inspector::V8Inspector* v8_inspector__V8Inspector__create(
    v8::Isolate* isolate, v8_inspector::V8InspectorClient* client) {
  std::unique_ptr<v8_inspector::V8Inspector> u =
      v8_inspector::V8Inspector::create(isolate, client);
  return u.release();
}

v8_inspector::V8InspectorSession* v8_inspector__V8Inspector__connect(
    v8_inspector::V8Inspector* self, int context_group_id,
    v8_inspector::V8Inspector::Channel* channel,
    v8_inspector::StringView state) {
  std::unique_ptr<v8_inspector::V8InspectorSession> u =
      self->connect(context_group_id, channel, state);
  return u.release();
}

void v8_inspector__V8Inspector__contextCreated(
    v8_inspector::V8Inspector* self, const v8::Context& context,
    int contextGroupId, v8_inspector::StringView humanReadableName) {
  self->contextCreated(v8_inspector::V8ContextInfo(
      ptr_to_local(&context), contextGroupId, humanReadableName));
}

void v8_inspector__V8InspectorSession__DELETE(
    v8_inspector::V8InspectorSession* self) {
  delete self;
}

void v8_inspector__V8InspectorSession__dispatchProtocolMessage(
    v8_inspector::V8InspectorSession* self, v8_inspector::StringView message) {
  self->dispatchProtocolMessage(message);
}

void v8_inspector__V8InspectorSession__schedulePauseOnNextStatement(
    v8_inspector::V8InspectorSession* self, v8_inspector::StringView reason,
    v8_inspector::StringView detail) {
  self->schedulePauseOnNextStatement(reason, detail);
}
}  // extern "C"

struct v8_inspector__V8Inspector__Channel__BASE
    : public v8_inspector::V8Inspector::Channel {
  using v8_inspector::V8Inspector::Channel::Channel;

  void sendResponse(
      int callId,
      std::unique_ptr<v8_inspector::StringBuffer> message) override {
    v8_inspector__V8Inspector__Channel__BASE__sendResponse(this, callId,
                                                           message.release());
  }
  void sendNotification(
      std::unique_ptr<v8_inspector::StringBuffer> message) override {
    v8_inspector__V8Inspector__Channel__BASE__sendNotification(
        this, message.release());
  }
  void flushProtocolNotifications() override {
    v8_inspector__V8Inspector__Channel__BASE__flushProtocolNotifications(this);
  }
};

extern "C" {
void v8_inspector__V8Inspector__Channel__BASE__CONSTRUCT(
    uninit_t<v8_inspector__V8Inspector__Channel__BASE>* buf) {
  construct_in_place<v8_inspector__V8Inspector__Channel__BASE>(buf);
}

void v8_inspector__V8Inspector__Channel__sendResponse(
    v8_inspector::V8Inspector::Channel* self, int callId,
    v8_inspector::StringBuffer* message) {
  self->sendResponse(
      callId,
      static_cast<std::unique_ptr<v8_inspector::StringBuffer>>(message));
}
void v8_inspector__V8Inspector__Channel__sendNotification(
    v8_inspector::V8Inspector::Channel* self,
    v8_inspector::StringBuffer* message) {
  self->sendNotification(
      static_cast<std::unique_ptr<v8_inspector::StringBuffer>>(message));
}
void v8_inspector__V8Inspector__Channel__flushProtocolNotifications(
    v8_inspector::V8Inspector::Channel* self) {
  self->flushProtocolNotifications();
}

void v8_inspector__V8InspectorClient__BASE__runMessageLoopOnPause(
    v8_inspector::V8InspectorClient* self, int contextGroupId);
void v8_inspector__V8InspectorClient__BASE__quitMessageLoopOnPause(
    v8_inspector::V8InspectorClient* self);
void v8_inspector__V8InspectorClient__BASE__runIfWaitingForDebugger(
    v8_inspector::V8InspectorClient* self, int contextGroupId);
void v8_inspector__V8InspectorClient__BASE__consoleAPIMessage(
    v8_inspector::V8InspectorClient* self, int contextGroupId,
    v8::Isolate::MessageErrorLevel level,
    const v8_inspector::StringView& message,
    const v8_inspector::StringView& url, unsigned lineNumber,
    unsigned columnNumber, v8_inspector::V8StackTrace* stackTrace);
}  // extern "C"

struct v8_inspector__V8InspectorClient__BASE
    : public v8_inspector::V8InspectorClient {
  using v8_inspector::V8InspectorClient::V8InspectorClient;

  void runMessageLoopOnPause(int contextGroupId) override {
    v8_inspector__V8InspectorClient__BASE__runMessageLoopOnPause(
        this, contextGroupId);
  }
  void quitMessageLoopOnPause() override {
    v8_inspector__V8InspectorClient__BASE__quitMessageLoopOnPause(this);
  }
  void runIfWaitingForDebugger(int contextGroupId) override {
    v8_inspector__V8InspectorClient__BASE__runIfWaitingForDebugger(
        this, contextGroupId);
  }
  void consoleAPIMessage(int contextGroupId,
                         v8::Isolate::MessageErrorLevel level,
                         const v8_inspector::StringView& message,
                         const v8_inspector::StringView& url,
                         unsigned lineNumber, unsigned columnNumber,
                         v8_inspector::V8StackTrace* stackTrace) override {
    v8_inspector__V8InspectorClient__BASE__consoleAPIMessage(
        this, contextGroupId, level, message, url, lineNumber, columnNumber,
        stackTrace);
  }
};

extern "C" {
void v8_inspector__V8InspectorClient__BASE__CONSTRUCT(
    uninit_t<v8_inspector__V8InspectorClient__BASE>* buf) {
  construct_in_place<v8_inspector__V8InspectorClient__BASE>(buf);
}

void v8_inspector__V8InspectorClient__runMessageLoopOnPause(
    v8_inspector::V8InspectorClient* self, int contextGroupId) {
  self->runMessageLoopOnPause(contextGroupId);
}
void v8_inspector__V8InspectorClient__quitMessageLoopOnPause(
    v8_inspector::V8InspectorClient* self) {
  self->quitMessageLoopOnPause();
}
void v8_inspector__V8InspectorClient__runIfWaitingForDebugger(
    v8_inspector::V8InspectorClient* self, int contextGroupId) {
  self->runIfWaitingForDebugger(contextGroupId);
}

void v8_inspector__V8InspectorClient__consoleAPIMessage(
    v8_inspector::V8InspectorClient* self, int contextGroupId,
    v8::Isolate::MessageErrorLevel level,
    const v8_inspector::StringView& message,
    const v8_inspector::StringView& url, unsigned lineNumber,
    unsigned columnNumber, v8_inspector::V8StackTrace* stackTrace) {
  self->consoleAPIMessage(contextGroupId, level, message, url, lineNumber,
                          columnNumber, stackTrace);
}

void v8_inspector__StringBuffer__DELETE(v8_inspector::StringBuffer* self) {
  delete self;
}

three_pointers_t v8_inspector__StringBuffer__string(
    const v8_inspector::StringBuffer& self) {
  return make_pod<three_pointers_t>(self.string());
}

v8_inspector::StringBuffer* v8_inspector__StringBuffer__create(
    v8_inspector::StringView source) {
  return v8_inspector::StringBuffer::create(source).release();
}

int v8__Location__GetLineNumber(v8::Location* self) {
  return self->GetLineNumber();
}

int v8__Location__GetColumnNumber(v8::Location* self) {
  return self->GetColumnNumber();
}

v8::Module::Status v8__Module__GetStatus(const v8::Module& self) {
  return self.GetStatus();
}

const v8::Value* v8__Module__GetException(const v8::Module& self) {
  return local_to_ptr(self.GetException());
}

int v8__Module__GetModuleRequestsLength(const v8::Module& self) {
  return self.GetModuleRequestsLength();
}

const v8::String* v8__Module__GetModuleRequest(const v8::Module& self, int i) {
  return local_to_ptr(self.GetModuleRequest(i));
}

void v8__Module__GetModuleRequestLocation(const v8::Module& self, int i,
                                          v8::Location* out) {
  *out = self.GetModuleRequestLocation(i);
}

const v8::Value* v8__Module__GetModuleNamespace(const v8::Module& self) {
  return local_to_ptr(ptr_to_local(&self)->GetModuleNamespace());
}

int v8__Module__GetIdentityHash(const v8::Module& self) {
  return self.GetIdentityHash();
}

MaybeBool v8__Module__InstantiateModule(const v8::Module& self,
                                        const v8::Context& context,
                                        v8::Module::ResolveCallback cb) {
  return maybe_to_maybe_bool(
      ptr_to_local(&self)->InstantiateModule(ptr_to_local(&context), cb));
}

const v8::Value* v8__Module__Evaluate(const v8::Module& self,
                                      const v8::Context& context) {
  return maybe_local_to_ptr(
      ptr_to_local(&self)->Evaluate(ptr_to_local(&context)));
}

using HeapSnapshotCallback = bool (*)(void*, const char*, size_t);

void v8__HeapProfiler__TakeHeapSnapshot(v8::Isolate* isolate,
                                        HeapSnapshotCallback callback,
                                        void* arg) {
  struct OutputStream : public v8::OutputStream {
    OutputStream(HeapSnapshotCallback callback, void* arg)
        : callback_(callback), arg_(arg) {}
    void EndOfStream() override {
      static_cast<void>(callback_(arg_, nullptr, 0));
    }
    v8::OutputStream::WriteResult WriteAsciiChunk(char* data,
                                                  int size) override {
      assert(size >= 0);  // Can never be < 0 barring bugs in V8.
      if (callback_(arg_, data, static_cast<size_t>(size)))
        return v8::OutputStream::kContinue;
      return v8::OutputStream::kAbort;
    }
    HeapSnapshotCallback const callback_;
    void* const arg_;
  };

  const v8::HeapSnapshot* snapshot =
      isolate->GetHeapProfiler()->TakeHeapSnapshot();
  if (snapshot == nullptr) return;  // Snapshotting failed, probably OOM.
  OutputStream stream(callback, arg);
  snapshot->Serialize(&stream);
  // We don't want to call HeapProfiler::DeleteAllHeapSnapshots() because that
  // invalidates snapshots we don't own. The const_cast hack has been in use
  // in node-heapdump for the last 8 years and I think there is a pretty
  // good chance it'll keep working for 8 more.
  const_cast<v8::HeapSnapshot*>(snapshot)->Delete();
}

// This is necessary for v8__internal__GetIsolateFromHeapObject() to be
// reliable enough for our purposes.
#if !(defined V8_SHARED_RO_HEAP or defined V8_COMPRESS_POINTERS)
#error V8 must be built with either the 'v8_enable_pointer_compression' or \
'v8_enable_shared_ro_heap' feature enabled.
#endif

v8::Isolate* v8__internal__GetIsolateFromHeapObject(const v8::Data& location) {
  auto address = *reinterpret_cast<const v8::internal::Address*>(&location);
  auto maybe_object = v8::internal::MaybeObject(address);
  if (maybe_object.IsSmi() || maybe_object.IsCleared()) {
    return nullptr;
  }
  auto heap_object = maybe_object.GetHeapObject();
  v8::internal::Isolate* isolate;
  if (!v8::internal::GetIsolateFromHeapObject(heap_object, &isolate)) {
    return nullptr;
  }
  return reinterpret_cast<v8::Isolate*>(isolate);
}

}  // extern "C"
