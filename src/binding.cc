#include <cassert>
#include <cstdint>
#include <iostream>

#include "support.h"
#include "v8/include/libplatform/libplatform.h"
#include "v8/include/v8-inspector.h"
#include "v8/include/v8-platform.h"
#include "v8/include/v8.h"

// TODO(ry) do not use "using namespace" so the binding code is more explicit.
using namespace v8;
using namespace support;

static_assert(sizeof(ScriptOrigin) == sizeof(size_t) * 7,
              "ScriptOrigin size mismatch");

static_assert(sizeof(HandleScope) == sizeof(size_t) * 3,
              "HandleScope size mismatch");

static_assert(sizeof(v8::PromiseRejectMessage) == sizeof(size_t) * 3,
              "PromiseRejectMessage size mismatch");

static_assert(sizeof(v8::Locker) == sizeof(size_t) * 2,
              "Locker size mismatch");

extern "C" {

void v8__V8__SetFlagsFromCommandLine(int* argc, char** argv) {
  V8::SetFlagsFromCommandLine(argc, argv, true);
}

const char* v8__V8__GetVersion() { return V8::GetVersion(); }

void v8__V8__InitializePlatform(Platform& platform) {
  V8::InitializePlatform(&platform);
}

void v8__V8__Initialize() { V8::Initialize(); }

bool v8__V8__Dispose() { return V8::Dispose(); }

void v8__V8__ShutdownPlatform() { V8::ShutdownPlatform(); }

// This function consumes the Isolate::CreateParams object. The Isolate takes
// ownership of the ArrayBuffer::Allocator referenced by the params object.
Isolate* v8__Isolate__New(Isolate::CreateParams& params) {
  auto isolate = Isolate::New(params);
  delete &params;
  return isolate;
}

void v8__Isolate__Dispose(Isolate* isolate) {
  auto allocator = isolate->GetArrayBufferAllocator();
  isolate->Dispose();
  delete allocator;
}

void v8__Isolate__Enter(Isolate* isolate) { isolate->Enter(); }

void v8__Isolate__Exit(Isolate* isolate) { isolate->Exit(); }

void v8__Isolate__SetPromiseRejectCallback(Isolate* isolate,
                                           v8::PromiseRejectCallback callback) {
  isolate->SetPromiseRejectCallback(callback);
}

void v8__Isolate__SetCaptureStackTraceForUncaughtExceptions(Isolate* isolate,
                                                            bool capture,
                                                            int frame_limit) {
  // Note: StackTraceOptions are deprecated so we don't bother to bind to it.
  isolate->SetCaptureStackTraceForUncaughtExceptions(capture, frame_limit);
}

Isolate::CreateParams* v8__Isolate__CreateParams__NEW() {
  return new Isolate::CreateParams();
}

// This function is only called if the Isolate::CreateParams object is *not*
// consumed by Isolate::New().
void v8__Isolate__CreateParams__DELETE(Isolate::CreateParams& self) {
  delete self.array_buffer_allocator;
  delete &self;
}

// This function takes ownership of the ArrayBuffer::Allocator.
void v8__Isolate__CreateParams__SET__array_buffer_allocator(
    Isolate::CreateParams& self, ArrayBuffer::Allocator* value) {
  delete self.array_buffer_allocator;
  self.array_buffer_allocator = value;
}

void v8__HandleScope__CONSTRUCT(uninit_t<HandleScope>& buf, Isolate* isolate) {
  construct_in_place<HandleScope>(buf, isolate);
}

void v8__HandleScope__DESTRUCT(HandleScope& self) { self.~HandleScope(); }

Isolate* v8__HandleScope__GetIsolate(const HandleScope& self) {
  return self.GetIsolate();
}

void v8__Locker__CONSTRUCT(uninit_t<Locker>& buf, Isolate* isolate) {
  construct_in_place<Locker>(buf, isolate);
}

void v8__Locker__DESTRUCT(Locker& self) { self.~Locker(); }

bool v8__Value__IsUndefined(const Value& self) { return self.IsUndefined(); }

bool v8__Value__IsNull(const Value& self) { return self.IsNull(); }

bool v8__Value__IsNullOrUndefined(const Value& self) {
  return self.IsNullOrUndefined();
}

v8::Primitive* v8__Null(v8::Isolate* isolate) {
  return local_to_ptr(v8::Null(isolate));
}

v8::Primitive* v8__Undefined(v8::Isolate* isolate) {
  return local_to_ptr(v8::Undefined(isolate));
}

v8::Boolean* v8__True(v8::Isolate* isolate) {
  return local_to_ptr(v8::True(isolate));
}

v8::Boolean* v8__False(v8::Isolate* isolate) {
  return local_to_ptr(v8::False(isolate));
}

String* v8__String__NewFromUtf8(Isolate* isolate, const char* data,
                                NewStringType type, int length) {
  return maybe_local_to_ptr(String::NewFromUtf8(isolate, data, type, length));
}

int v8__String__Length(const String& self) { return self.Length(); }

int v8__String__Utf8Length(const String& self, Isolate* isolate) {
  return self.Utf8Length(isolate);
}

int v8__String__WriteUtf8(const String& self, Isolate* isolate, char* buffer,
                          int length, int* nchars_ref, int options) {
  return self.WriteUtf8(isolate, buffer, length, nchars_ref, options);
}

v8::Object* v8__Object__New(v8::Isolate* isolate,
                            v8::Local<v8::Value> prototype_or_null,
                            v8::Local<v8::Name>* names,
                            v8::Local<v8::Value>* values, size_t length) {
  return local_to_ptr(
      v8::Object::New(isolate, prototype_or_null, names, values, length));
}

v8::Isolate* v8__Object__GetIsolate(v8::Object& self) {
  return self.GetIsolate();
}

Number* v8__Number__New(Isolate* isolate, double value) {
  return *Number::New(isolate, value);
}

double v8__Number__Value(const Number& self) { return self.Value(); }

Integer* v8__Integer__New(Isolate* isolate, int32_t value) {
  return *Integer::New(isolate, value);
}

Integer* v8__Integer__NewFromUnsigned(Isolate* isolate, uint32_t value) {
  return *Integer::NewFromUnsigned(isolate, value);
}

int64_t v8__Integer__Value(const Integer& self) { return self.Value(); }

ArrayBuffer::Allocator* v8__ArrayBuffer__Allocator__NewDefaultAllocator() {
  return ArrayBuffer::Allocator::NewDefaultAllocator();
}

void v8__ArrayBuffer__Allocator__DELETE(ArrayBuffer::Allocator& self) {
  delete &self;
}

Context* v8__Context__New(Isolate* isolate) {
  // TODO: optional arguments.
  return *Context::New(isolate);
}

void v8__Context__Enter(Context& self) { self.Enter(); }

void v8__Context__Exit(Context& self) { self.Exit(); }

Isolate* v8__Context__GetIsolate(Context& self) { return self.GetIsolate(); }

Object* v8__Context__Global(Context& self) { return *self.Global(); }

v8::String* v8__Message__Get(v8::Message* self) {
  return local_to_ptr(self->Get());
}

v8::Value* v8__Exception__RangeError(v8::Local<v8::String> message) {
  return local_to_ptr(v8::Exception::RangeError(message));
}

v8::Value* v8__Exception__ReferenceError(v8::Local<v8::String> message) {
  return local_to_ptr(v8::Exception::ReferenceError(message));
}

v8::Value* v8__Exception__SyntaxError(v8::Local<v8::String> message) {
  return local_to_ptr(v8::Exception::SyntaxError(message));
}

v8::Value* v8__Exception__TypeError(v8::Local<v8::String> message) {
  return local_to_ptr(v8::Exception::TypeError(message));
}

v8::Value* v8__Exception__Error(v8::Local<v8::String> message) {
  return local_to_ptr(v8::Exception::Error(message));
}

v8::Message* v8__Exception__CreateMessage(v8::Isolate* isolate,
                                          v8::Local<v8::Value> exception) {
  return local_to_ptr(v8::Exception::CreateMessage(isolate, exception));
}

v8::StackTrace* v8__Exception__GetStackTrace(v8::Local<v8::Value> exception) {
  return local_to_ptr(v8::Exception::GetStackTrace(exception));
}

v8::Function* v8__Function__New(v8::Local<v8::Context> context,
                                v8::FunctionCallback callback) {
  return maybe_local_to_ptr(v8::Function::New(context, callback));
}

v8::Value* v8__Function__Call(v8::Function* self,
                              v8::Local<v8::Context> context,
                              v8::Local<v8::Value> recv, int argc,
                              v8::Local<v8::Value> argv[]) {
  return maybe_local_to_ptr(self->Call(context, recv, argc, argv));
}

v8::FunctionTemplate* v8__FunctionTemplate__New(
    v8::Isolate* isolate, v8::FunctionCallback callback = nullptr) {
  return local_to_ptr(v8::FunctionTemplate::New(isolate, callback));
}

v8::Function* v8__FunctionTemplate__GetFunction(
    v8::Local<v8::FunctionTemplate> self, v8::Local<v8::Context> context) {
  return maybe_local_to_ptr(self->GetFunction(context));
}
int v8__FunctionCallbackInfo__Length(
    v8::FunctionCallbackInfo<v8::Value>* self) {
  return self->Length();
}

v8::Isolate* v8__FunctionCallbackInfo__GetIsolate(
    v8::FunctionCallbackInfo<v8::Value>* self) {
  return self->GetIsolate();
}

v8::ReturnValue<v8::Value>* v8__FunctionCallbackInfo__GetReturnValue(
    v8::FunctionCallbackInfo<v8::Value>* self) {
  v8::ReturnValue<v8::Value>* rv =
      new v8::ReturnValue<v8::Value>(self->GetReturnValue());
  return rv;
}

void v8__ReturnValue__Set(v8::ReturnValue<v8::Value>* self,
                          v8::Local<v8::Value> value) {
  self->Set(value);
}

v8::Value* v8__ReturnValue__Get(v8::ReturnValue<v8::Value>* self) {
  return local_to_ptr(self->Get());
}

v8::Isolate* v8__ReturnValue__GetIsolate(v8::ReturnValue<v8::Value>* self) {
  return self->GetIsolate();
}

int v8__StackTrace__GetFrameCount(v8::StackTrace* self) {
  return self->GetFrameCount();
}

Script* v8__Script__Compile(Context* context, String* source,
                            ScriptOrigin* origin) {
  return maybe_local_to_ptr(
      Script::Compile(ptr_to_local(context), ptr_to_local(source), origin));
}

Value* v8__Script__Run(Script& script, Context* context) {
  return maybe_local_to_ptr(script.Run(ptr_to_local(context)));
}

void v8__ScriptOrigin__CONSTRUCT(uninit_t<ScriptOrigin>& buf,
                                 Value* resource_name,
                                 Integer* resource_line_offset,
                                 Integer* resource_column_offset,
                                 Boolean* resource_is_shared_cross_origin,
                                 Integer* script_id, Value* source_map_url,
                                 Boolean* resource_is_opaque, Boolean* is_wasm,
                                 Boolean* is_module) {
  construct_in_place<ScriptOrigin>(
      buf, ptr_to_local(resource_name), ptr_to_local(resource_line_offset),
      ptr_to_local(resource_column_offset),
      ptr_to_local(resource_is_shared_cross_origin), ptr_to_local(script_id),
      ptr_to_local(source_map_url), ptr_to_local(resource_is_opaque),
      ptr_to_local(is_wasm), ptr_to_local(is_module));
}

v8::Value* v8__JSON__Parse(v8::Local<v8::Context> context,
                           v8::Local<v8::String> json_string) {
  return maybe_local_to_ptr(v8::JSON::Parse(context, json_string));
}

v8::String* v8__JSON__Stringify(v8::Local<v8::Context> context,
                                v8::Local<v8::Value> json_object) {
  return maybe_local_to_ptr(v8::JSON::Stringify(context, json_object));
}

v8::Promise::Resolver* v8__Promise__Resolver__New(
    v8::Local<v8::Context> context) {
  return maybe_local_to_ptr(v8::Promise::Resolver::New(context));
}

v8::Promise* v8__Promise__Resolver__GetPromise(v8::Promise::Resolver* self) {
  return local_to_ptr(self->GetPromise());
}

MaybeBool v8__Promise__Resolver__Resolve(v8::Promise::Resolver* self,
                                         v8::Local<v8::Context> context,
                                         v8::Local<v8::Value> value) {
  return maybe_to_maybe_bool(self->Resolve(context, value));
}

MaybeBool v8__Promise__Resolver__Reject(v8::Promise::Resolver* self,
                                        v8::Local<v8::Context> context,
                                        v8::Local<v8::Value> value) {
  return maybe_to_maybe_bool(self->Reject(context, value));
}

v8::Promise::PromiseState v8__Promise__State(v8::Promise* self) {
  return self->State();
}

bool v8__Promise__HasHandler(v8::Promise* self) { return self->HasHandler(); }

v8::Value* v8__Promise__Result(v8::Promise* self) {
  return local_to_ptr(self->Result());
}

v8::Promise* v8__Promise__Catch(v8::Promise* self,
                                v8::Local<v8::Context> context,
                                v8::Local<v8::Function> handler) {
  return maybe_local_to_ptr(self->Catch(context, handler));
}

v8::Promise* v8__Promise__Then(v8::Promise* self,
                               v8::Local<v8::Context> context,
                               v8::Local<v8::Function> handler) {
  return maybe_local_to_ptr(self->Then(context, handler));
}

v8::Promise* v8__Promise__Then2(v8::Promise* self,
                                v8::Local<v8::Context> context,
                                v8::Local<v8::Function> on_fulfilled,
                                v8::Local<v8::Function> on_rejected) {
  return maybe_local_to_ptr(self->Then(context, on_fulfilled, on_rejected));
}

v8::PromiseRejectEvent v8__PromiseRejectMessage__GetEvent(
    const v8::PromiseRejectMessage& self) {
  return self.GetEvent();
}

v8::Promise* v8__PromiseRejectMessage__GetPromise(
    const v8::PromiseRejectMessage& self) {
  return local_to_ptr(self.GetPromise());
}

v8::Value* v8__PromiseRejectMessage__GetValue(
    const v8::PromiseRejectMessage& self) {
  return local_to_ptr(self.GetValue());
}

v8::Platform* v8__platform__NewDefaultPlatform() {
  // TODO: support optional arguments.
  return v8::platform::NewDefaultPlatform().release();
}

void v8__Platform__DELETE(v8::Platform& self) { delete &self; }
void v8__Task__BASE__DELETE(Task& self);
void v8__Task__BASE__Run(Task& self);

struct v8__Task__BASE : public Task {
  using Task::Task;
  void operator delete(void* ptr) noexcept {
    v8__Task__BASE__DELETE(*reinterpret_cast<Task*>(ptr));
  }
  void Run() override { v8__Task__BASE__Run(*this); }
};

void v8__Task__BASE__CONSTRUCT(uninit_t<v8__Task__BASE>& buf) {
  construct_in_place<v8__Task__BASE>(buf);
}
void v8__Task__DELETE(Task& self) { delete &self; }
void v8__Task__Run(Task& self) { self.Run(); }

void v8_inspector__V8Inspector__Channel__BASE__sendResponse(
    v8_inspector::V8Inspector::Channel& self, int callId,
    v8_inspector::StringBuffer* message);
void v8_inspector__V8Inspector__Channel__BASE__sendNotification(
    v8_inspector::V8Inspector::Channel& self,
    v8_inspector::StringBuffer* message);
void v8_inspector__V8Inspector__Channel__BASE__flushProtocolNotifications(
    v8_inspector::V8Inspector::Channel& self);
}  // extern "C"

struct v8_inspector__V8Inspector__Channel__BASE
    : public v8_inspector::V8Inspector::Channel {
  using v8_inspector::V8Inspector::Channel::Channel;

  void sendResponse(
      int callId,
      std::unique_ptr<v8_inspector::StringBuffer> message) override {
    v8_inspector__V8Inspector__Channel__BASE__sendResponse(*this, callId,
                                                           message.release());
  }
  void sendNotification(
      std::unique_ptr<v8_inspector::StringBuffer> message) override {
    v8_inspector__V8Inspector__Channel__BASE__sendNotification(
        *this, message.release());
  }
  void flushProtocolNotifications() override {
    v8_inspector__V8Inspector__Channel__BASE__flushProtocolNotifications(*this);
  }
};

extern "C" {
void v8_inspector__V8Inspector__Channel__BASE__CONSTRUCT(
    uninit_t<v8_inspector__V8Inspector__Channel__BASE>& buf) {
  construct_in_place<v8_inspector__V8Inspector__Channel__BASE>(buf);
}

void v8_inspector__V8Inspector__Channel__sendResponse(
    v8_inspector::V8Inspector::Channel& self, int callId,
    v8_inspector::StringBuffer* message) {
  self.sendResponse(
      callId,
      static_cast<std::unique_ptr<v8_inspector::StringBuffer>>(message));
}
void v8_inspector__V8Inspector__Channel__sendNotification(
    v8_inspector::V8Inspector::Channel& self,
    v8_inspector::StringBuffer* message) {
  self.sendNotification(
      static_cast<std::unique_ptr<v8_inspector::StringBuffer>>(message));
}
void v8_inspector__V8Inspector__Channel__flushProtocolNotifications(
    v8_inspector::V8Inspector::Channel& self) {
  self.flushProtocolNotifications();
}

void v8_inspector__V8InspectorClient__BASE__runMessageLoopOnPause(
    v8_inspector::V8InspectorClient& self, int contextGroupId);
void v8_inspector__V8InspectorClient__BASE__quitMessageLoopOnPause(
    v8_inspector::V8InspectorClient& self);
void v8_inspector__V8InspectorClient__BASE__runIfWaitingForDebugger(
    v8_inspector::V8InspectorClient& self, int contextGroupId);
}  // extern "C"

struct v8_inspector__V8InspectorClient__BASE
    : public v8_inspector::V8InspectorClient {
  using v8_inspector::V8InspectorClient::V8InspectorClient;

  void runMessageLoopOnPause(int contextGroupId) override {
    v8_inspector__V8InspectorClient__BASE__runMessageLoopOnPause(
        *this, contextGroupId);
  }
  void quitMessageLoopOnPause() override {
    v8_inspector__V8InspectorClient__BASE__quitMessageLoopOnPause(*this);
  }
  void runIfWaitingForDebugger(int contextGroupId) override {
    v8_inspector__V8InspectorClient__BASE__runIfWaitingForDebugger(
        *this, contextGroupId);
  }
};

extern "C" {
void v8_inspector__V8InspectorClient__BASE__CONSTRUCT(
    uninit_t<v8_inspector__V8InspectorClient__BASE>& buf) {
  construct_in_place<v8_inspector__V8InspectorClient__BASE>(buf);
}

void v8_inspector__V8InspectorClient__runMessageLoopOnPause(
    v8_inspector::V8InspectorClient& self, int contextGroupId) {
  self.runMessageLoopOnPause(contextGroupId);
}
void v8_inspector__V8InspectorClient__quitMessageLoopOnPause(
    v8_inspector::V8InspectorClient& self) {
  self.quitMessageLoopOnPause();
}
void v8_inspector__V8InspectorClient__runIfWaitingForDebugger(
    v8_inspector::V8InspectorClient& self, int contextGroupId) {
  self.runIfWaitingForDebugger(contextGroupId);
}

void v8_inspector__StringBuffer__DELETE(v8_inspector::StringBuffer& self) {
  delete &self;
}

const v8_inspector::StringView* v8_inspector__StringBuffer__string(
    v8_inspector::StringBuffer& self) {
  return &self.string();
}

v8_inspector::StringBuffer* v8_inspector__StringBuffer__create(
    const v8_inspector::StringView& source) {
  return v8_inspector::StringBuffer::create(source).release();
}
}  // extern "C"
