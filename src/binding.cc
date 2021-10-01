// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.
#include <cassert>
#include <cstdint>
#include <iostream>

#include "support.h"
#include "v8/include/libplatform/libplatform.h"
#include "v8/include/v8-fast-api-calls.h"
#include "v8/include/v8-inspector.h"
#include "v8/include/v8-platform.h"
#include "v8/include/v8-profiler.h"
#include "v8/include/v8.h"
#include "v8/src/execution/isolate-utils-inl.h"
#include "v8/src/execution/isolate-utils.h"
#include "v8/src/flags/flags.h"
#include "v8/src/objects/objects-inl.h"
#include "v8/src/objects/objects.h"
#include "v8/src/objects/smi.h"

using namespace support;

static_assert(sizeof(two_pointers_t) ==
                  sizeof(std::shared_ptr<v8::BackingStore>),
              "std::shared_ptr<v8::BackingStore> size mismatch");

static_assert(sizeof(v8::ScriptOrigin) <= sizeof(size_t) * 8,
              "ScriptOrigin size mismatch");

static_assert(sizeof(v8::HandleScope) == sizeof(size_t) * 3,
              "HandleScope size mismatch");

static_assert(sizeof(v8::EscapableHandleScope) == sizeof(size_t) * 4,
              "EscapableHandleScope size mismatch");

static_assert(sizeof(v8::PromiseRejectMessage) == sizeof(size_t) * 3,
              "PromiseRejectMessage size mismatch");

static_assert(sizeof(v8::Locker) == sizeof(size_t) * 2, "Locker size mismatch");

static_assert(sizeof(v8::ScriptCompiler::Source) <= sizeof(size_t) * 8,
              "Source size mismatch");

static_assert(sizeof(v8::FunctionCallbackInfo<v8::Value>) == sizeof(size_t) * 3,
              "FunctionCallbackInfo size mismatch");

static_assert(sizeof(v8::PropertyCallbackInfo<v8::Value>) == sizeof(size_t) * 1,
              "PropertyCallbackInfo size mismatch");

static_assert(sizeof(v8::ReturnValue<v8::Value>) == sizeof(size_t) * 1,
              "ReturnValue size mismatch");

static_assert(sizeof(v8::TryCatch) == sizeof(size_t) * 6,
              "TryCatch size mismatch");

static_assert(sizeof(v8::Location) == sizeof(int) * 2,
              "Location size mismatch");

static_assert(sizeof(v8::SnapshotCreator) == sizeof(size_t) * 1,
              "SnapshotCreator size mismatch");

static_assert(sizeof(v8::CFunction) == sizeof(size_t) * 2,
              "CFunction size mismatch");

static_assert(sizeof(three_pointers_t) == sizeof(v8_inspector::StringView),
              "StringView size mismatch");

#if INTPTR_MAX == INT64_MAX  // 64-bit platforms
static_assert(sizeof(v8::ScriptCompiler::CachedData) == 24,
              "CachedData size mismatch");
static_assert(offsetof(v8::ScriptCompiler::CachedData, data) == 0,
              "CachedData.data offset mismatch");
static_assert(offsetof(v8::ScriptCompiler::CachedData, length) == 8,
              "CachedData.length offset mismatch");
static_assert(offsetof(v8::ScriptCompiler::CachedData, rejected) == 12,
              "CachedData.rejected offset mismatch");
static_assert(offsetof(v8::ScriptCompiler::CachedData, buffer_policy) == 16,
              "CachedData.buffer_policy offset mismatch");
#else
static_assert(sizeof(v8::ScriptCompiler::CachedData) == 16,
              "CachedData size mismatch");
static_assert(offsetof(v8::ScriptCompiler::CachedData, data) == 0,
              "CachedData.data offset mismatch");
static_assert(offsetof(v8::ScriptCompiler::CachedData, length) == 4,
              "CachedData.length offset mismatch");
static_assert(offsetof(v8::ScriptCompiler::CachedData, rejected) == 8,
              "CachedData.rejected offset mismatch");
static_assert(offsetof(v8::ScriptCompiler::CachedData, buffer_policy) == 12,
              "CachedData.buffer_policy offset mismatch");
#endif

enum InternalSlots {
  kSlotDynamicImport = 0,
  kNumInternalSlots,
};
#define SLOT_NUM_EXTERNAL(isolate) \
  (isolate->GetNumberOfDataSlots() - kNumInternalSlots)
#define SLOT_INTERNAL(isolate, slot) \
  (isolate->GetNumberOfDataSlots() - 1 - slot)

// This is an extern C calling convention compatible version of
// v8::HostImportModuleDynamicallyWithImportAssertionsCallback
typedef v8::Promise* (
    *v8__HostImportModuleDynamicallyWithImportAssertionsCallback)(
    v8::Local<v8::Context> context, v8::Local<v8::ScriptOrModule> referrer,
    v8::Local<v8::String> specifier,
    v8::Local<v8::FixedArray> import_assertions);

v8::MaybeLocal<v8::Promise>
HostImportModuleDynamicallyWithImportAssertionsCallback(
    v8::Local<v8::Context> context, v8::Local<v8::ScriptOrModule> referrer,
    v8::Local<v8::String> specifier,
    v8::Local<v8::FixedArray> import_assertions) {
  auto* isolate = context->GetIsolate();
  void* d = isolate->GetData(SLOT_INTERNAL(isolate, kSlotDynamicImport));
  auto* callback = reinterpret_cast<
      v8__HostImportModuleDynamicallyWithImportAssertionsCallback>(d);
  assert(callback != nullptr);
  auto* promise_ptr = callback(context, referrer, specifier, import_assertions);
  if (promise_ptr == nullptr) {
    return v8::MaybeLocal<v8::Promise>();
  } else {
    return v8::MaybeLocal<v8::Promise>(ptr_to_local(promise_ptr));
  }
}

extern "C" {
void v8__V8__SetFlagsFromCommandLine(int* argc, char** argv,
                                     const char* usage) {
  namespace i = v8::internal;
  using HelpOptions = i::FlagList::HelpOptions;
  HelpOptions help_options = HelpOptions(HelpOptions::kExit, usage);
  i::FlagList::SetFlagsFromCommandLine(argc, argv, true, help_options);
}

void v8__V8__SetFlagsFromString(const char* flags, size_t length) {
  v8::V8::SetFlagsFromString(flags, length);
}

void v8__V8__SetEntropySource(v8::EntropySource callback) {
  v8::V8::SetEntropySource(callback);
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

void v8__Isolate__ClearKeptObjects(v8::Isolate* isolate) {
  isolate->ClearKeptObjects();
}

void v8__Isolate__LowMemoryNotification(v8::Isolate* isolate) {
  isolate->LowMemoryNotification();
}

void v8__Isolate__GetHeapStatistics(v8::Isolate* isolate,
                                    v8::HeapStatistics* s) {
  isolate->GetHeapStatistics(s);
}

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

const v8::Data* v8__Isolate__GetDataFromSnapshotOnce(v8::Isolate* isolate,
                                                     size_t index) {
  return maybe_local_to_ptr(isolate->GetDataFromSnapshotOnce<v8::Data>(index));
}

v8::MicrotasksPolicy v8__Isolate__GetMicrotasksPolicy(
    const v8::Isolate* isolate) {
  return isolate->GetMicrotasksPolicy();
}

void v8__Isolate__SetMicrotasksPolicy(v8::Isolate* isolate,
                                      v8::MicrotasksPolicy policy) {
  static_assert(0 == static_cast<uint32_t>(v8::MicrotasksPolicy::kExplicit),
                "v8::MicrotasksPolicy::kExplicit mismatch");
  static_assert(1 == static_cast<uint32_t>(v8::MicrotasksPolicy::kScoped),
                "v8::MicrotasksPolicy::kScoped mismatch");
  static_assert(2 == static_cast<uint32_t>(v8::MicrotasksPolicy::kAuto),
                "v8::MicrotasksPolicy::kAuto mismatch");
  isolate->SetMicrotasksPolicy(policy);
}

void v8__Isolate__PerformMicrotaskCheckpoint(v8::Isolate* isolate) {
  isolate->PerformMicrotaskCheckpoint();
}

void v8__Isolate__EnqueueMicrotask(v8::Isolate* isolate,
                                   const v8::Function& function) {
  isolate->EnqueueMicrotask(ptr_to_local(&function));
}

void v8__Isolate__RequestInterrupt(v8::Isolate* isolate,
                                   v8::InterruptCallback callback, void* data) {
  isolate->RequestInterrupt(callback, data);
}

void v8__Isolate__SetPrepareStackTraceCallback(
    v8::Isolate* isolate, v8::PrepareStackTraceCallback callback) {
  isolate->SetPrepareStackTraceCallback(callback);
}

void v8__Isolate__SetPromiseHook(v8::Isolate* isolate, v8::PromiseHook hook) {
  isolate->SetPromiseHook(hook);
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
    v8::Isolate* isolate,
    v8__HostImportModuleDynamicallyWithImportAssertionsCallback callback) {
  isolate->SetData(SLOT_INTERNAL(isolate, kSlotDynamicImport),
                   reinterpret_cast<void*>(callback));
  isolate->SetHostImportModuleDynamicallyCallback(
      HostImportModuleDynamicallyWithImportAssertionsCallback);
}

bool v8__Isolate__AddMessageListener(v8::Isolate* isolate,
                                     v8::MessageCallback callback) {
  return isolate->AddMessageListener(callback);
}

void v8__Isolate__AddNearHeapLimitCallback(v8::Isolate* isolate,
                                           v8::NearHeapLimitCallback callback,
                                           void* data) {
  isolate->AddNearHeapLimitCallback(callback, data);
}

void v8__Isolate__RemoveNearHeapLimitCallback(
    v8::Isolate* isolate, v8::NearHeapLimitCallback callback,
    size_t heap_limit) {
  isolate->RemoveNearHeapLimitCallback(callback, heap_limit);
}

void v8__Isolate__SetOOMErrorHandler(v8::Isolate* isolate,
                                     v8::OOMErrorCallback callback) {
  isolate->SetOOMErrorHandler(callback);
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

void v8__Isolate__SetAllowAtomicsWait(v8::Isolate* isolate, bool allow) {
  isolate->SetAllowAtomicsWait(allow);
}

void v8__Isolate__SetWasmStreamingCallback(v8::Isolate* isolate,
                                           v8::WasmStreamingCallback callback) {
  isolate->SetWasmStreamingCallback(callback);
}

bool v8__Isolate__HasPendingBackgroundTasks(v8::Isolate* isolate) {
  return isolate->HasPendingBackgroundTasks();
}

void v8__Isolate__CreateParams__CONSTRUCT(
    uninit_t<v8::Isolate::CreateParams>* buf) {
  construct_in_place<v8::Isolate::CreateParams>(buf);
}

size_t v8__Isolate__CreateParams__SIZEOF() {
  return sizeof(v8::Isolate::CreateParams);
}

void v8__ResourceConstraints__ConfigureDefaultsFromHeapSize(
    v8::ResourceConstraints* constraints, size_t initial_heap_size_in_bytes,
    size_t maximum_heap_size_in_bytes) {
  constraints->ConfigureDefaultsFromHeapSize(initial_heap_size_in_bytes,
                                             maximum_heap_size_in_bytes);
}

void v8__HandleScope__CONSTRUCT(uninit_t<v8::HandleScope>* buf,
                                v8::Isolate* isolate) {
  construct_in_place<v8::HandleScope>(buf, isolate);
}

void v8__HandleScope__DESTRUCT(v8::HandleScope* self) { self->~HandleScope(); }

const v8::Data* v8__Local__New(v8::Isolate* isolate, const v8::Data& other) {
  return local_to_ptr(v8::Local<v8::Data>::New(isolate, ptr_to_local(&other)));
}

const v8::Data* v8__Global__New(v8::Isolate* isolate, const v8::Data& other) {
  // We have to use `std::move()` here because v8 disables the copy constructor
  // for class `v8::Global`.
  auto global = v8::Global<v8::Data>(isolate, ptr_to_local(&other));
  return make_pod<v8::Data*>(std::move(global));
}

void v8__Global__Reset(const v8::Data* data) {
  auto global = ptr_to_global(data);
  global.Reset();
}

void v8__ScriptCompiler__Source__CONSTRUCT(
    uninit_t<v8::ScriptCompiler::Source>* buf, const v8::String& source_string,
    const v8::ScriptOrigin* origin,
    v8::ScriptCompiler::CachedData* cached_data) {
  if (origin) {
    construct_in_place<v8::ScriptCompiler::Source>(
        buf, ptr_to_local(&source_string), *origin, cached_data);
  } else {
    construct_in_place<v8::ScriptCompiler::Source>(
        buf, ptr_to_local(&source_string), cached_data);
  }
}

void v8__ScriptCompiler__Source__DESTRUCT(v8::ScriptCompiler::Source* self) {
  self->~Source();
}

v8::ScriptCompiler::CachedData* v8__ScriptCompiler__CachedData__NEW(
    const uint8_t* data, int length) {
  return new v8::ScriptCompiler::CachedData(
      data, length, v8::ScriptCompiler::CachedData::BufferNotOwned);
}

void v8__ScriptCompiler__CachedData__DELETE(
    v8::ScriptCompiler::CachedData* self) {
  delete self;
}

const v8::ScriptCompiler::CachedData* v8__ScriptCompiler__Source__GetCachedData(
    const v8::ScriptCompiler::Source* source) {
  return source->GetCachedData();
}

const v8::Module* v8__ScriptCompiler__CompileModule(
    v8::Isolate* isolate, v8::ScriptCompiler::Source* source,
    v8::ScriptCompiler::CompileOptions options,
    v8::ScriptCompiler::NoCacheReason no_cache_reason) {
  v8::MaybeLocal<v8::Module> maybe_local = v8::ScriptCompiler::CompileModule(
      isolate, source, options, no_cache_reason);
  return maybe_local_to_ptr(maybe_local);
}

const v8::Script* v8__ScriptCompiler__Compile(
    const v8::Context* context, v8::ScriptCompiler::Source* source,
    v8::ScriptCompiler::CompileOptions options,
    v8::ScriptCompiler::NoCacheReason no_cache_reason) {
  v8::MaybeLocal<v8::Script> maybe_local = v8::ScriptCompiler::Compile(
      ptr_to_local(context), source, options, no_cache_reason);
  return maybe_local_to_ptr(maybe_local);
}

const v8::Function* v8__ScriptCompiler__CompileFunctionInContext(
    const v8::Context* context, v8::ScriptCompiler::Source* source,
    size_t arguments_count, const v8::String** arguments,
    size_t context_extensions_count, const v8::Object** context_extensions,
    v8::ScriptCompiler::CompileOptions options,
    v8::ScriptCompiler::NoCacheReason no_cache_reason) {
  return maybe_local_to_ptr(v8::ScriptCompiler::CompileFunctionInContext(
      ptr_to_local(context), source, arguments_count,
      reinterpret_cast<v8::Local<v8::String>*>(arguments),
      context_extensions_count,
      reinterpret_cast<v8::Local<v8::Object>*>(context_extensions), options,
      no_cache_reason, nullptr));
}

const v8::UnboundScript* v8__ScriptCompiler__CompileUnboundScript(
    v8::Isolate* isolate, v8::ScriptCompiler::Source* source,
    v8::ScriptCompiler::CompileOptions options,
    v8::ScriptCompiler::NoCacheReason no_cache_reason) {
  v8::MaybeLocal<v8::UnboundScript> maybe_local =
      v8::ScriptCompiler::CompileUnboundScript(isolate, source, options,
                                               no_cache_reason);
  return maybe_local_to_ptr(maybe_local);
}

bool v8__Data__EQ(const v8::Data& self, const v8::Data& other) {
  return ptr_to_local(&self) == ptr_to_local(&other);
}

bool v8__Data__IsValue(const v8::Data& self) { return self.IsValue(); }

bool v8__Data__IsModule(const v8::Data& self) { return self.IsModule(); }

bool v8__Data__IsPrivate(const v8::Data& self) { return self.IsPrivate(); }

bool v8__Data__IsObjectTemplate(const v8::Data& self) {
  return self.IsObjectTemplate();
}

bool v8__Data__IsFunctionTemplate(const v8::Data& self) {
  return self.IsFunctionTemplate();
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

const v8::Boolean* v8__Value__ToBoolean(const v8::Value& self,
                                        v8::Isolate* isolate) {
  return local_to_ptr(self.ToBoolean(isolate));
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

bool v8__Value__BooleanValue(const v8::Value& self, v8::Isolate* isolate) {
  return self.BooleanValue(isolate);
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

int v8__FixedArray__Length(const v8::FixedArray& self) { return self.Length(); }

const v8::Data* v8__FixedArray__Get(const v8::FixedArray& self,
                                    const v8::Context& context, int index) {
  return local_to_ptr(ptr_to_local(&self)->Get(ptr_to_local(&context), index));
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

void v8__ArrayBuffer__Detach(const v8::ArrayBuffer& self) {
  ptr_to_local(&self)->Detach();
}

bool v8__ArrayBuffer__IsDetachable(const v8::ArrayBuffer& self) {
  return ptr_to_local(&self)->IsDetachable();
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

int v8__Name__GetIdentityHash(const v8::Name& self) {
  return ptr_to_local(&self)->GetIdentityHash();
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

const v8::String* v8__String__NewFromOneByte(v8::Isolate* isolate,
                                             const uint8_t* data,
                                             v8::NewStringType new_type,
                                             int length) {
  return maybe_local_to_ptr(
      v8::String::NewFromOneByte(isolate, data, new_type, length));
}

const v8::String* v8__String__NewFromTwoByte(v8::Isolate* isolate,
                                             const uint16_t* data,
                                             v8::NewStringType new_type,
                                             int length) {
  return maybe_local_to_ptr(
      v8::String::NewFromTwoByte(isolate, data, new_type, length));
}

int v8__String__Length(const v8::String& self) { return self.Length(); }

int v8__String__Utf8Length(const v8::String& self, v8::Isolate* isolate) {
  return self.Utf8Length(isolate);
}

int v8__String__Write(const v8::String& self, v8::Isolate* isolate,
                      uint16_t* buffer, int start, int length, int options) {
  return self.Write(isolate, buffer, start, length, options);
}

int v8__String__WriteOneByte(const v8::String& self, v8::Isolate* isolate,
                             uint8_t* buffer, int start, int length,
                             int options) {
  return self.WriteOneByte(isolate, buffer, start, length, options);
}

int v8__String__WriteUtf8(const v8::String& self, v8::Isolate* isolate,
                          char* buffer, int length, int* nchars_ref,
                          int options) {
  return self.WriteUtf8(isolate, buffer, length, nchars_ref, options);
}

class ExternalStaticOneByteStringResource
    : public v8::String::ExternalOneByteStringResource {
 public:
  ExternalStaticOneByteStringResource(const char* data, int length)
      : _data(data), _length(length) {}
  const char* data() const override { return _data; }
  size_t length() const override { return _length; }

 private:
  const char* _data;
  const int _length;
};

const v8::String* v8__String__NewExternalOneByteStatic(v8::Isolate* isolate,
                                                       const char* data,
                                                       int length) {
  return maybe_local_to_ptr(v8::String::NewExternalOneByte(
      isolate, new ExternalStaticOneByteStringResource(data, length)));
}

class ExternalStaticStringResource : public v8::String::ExternalStringResource {
 public:
  ExternalStaticStringResource(const uint16_t* data, int length)
      : _data(data), _length(length) {}
  const uint16_t* data() const override { return _data; }
  size_t length() const override { return _length; }

 private:
  const uint16_t* _data;
  const int _length;
};

const v8::String* v8__String__NewExternalTwoByteStatic(v8::Isolate* isolate,
                                                       const uint16_t* data,
                                                       int length) {
  return maybe_local_to_ptr(v8::String::NewExternalTwoByte(
      isolate, new ExternalStaticStringResource(data, length)));
}

bool v8__String__IsExternal(const v8::String& self) {
  return self.IsExternal();
}
bool v8__String__IsExternalOneByte(const v8::String& self) {
  return self.IsExternalOneByte();
}
bool v8__String__IsExternalTwoByte(const v8::String& self) {
  return self.IsExternalTwoByte();
}
bool v8__String__IsOneByte(const v8::String& self) { return self.IsOneByte(); }
bool v8__String__ContainsOnlyOneByte(const v8::String& self) {
  return self.ContainsOnlyOneByte();
}

const v8::Symbol* v8__Symbol__New(v8::Isolate* isolate,
                                  const v8::String& description) {
  return local_to_ptr(v8::Symbol::New(isolate, ptr_to_local(&description)));
}

const v8::Symbol* v8__Symbol__For(v8::Isolate* isolate,
                                  const v8::String& description) {
  return local_to_ptr(v8::Symbol::For(isolate, ptr_to_local(&description)));
}

const v8::Symbol* v8__Symbol__ForApi(v8::Isolate* isolate,
                                     const v8::String& description) {
  return local_to_ptr(v8::Symbol::ForApi(isolate, ptr_to_local(&description)));
}

#define V(NAME)                                                   \
  const v8::Symbol* v8__Symbol__Get##NAME(v8::Isolate* isolate) { \
    return local_to_ptr(v8::Symbol::Get##NAME(isolate));          \
  }

V(AsyncIterator)
V(HasInstance)
V(IsConcatSpreadable)
V(Iterator)
V(Match)
V(Replace)
V(Search)
V(Split)
V(ToPrimitive)
V(ToStringTag)
V(Unscopables)
#undef V

const v8::Value* v8__Symbol__Description(const v8::Symbol& self) {
  return local_to_ptr(ptr_to_local(&self)->Description());
}

const v8::Private* v8__Private__New(v8::Isolate* isolate,
                                    const v8::String& name) {
  return local_to_ptr(v8::Private::New(isolate, ptr_to_local(&name)));
}

const v8::Private* v8__Private__ForApi(v8::Isolate* isolate,
                                       const v8::String& name) {
  return local_to_ptr(v8::Private::ForApi(isolate, ptr_to_local(&name)));
}

const v8::Value* v8__Private__Name(const v8::Private& self) {
  return local_to_ptr(ptr_to_local(&self)->Name());
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

int v8__ObjectTemplate__InternalFieldCount(const v8::ObjectTemplate& self) {
  return ptr_to_local(&self)->InternalFieldCount();
}

void v8__ObjectTemplate__SetInternalFieldCount(const v8::ObjectTemplate& self,
                                               int value) {
  ptr_to_local(&self)->SetInternalFieldCount(value);
}

void v8__ObjectTemplate__SetAccessor(const v8::ObjectTemplate& self,
                                     const v8::Name& key,
                                     v8::AccessorNameGetterCallback getter) {
  ptr_to_local(&self)->SetAccessor(ptr_to_local(&key), getter);
}

void v8__ObjectTemplate__SetAccessorWithSetter(
    const v8::ObjectTemplate& self, const v8::Name& key,
    v8::AccessorNameGetterCallback getter,
    v8::AccessorNameSetterCallback setter) {
  ptr_to_local(&self)->SetAccessor(ptr_to_local(&key), getter, setter);
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

MaybeBool v8__Object__SetAccessorWithSetter(
    const v8::Object& self, const v8::Context& context, const v8::Name& key,
    v8::AccessorNameGetterCallback getter,
    v8::AccessorNameSetterCallback setter) {
  return maybe_to_maybe_bool(ptr_to_local(&self)->SetAccessor(
      ptr_to_local(&context), ptr_to_local(&key), getter, setter));
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

MaybeBool v8__Object__Has(const v8::Object& self, const v8::Context& context,
                          const v8::Value& key) {
  return maybe_to_maybe_bool(
      ptr_to_local(&self)->Has(ptr_to_local(&context), ptr_to_local(&key)));
}

MaybeBool v8__Object__HasIndex(const v8::Object& self,
                               const v8::Context& context, uint32_t index) {
  return maybe_to_maybe_bool(
      ptr_to_local(&self)->Has(ptr_to_local(&context), index));
}

MaybeBool v8__Object__Delete(const v8::Object& self, const v8::Context& context,
                             const v8::Value& key) {
  return maybe_to_maybe_bool(
      ptr_to_local(&self)->Delete(ptr_to_local(&context), ptr_to_local(&key)));
}

MaybeBool v8__Object__DeleteIndex(const v8::Object& self,
                                  const v8::Context& context, uint32_t index) {
  return maybe_to_maybe_bool(
      ptr_to_local(&self)->Delete(ptr_to_local(&context), index));
}

int v8__Object__InternalFieldCount(const v8::Object& self) {
  return ptr_to_local(&self)->InternalFieldCount();
}

const v8::Value* v8__Object__GetInternalField(const v8::Object& self,
                                              int index) {
  return local_to_ptr(ptr_to_local(&self)->GetInternalField(index));
}

void v8__Object__SetInternalField(const v8::Object& self, int index,
                                  const v8::Value& value) {
  ptr_to_local(&self)->SetInternalField(index, ptr_to_local(&value));
}

const v8::Value* v8__Object__GetPrivate(const v8::Object& self,
                                        const v8::Context& context,
                                        const v8::Private& key) {
  return maybe_local_to_ptr(ptr_to_local(&self)->GetPrivate(
      ptr_to_local(&context), ptr_to_local(&key)));
}

MaybeBool v8__Object__SetPrivate(const v8::Object& self,
                                 const v8::Context& context,
                                 const v8::Private& key,
                                 const v8::Value& value) {
  return maybe_to_maybe_bool(ptr_to_local(&self)->SetPrivate(
      ptr_to_local(&context), ptr_to_local(&key), ptr_to_local(&value)));
}

MaybeBool v8__Object__DeletePrivate(const v8::Object& self,
                                    const v8::Context& context,
                                    const v8::Private& key) {
  return maybe_to_maybe_bool(ptr_to_local(&self)->DeletePrivate(
      ptr_to_local(&context), ptr_to_local(&key)));
}

MaybeBool v8__Object__HasPrivate(const v8::Object& self,
                                 const v8::Context& context,
                                 const v8::Private& key) {
  return maybe_to_maybe_bool(ptr_to_local(&self)->HasPrivate(
      ptr_to_local(&context), ptr_to_local(&key)));
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

const v8::Date* v8__Date__New(const v8::Context& context, double time) {
  // v8::Date::New() is kind of weird in that it returns a v8::Value,
  // not a v8::Date, even though the object is always a Date object.
  // Let's paper over that quirk here.
  v8::MaybeLocal<v8::Date> maybe_date;

  v8::Local<v8::Value> value;
  if (v8::Date::New(ptr_to_local(&context), time).ToLocal(&value)) {
    assert(value->IsDate());
    maybe_date = value.As<v8::Date>();
  }

  return maybe_local_to_ptr(maybe_date);
}

double v8__Date__ValueOf(const v8::Date& self) { return self.ValueOf(); }

const v8::External* v8__External__New(v8::Isolate* isolate, void* value) {
  return local_to_ptr(v8::External::New(isolate, value));
}

void* v8__External__Value(const v8::External& self) { return self.Value(); }

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

const v8::BigInt* v8__BigInt__New(v8::Isolate* isolate, int64_t value) {
  return local_to_ptr(v8::BigInt::New(isolate, value));
}

const v8::BigInt* v8__BigInt__NewFromUnsigned(v8::Isolate* isolate,
                                              uint64_t value) {
  return local_to_ptr(v8::BigInt::NewFromUnsigned(isolate, value));
}

const v8::BigInt* v8__BigInt__NewFromWords(const v8::Context& context,
                                           int sign_bit, int word_count,
                                           const uint64_t words[]) {
  return maybe_local_to_ptr(v8::BigInt::NewFromWords(
      ptr_to_local(&context), sign_bit, word_count, words));
}

uint64_t v8__BigInt__Uint64Value(const v8::BigInt& self, bool* lossless) {
  return ptr_to_local(&self)->Uint64Value(lossless);
}

int64_t v8__BigInt__Int64Value(const v8::BigInt& self, bool* lossless) {
  return ptr_to_local(&self)->Int64Value(lossless);
}

int v8__BigInt__WordCount(const v8::BigInt& self) {
  return ptr_to_local(&self)->WordCount();
}

void v8__BigInt__ToWordsArray(const v8::BigInt& self, int* sign_bit,
                              int* word_count, uint64_t words[]) {
  ptr_to_local(&self)->ToWordsArray(sign_bit, word_count, words);
}

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

struct RustAllocatorVtable {
  void* (*allocate)(void* handle, size_t length);
  void* (*allocate_uninitialized)(void* handle, size_t length);
  void (*free)(void* handle, void* data, size_t length);
  void* (*reallocate)(void* handle, void* data, size_t old_length,
                      size_t new_length);
  void (*drop)(void* handle);
};

class RustAllocator : public v8::ArrayBuffer::Allocator {
 private:
  void* handle;
  const RustAllocatorVtable* vtable;

 public:
  RustAllocator(void* handle, const RustAllocatorVtable* vtable) {
    this->handle = handle;
    this->vtable = vtable;
  }

  RustAllocator(const RustAllocator& that) = delete;
  RustAllocator(RustAllocator&& that) = delete;
  void operator=(const RustAllocator& that) = delete;
  void operator=(RustAllocator&& that) = delete;

  virtual ~RustAllocator() { vtable->drop(handle); }

  void* Allocate(size_t length) final {
    return vtable->allocate(handle, length);
  }

  void* AllocateUninitialized(size_t length) final {
    return vtable->allocate_uninitialized(handle, length);
  }

  void Free(void* data, size_t length) final {
    vtable->free(handle, data, length);
  }

  void* Reallocate(void* data, size_t old_length, size_t new_length) final {
    return vtable->reallocate(handle, data, old_length, new_length);
  }
};

v8::ArrayBuffer::Allocator* v8__ArrayBuffer__Allocator__NewDefaultAllocator() {
  return v8::ArrayBuffer::Allocator::NewDefaultAllocator();
}

v8::ArrayBuffer::Allocator* v8__ArrayBuffer__Allocator__NewRustAllocator(
    void* handle, const RustAllocatorVtable* vtable) {
  return new RustAllocator(handle, vtable);
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

bool v8__Context__EQ(const v8::Context& self, const v8::Context& other) {
  return ptr_to_local(&self) == ptr_to_local(&other);
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

const v8::Data* v8__Context__GetDataFromSnapshotOnce(v8::Context& self,
                                                     size_t index) {
  return maybe_local_to_ptr(
      ptr_to_local(&self)->GetDataFromSnapshotOnce<v8::Data>(index));
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

const v8::Function* v8__Function__New(
    const v8::Context& context, v8::FunctionCallback callback,
    const v8::Value* data_or_null, int length,
    v8::ConstructorBehavior constructor_behavior,
    v8::SideEffectType side_effect_type) {
  return maybe_local_to_ptr(v8::Function::New(
      ptr_to_local(&context), callback, ptr_to_local(data_or_null), length,
      constructor_behavior, side_effect_type));
}

const v8::Value* v8__Function__Call(const v8::Function& self,
                                    const v8::Context& context,
                                    const v8::Value& recv, int argc,
                                    const v8::Value* const argv[]) {
  return maybe_local_to_ptr(
      ptr_to_local(&self)->Call(ptr_to_local(&context), ptr_to_local(&recv),
                                argc, const_ptr_array_to_local_array(argv)));
}

const v8::Object* v8__Function__NewInstance(const v8::Function& self,
                                            const v8::Context& context,
                                            int argc,
                                            const v8::Value* const argv[]) {
  return maybe_local_to_ptr(ptr_to_local(&self)->NewInstance(
      ptr_to_local(&context), argc, const_ptr_array_to_local_array(argv)));
}

const v8::Value* v8__Function__GetName(const v8::Function& self) {
  return local_to_ptr(self.GetName());
}

void v8__Function__SetName(const v8::Function& self, const v8::String& name) {
  return ptr_to_local(&self)->SetName(ptr_to_local(&name));
}

const v8::Signature* v8__Signature__New(v8::Isolate* isolate,
                                        const v8::FunctionTemplate* templ) {
  return local_to_ptr(v8::Signature::New(isolate, ptr_to_local(templ)));
}

const v8::FunctionTemplate* v8__FunctionTemplate__New(
    v8::Isolate* isolate, v8::FunctionCallback callback,
    const v8::Value* data_or_null, const v8::Signature* signature_or_null,
    int length, v8::ConstructorBehavior constructor_behavior,
    v8::SideEffectType side_effect_type,
    const v8::CFunction* c_function_or_null) {
  return local_to_ptr(v8::FunctionTemplate::New(
      isolate, callback, ptr_to_local(data_or_null),
      ptr_to_local(signature_or_null), length, constructor_behavior,
      side_effect_type, c_function_or_null));
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

void v8__FunctionTemplate__Inherit(const v8::FunctionTemplate& self,
                                   const v8::FunctionTemplate& parent) {
  ptr_to_local(&self)->Inherit(ptr_to_local(&parent));
}

void v8__FunctionTemplate__ReadOnlyPrototype(const v8::FunctionTemplate& self) {
  ptr_to_local(&self)->ReadOnlyPrototype();
}

void v8__FunctionTemplate__RemovePrototype(const v8::FunctionTemplate& self) {
  ptr_to_local(&self)->RemovePrototype();
}

const v8::ObjectTemplate* v8__FunctionTemplate__PrototypeTemplate(
    const v8::FunctionTemplate& self) {
  return local_to_ptr(ptr_to_local(&self)->PrototypeTemplate());
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

#define V(NAME)                                                          \
  const v8::NAME* v8__##NAME##__New(const v8::ArrayBuffer& buf_ptr,      \
                                    size_t byte_offset, size_t length) { \
    return local_to_ptr(                                                 \
        v8::NAME::New(ptr_to_local(&buf_ptr), byte_offset, length));     \
  }

V(Uint8Array)
V(Uint8ClampedArray)
V(Int8Array)
V(Uint16Array)
V(Int16Array)
V(Uint32Array)
V(Int32Array)
V(Float32Array)
V(Float64Array)
V(BigUint64Array)
V(BigInt64Array)
#undef V

const v8::Script* v8__Script__Compile(const v8::Context& context,
                                      const v8::String& source,
                                      const v8::ScriptOrigin& origin) {
  return maybe_local_to_ptr(
      v8::Script::Compile(ptr_to_local(&context), ptr_to_local(&source),
                          const_cast<v8::ScriptOrigin*>(&origin)));
}

const v8::UnboundScript* v8__Script__GetUnboundScript(
    const v8::Script& script) {
  return local_to_ptr(ptr_to_local(&script)->GetUnboundScript());
}

const v8::Script* v8__UnboundScript__BindToCurrentContext(
    const v8::UnboundScript& unbound_script) {
  return local_to_ptr(ptr_to_local(&unbound_script)->BindToCurrentContext());
}

v8::ScriptCompiler::CachedData* v8__UnboundScript__CreateCodeCache(
    const v8::UnboundScript& unbound_script) {
  return v8::ScriptCompiler::CreateCodeCache(ptr_to_local(&unbound_script));
}

v8::ScriptCompiler::CachedData* v8__UnboundModuleScript__CreateCodeCache(
    const v8::UnboundModuleScript& unbound_module_script) {
  return v8::ScriptCompiler::CreateCodeCache(
      ptr_to_local(&unbound_module_script));
}

const v8::Value* v8__Script__Run(const v8::Script& script,
                                 const v8::Context& context) {
  return maybe_local_to_ptr(ptr_to_local(&script)->Run(ptr_to_local(&context)));
}

void v8__ScriptOrigin__CONSTRUCT(
    v8::Isolate* isolate, uninit_t<v8::ScriptOrigin>* buf,
    const v8::Value& resource_name, int resource_line_offset,
    int resource_column_offset, bool resource_is_shared_cross_origin,
    int script_id, const v8::Value& source_map_url, bool resource_is_opaque,
    bool is_wasm, bool is_module) {
  construct_in_place<v8::ScriptOrigin>(
      buf, isolate, ptr_to_local(&resource_name), resource_line_offset,
      resource_column_offset, resource_is_shared_cross_origin, script_id,
      ptr_to_local(&source_map_url), resource_is_opaque, is_wasm, is_module);
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

v8::Isolate* v8__SnapshotCreator__GetIsolate(const v8::SnapshotCreator& self) {
  // `v8::SnapshotCreator::GetIsolate()` is not declared as a const method, but
  // this appears to be a mistake.
  auto self_ptr = const_cast<v8::SnapshotCreator*>(&self);
  return self_ptr->GetIsolate();
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

size_t v8__SnapshotCreator__AddData_to_isolate(v8::SnapshotCreator* self,
                                               const v8::Data& data) {
  return self->AddData(ptr_to_local(&data));
}

size_t v8__SnapshotCreator__AddData_to_context(v8::SnapshotCreator* self,
                                               const v8::Context& context,
                                               const v8::Data& data) {
  return self->AddData(ptr_to_local(&context), ptr_to_local(&data));
}

v8::StartupData v8__SnapshotCreator__CreateBlob(
    v8::SnapshotCreator* self,
    v8::SnapshotCreator::FunctionCodeHandling function_code_handling) {
  return self->CreateBlob(function_code_handling);
}

v8::Platform* v8__Platform__NewDefaultPlatform(int thread_pool_size,
                                               bool idle_task_support) {
  return v8::platform::NewDefaultPlatform(
             thread_pool_size,
             idle_task_support ? v8::platform::IdleTaskSupport::kEnabled
                               : v8::platform::IdleTaskSupport::kDisabled,
             v8::platform::InProcessStackDumping::kDisabled, nullptr)
      .release();
}

v8::Platform* v8__Platform__NewSingleThreadedDefaultPlatform(
    bool idle_task_support) {
  return v8::platform::NewSingleThreadedDefaultPlatform(
             idle_task_support ? v8::platform::IdleTaskSupport::kEnabled
                               : v8::platform::IdleTaskSupport::kDisabled,
             v8::platform::InProcessStackDumping::kDisabled, nullptr)
      .release();
}

bool v8__Platform__PumpMessageLoop(v8::Platform* platform, v8::Isolate* isolate,
                                   bool wait_for_work) {
  return v8::platform::PumpMessageLoop(
      platform, isolate,
      wait_for_work ? v8::platform::MessageLoopBehavior::kWaitForWork
                    : v8::platform::MessageLoopBehavior::kDoNotWait);
}

void v8__Platform__RunIdleTasks(v8::Platform* platform, v8::Isolate* isolate,
                                double idle_time_in_seconds) {
  v8::platform::RunIdleTasks(platform, isolate, idle_time_in_seconds);
}

void v8__Platform__DELETE(v8::Platform* self) { delete self; }

two_pointers_t std__shared_ptr__v8__Platform__CONVERT__std__unique_ptr(
    v8::Platform* unique_ptr) {
  return make_pod<two_pointers_t>(std::shared_ptr<v8::Platform>(unique_ptr));
}

v8::Platform* std__shared_ptr__v8__Platform__get(
    const std::shared_ptr<v8::Platform>& ptr) {
  return ptr.get();
}

two_pointers_t std__shared_ptr__v8__Platform__COPY(
    const std::shared_ptr<v8::Platform>& ptr) {
  return make_pod<two_pointers_t>(ptr);
}

void std__shared_ptr__v8__Platform__reset(std::shared_ptr<v8::Platform>* ptr) {
  ptr->reset();
}

long std__shared_ptr__v8__Platform__use_count(
    const std::shared_ptr<v8::Platform>& ptr) {
  return ptr.use_count();
}

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

bool v8_inspector__V8InspectorSession__canDispatchMethod(
    v8_inspector::StringView method) {
  return v8_inspector::V8InspectorSession::canDispatchMethod(method);
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

int64_t v8_inspector__V8InspectorClient__BASE__generateUniqueId(
    v8_inspector::V8InspectorClient* self);
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

  int64_t generateUniqueId() override {
    return v8_inspector__V8InspectorClient__BASE__generateUniqueId(this);
  }
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

int64_t v8_inspector__V8InspectorClient__generateUniqueId(
    v8_inspector::V8InspectorClient* self) {
  return self->generateUniqueId();
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

const v8::FixedArray* v8__Module__GetModuleRequests(const v8::Module& self) {
  return local_to_ptr(self.GetModuleRequests());
}

void v8__Module__SourceOffsetToLocation(const v8::Module& self, int offset,
                                        v8::Location* out) {
  *out = self.SourceOffsetToLocation(offset);
}

const v8::Value* v8__Module__GetModuleNamespace(const v8::Module& self) {
  return local_to_ptr(ptr_to_local(&self)->GetModuleNamespace());
}

int v8__Module__GetIdentityHash(const v8::Module& self) {
  return self.GetIdentityHash();
}

int v8__Module__ScriptId(const v8::Module& self) {
  // Module::ScriptId() isn't marked const but its implementation is
  // so this const_cast is sound.
  // TODO(bnoordhuis) Open V8 CL to mark Module::ScriptId() and
  // UnboundScript::GetId() const.
  return const_cast<v8::Module&>(self).ScriptId();
}

MaybeBool v8__Module__InstantiateModule(const v8::Module& self,
                                        const v8::Context& context,
                                        v8::Module::ResolveModuleCallback cb) {
  return maybe_to_maybe_bool(
      ptr_to_local(&self)->InstantiateModule(ptr_to_local(&context), cb));
}

const v8::Value* v8__Module__Evaluate(const v8::Module& self,
                                      const v8::Context& context) {
  return maybe_local_to_ptr(
      ptr_to_local(&self)->Evaluate(ptr_to_local(&context)));
}

bool v8__Module__IsSourceTextModule(const v8::Module& self) {
  return ptr_to_local(&self)->IsSourceTextModule();
}

bool v8__Module__IsSyntheticModule(const v8::Module& self) {
  return ptr_to_local(&self)->IsSyntheticModule();
}

const v8::Module* v8__Module__CreateSyntheticModule(
    v8::Isolate* isolate, const v8::String* module_name,
    size_t export_names_len, const v8::String* export_names_raw[],
    v8::Module::SyntheticModuleEvaluationSteps evaluation_steps) {
  std::vector<v8::Local<v8::String>> export_names{};
  for (size_t i = 0; i < export_names_len; i += 1) {
    export_names.push_back(ptr_to_local(export_names_raw[i]));
  }
  return local_to_ptr(v8::Module::CreateSyntheticModule(
      isolate, ptr_to_local(module_name), export_names, evaluation_steps));
}

MaybeBool v8__Module__SetSyntheticModuleExport(const v8::Module& self,
                                               v8::Isolate* isolate,
                                               const v8::String* export_name,
                                               const v8::Value* export_value) {
  return maybe_to_maybe_bool(ptr_to_local(&self)->SetSyntheticModuleExport(
      isolate, ptr_to_local(export_name), ptr_to_local(export_value)));
}

const v8::UnboundModuleScript* v8__Module__GetUnboundModuleScript(
    const v8::Module& self) {
  return local_to_ptr(ptr_to_local(&self)->GetUnboundModuleScript());
}

const v8::String* v8__ModuleRequest__GetSpecifier(
    const v8::ModuleRequest& self) {
  return local_to_ptr(self.GetSpecifier());
}

int v8__ModuleRequest__GetSourceOffset(const v8::ModuleRequest& self) {
  return self.GetSourceOffset();
}

const v8::FixedArray* v8__ModuleRequest__GetImportAssertions(
    const v8::ModuleRequest& self) {
  return local_to_ptr(self.GetImportAssertions());
}

struct WasmStreamingSharedPtr {
  std::shared_ptr<v8::WasmStreaming> inner;
};

static_assert(sizeof(WasmStreamingSharedPtr) <= 2 * sizeof(void*),
              "std::shared_ptr<v8::WasmStreaming> size mismatch");

void v8__WasmStreaming__Unpack(v8::Isolate* isolate, const v8::Value& value,
                               WasmStreamingSharedPtr* self) {
  new (self) WasmStreamingSharedPtr();
  self->inner = v8::WasmStreaming::Unpack(isolate, ptr_to_local(&value));
}

void v8__WasmStreaming__shared_ptr_DESTRUCT(WasmStreamingSharedPtr* self) {
  self->~WasmStreamingSharedPtr();
}

void v8__WasmStreaming__OnBytesReceived(WasmStreamingSharedPtr* self,
                                        const uint8_t* data, size_t len) {
  self->inner->OnBytesReceived(data, len);
}

void v8__WasmStreaming__Finish(WasmStreamingSharedPtr* self) {
  self->inner->Finish();
}

void v8__WasmStreaming__Abort(WasmStreamingSharedPtr* self,
                              const v8::Value* exception) {
  self->inner->Abort(ptr_to_maybe_local(exception));
}

void v8__WasmStreaming__SetUrl(WasmStreamingSharedPtr* self,
                               const char* url, size_t len) {
  self->inner->SetUrl(url, len);
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
#if UINTPTR_MAX == 0xffffffffffffffff && !(defined V8_SHARED_RO_HEAP or defined V8_COMPRESS_POINTERS)
#error V8 must be built with either the 'v8_enable_pointer_compression' or \
'v8_enable_shared_ro_heap' feature enabled.
#endif

v8::Isolate* v8__internal__GetIsolateFromHeapObject(const v8::Data& data) {
  namespace i = v8::internal;
  i::Object object(reinterpret_cast<const i::Address&>(data));
  i::Isolate* isolate;
  return object.IsHeapObject() &&
                 i::GetIsolateFromHeapObject(object.GetHeapObject(), &isolate)
             ? reinterpret_cast<v8::Isolate*>(isolate)
             : nullptr;
}

int v8__internal__Object__GetHash(const v8::Data& data) {
  namespace i = v8::internal;
  i::Object object(reinterpret_cast<const i::Address&>(data));
  i::Isolate* isolate;
  int hash = object.IsHeapObject() && i::GetIsolateFromHeapObject(
                                          object.GetHeapObject(), &isolate)
                 ? object.GetOrCreateHash(isolate).value()
                 : i::Smi::ToInt(object.GetHash());
  assert(hash != 0);
  return hash;
}

void v8__HeapStatistics__CONSTRUCT(uninit_t<v8::HeapStatistics>* buf) {
  // Should be <= than its counterpart in src/isolate.rs
  static_assert(sizeof(v8::HeapStatistics) <= sizeof(uintptr_t[16]),
                "HeapStatistics mismatch");
  construct_in_place<v8::HeapStatistics>(buf);
}

// The const_cast doesn't violate const correctness, the methods
// are simple getters that don't mutate the object or global state.
#define V(name)                                                    \
  size_t v8__HeapStatistics__##name(const v8::HeapStatistics* s) { \
    return const_cast<v8::HeapStatistics*>(s)->name();             \
  }

V(total_heap_size)
V(total_heap_size_executable)
V(total_physical_size)
V(total_available_size)
V(total_global_handles_size)
V(used_global_handles_size)
V(used_heap_size)
V(heap_size_limit)
V(malloced_memory)
V(external_memory)
V(peak_malloced_memory)
V(number_of_native_contexts)
V(number_of_detached_contexts)
V(does_zap_garbage)  // Returns size_t, not bool like you'd expect.

#undef V
}  // extern "C"

// v8::ValueSerializer::Delegate

extern "C" {
void v8__ValueSerializer__Delegate__ThrowDataCloneError(
    v8::ValueSerializer::Delegate* self, v8::Local<v8::String> message);

MaybeBool v8__ValueSerializer__Delegate__WriteHostObject(
    v8::ValueSerializer::Delegate* self, v8::Isolate* isolate,
    v8::Local<v8::Object> object);

bool v8__ValueSerializer__Delegate__GetSharedArrayBufferId(
    v8::ValueSerializer::Delegate* self, v8::Isolate* isolate,
    v8::Local<v8::SharedArrayBuffer> shared_array_buffer, uint32_t* result);

bool v8__ValueSerializer__Delegate__GetWasmModuleTransferId(
    v8::ValueSerializer::Delegate* self, v8::Isolate* isolate,
    v8::Local<v8::WasmModuleObject> module, uint32_t* result);

void* v8__ValueSerializer__Delegate__ReallocateBufferMemory(
    v8::ValueSerializer::Delegate* self, void* old_buffer, size_t size,
    size_t* actual_size);

void v8__ValueSerializer__Delegate__FreeBufferMemory(
    v8::ValueSerializer::Delegate* self, void* buffer);
}

struct v8__ValueSerializer__Delegate : public v8::ValueSerializer::Delegate {
  void ThrowDataCloneError(v8::Local<v8::String> message) override {
    v8__ValueSerializer__Delegate__ThrowDataCloneError(this, message);
  }

  v8::Maybe<bool> WriteHostObject(v8::Isolate* isolate,
                                  v8::Local<v8::Object> object) override {
    return maybe_bool_to_maybe(
        v8__ValueSerializer__Delegate__WriteHostObject(this, isolate, object));
  }

  v8::Maybe<uint32_t> GetSharedArrayBufferId(
      v8::Isolate* isolate,
      v8::Local<v8::SharedArrayBuffer> shared_array_buffer) override {
    uint32_t result = 0;
    if (!v8__ValueSerializer__Delegate__GetSharedArrayBufferId(
            this, isolate, shared_array_buffer, &result)) {
      // Forward to the original method. It'll throw DataCloneError.
      return v8::ValueSerializer::Delegate::GetSharedArrayBufferId(
          isolate, shared_array_buffer);
    }
    return v8::Just(result);
  }

  v8::Maybe<uint32_t> GetWasmModuleTransferId(
      v8::Isolate* isolate, v8::Local<v8::WasmModuleObject> module) override {
    uint32_t result = 0;
    if (!v8__ValueSerializer__Delegate__GetWasmModuleTransferId(
            this, isolate, module, &result))
      return v8::Nothing<uint32_t>();
    return v8::Just(result);
  }

  void* ReallocateBufferMemory(void* old_buffer, size_t size,
                               size_t* actual_size) override {
    return v8__ValueSerializer__Delegate__ReallocateBufferMemory(
        this, old_buffer, size, actual_size);
  }

  void FreeBufferMemory(void* buffer) override {
    v8__ValueSerializer__Delegate__FreeBufferMemory(this, buffer);
  }
};

extern "C" {
void v8__ValueSerializer__Delegate__CONSTRUCT(
    uninit_t<v8__ValueSerializer__Delegate>* buf) {
  static_assert(sizeof(v8__ValueSerializer__Delegate) == sizeof(size_t),
                "v8__ValueSerializer__Delegate size mismatch");
  construct_in_place<v8__ValueSerializer__Delegate>(buf);
}
}

// v8::ValueSerializer

extern "C" {
void v8__ValueSerializer__CONSTRUCT(uninit_t<v8::ValueSerializer>* buf,
                                    v8::Isolate* isolate,
                                    v8::ValueSerializer::Delegate* delegate) {
  static_assert(sizeof(v8::ValueSerializer) == sizeof(size_t),
                "v8::ValueSerializer size mismatch");
  construct_in_place<v8::ValueSerializer>(buf, isolate, delegate);
}

void v8__ValueSerializer__DESTRUCT(v8::ValueSerializer* self) {
  self->~ValueSerializer();
}

void v8__ValueSerializer__Release(v8::ValueSerializer* self, uint8_t** ptr,
                                  size_t* size) {
  auto result = self->Release();
  *ptr = result.first;
  *size = result.second;
}

void v8__ValueSerializer__WriteHeader(v8::ValueSerializer* self) {
  self->WriteHeader();
}

MaybeBool v8__ValueSerializer__WriteValue(v8::ValueSerializer* self,
                                          v8::Local<v8::Context> context,
                                          v8::Local<v8::Value> value) {
  return maybe_to_maybe_bool(self->WriteValue(context, value));
}

void v8__ValueSerializer__TransferArrayBuffer(
    v8::ValueSerializer* self, uint32_t transfer_id,
    v8::Local<v8::ArrayBuffer> array_buffer) {
  self->TransferArrayBuffer(transfer_id, array_buffer);
}

void v8__ValueSerializer__WriteUint32(v8::ValueSerializer* self,
                                      uint32_t value) {
  self->WriteUint32(value);
}

void v8__ValueSerializer__WriteUint64(v8::ValueSerializer* self,
                                      uint64_t value) {
  self->WriteUint64(value);
}

void v8__ValueSerializer__WriteDouble(v8::ValueSerializer* self, double value) {
  self->WriteDouble(value);
}

void v8__ValueSerializer__WriteRawBytes(v8::ValueSerializer* self,
                                        const void* source, size_t length) {
  self->WriteRawBytes(source, length);
}
}

// v8::ValueDeserializer::Delegate

extern "C" {
v8::Object* v8__ValueDeserializer__Delegate__ReadHostObject(
    v8::ValueDeserializer::Delegate* self, v8::Isolate* isolate);

v8::SharedArrayBuffer*
v8__ValueDeserializer__Delegate__GetSharedArrayBufferFromId(
    v8::ValueDeserializer::Delegate* self, v8::Isolate* isolate,
    uint32_t transfer_id);

v8::WasmModuleObject* v8__ValueDeserializer__Delegate__GetWasmModuleFromId(
    v8::ValueDeserializer::Delegate* self, v8::Isolate* isolate,
    uint32_t clone_id);
}

struct v8__ValueDeserializer__Delegate
    : public v8::ValueDeserializer::Delegate {
  v8::MaybeLocal<v8::Object> ReadHostObject(v8::Isolate* isolate) override {
    return ptr_to_maybe_local(
        v8__ValueDeserializer__Delegate__ReadHostObject(this, isolate));
  }

  v8::MaybeLocal<v8::SharedArrayBuffer> GetSharedArrayBufferFromId(
      v8::Isolate* isolate, uint32_t transfer_id) override {
    return ptr_to_maybe_local(
        v8__ValueDeserializer__Delegate__GetSharedArrayBufferFromId(
            this, isolate, transfer_id));
  }

  v8::MaybeLocal<v8::WasmModuleObject> GetWasmModuleFromId(
      v8::Isolate* isolate, uint32_t clone_id) override {
    return ptr_to_maybe_local(
        v8__ValueDeserializer__Delegate__GetWasmModuleFromId(this, isolate,
                                                             clone_id));
  }
};

extern "C" {
void v8__ValueDeserializer__Delegate__CONSTRUCT(
    uninit_t<v8__ValueDeserializer__Delegate>* buf) {
  static_assert(sizeof(v8__ValueDeserializer__Delegate) == sizeof(size_t),
                "v8__ValueDeserializer__Delegate size mismatch");
  construct_in_place<v8__ValueDeserializer__Delegate>(buf);
}
}

// v8::ValueDeserializer

extern "C" {
void v8__ValueDeserializer__CONSTRUCT(
    uninit_t<v8::ValueDeserializer>* buf, v8::Isolate* isolate,
    const uint8_t* data, size_t size,
    v8::ValueDeserializer::Delegate* delegate) {
  static_assert(sizeof(v8::ValueDeserializer) == sizeof(size_t),
                "v8::ValueDeserializer size mismatch");
  construct_in_place<v8::ValueDeserializer>(buf, isolate, data, size, delegate);
}

void v8__ValueDeserializer__DESTRUCT(v8::ValueDeserializer* self) {
  self->~ValueDeserializer();
}

MaybeBool v8__ValueDeserializer__ReadHeader(v8::ValueDeserializer* self,
                                            v8::Local<v8::Context> context) {
  return maybe_to_maybe_bool(self->ReadHeader(context));
}

v8::Value* v8__ValueDeserializer__ReadValue(v8::ValueDeserializer* self,
                                            v8::Local<v8::Context> context) {
  return maybe_local_to_ptr(self->ReadValue(context));
}

void v8__ValueDeserializer__TransferArrayBuffer(
    v8::ValueDeserializer* self, uint32_t transfer_id,
    v8::Local<v8::ArrayBuffer> array_buffer) {
  self->TransferArrayBuffer(transfer_id, array_buffer);
}

void v8__ValueDeserializer__SetSupportsLegacyWireFormat(
    v8::ValueDeserializer* self, bool supports_legacy_wire_format) {
  self->SetSupportsLegacyWireFormat(supports_legacy_wire_format);
}

bool v8__ValueDeserializer__ReadUint32(v8::ValueDeserializer* self,
                                       uint32_t* value) {
  return self->ReadUint32(value);
}

bool v8__ValueDeserializer__ReadUint64(v8::ValueDeserializer* self,
                                       uint64_t* value) {
  return self->ReadUint64(value);
}

bool v8__ValueDeserializer__ReadDouble(v8::ValueDeserializer* self,
                                       double* value) {
  return self->ReadDouble(value);
}

bool v8__ValueDeserializer__ReadRawBytes(v8::ValueDeserializer* self,
                                         size_t length, const void** data) {
  return self->ReadRawBytes(length, data);
}
}  // extern "C"

// v8::CompiledWasmModule

extern "C" {
const v8::WasmModuleObject* v8__WasmModuleObject__FromCompiledModule(v8::Isolate* isolate,
                                                                     const v8::CompiledWasmModule* compiled_module) {
  return maybe_local_to_ptr(v8::WasmModuleObject::FromCompiledModule(isolate, *compiled_module));
}

v8::CompiledWasmModule* v8__WasmModuleObject__GetCompiledModule(const v8::WasmModuleObject* self) {
  v8::CompiledWasmModule cwm = ptr_to_local(self)->GetCompiledModule();
  return new v8::CompiledWasmModule(std::move(cwm));
}

const uint8_t* v8__CompiledWasmModule__GetWireBytesRef(v8::CompiledWasmModule* self,
                                                       size_t* length) {
  v8::MemorySpan<const uint8_t> span = self->GetWireBytesRef();
  *length = span.size();
  return span.data();
}

const char* v8__CompiledWasmModule__SourceUrl(v8::CompiledWasmModule* self,
                                              size_t* length) {
  const std::string& source_url = self->source_url();
  *length = source_url.size();
  return source_url.data();
}

void v8__CompiledWasmModule__DELETE(v8::CompiledWasmModule* self) {
  delete self;
}
}  // extern "C"