// Copyright 2026 the Deno authors. All rights reserved. MIT license.

#include "support.h"
#include "v8/third_party/inspector_protocol/crdtp/dispatch.h"
#include "v8/third_party/inspector_protocol/crdtp/frontend_channel.h"
#include "v8/third_party/inspector_protocol/crdtp/json.h"

using namespace support;
using namespace v8_crdtp;

extern "C" {
void crdtp__FrontendChannel__BASE__sendProtocolResponse(FrontendChannel* self,
                                                        int call_id,
                                                        Serializable* message);
void crdtp__FrontendChannel__BASE__sendProtocolNotification(
    FrontendChannel* self, Serializable* message);
void crdtp__FrontendChannel__BASE__fallThrough(
    FrontendChannel* self, int call_id, const uint8_t* method_data,
    size_t method_len, const uint8_t* message_data, size_t message_len);
void crdtp__FrontendChannel__BASE__flushProtocolNotifications(
    FrontendChannel* self);
}  // extern "C"

struct crdtp__FrontendChannel__BASE : public FrontendChannel {
  void SendProtocolResponse(int call_id,
                            std::unique_ptr<Serializable> message) override {
    crdtp__FrontendChannel__BASE__sendProtocolResponse(this, call_id,
                                                       message.release());
  }
  void SendProtocolNotification(
      std::unique_ptr<Serializable> message) override {
    crdtp__FrontendChannel__BASE__sendProtocolNotification(this,
                                                           message.release());
  }
  void FallThrough(int call_id, span<uint8_t> method,
                   span<uint8_t> message) override {
    crdtp__FrontendChannel__BASE__fallThrough(this, call_id, method.data(),
                                              method.size(), message.data(),
                                              message.size());
  }
  void FlushProtocolNotifications() override {
    crdtp__FrontendChannel__BASE__flushProtocolNotifications(this);
  }
};

extern "C" {

void crdtp__FrontendChannel__BASE__CONSTRUCT(
    uninit_t<crdtp__FrontendChannel__BASE>* buf) {
  construct_in_place<crdtp__FrontendChannel__BASE>(buf);
}

size_t crdtp__FrontendChannel__BASE__SIZE() {
  return sizeof(crdtp__FrontendChannel__BASE);
}

void crdtp__Serializable__DELETE(Serializable* self) { delete self; }

// Serialize to CBOR bytes
void crdtp__Serializable__serializeToCBOR(const Serializable* self,
                                          std::vector<uint8_t>* out) {
  self->AppendSerialized(out);
}

// Helper to get serialized bytes
size_t crdtp__Serializable__getSerializedSize(const Serializable* self) {
  std::vector<uint8_t> bytes;
  self->AppendSerialized(&bytes);
  return bytes.size();
}

void crdtp__Serializable__getSerializedBytes(const Serializable* self,
                                             uint8_t* out, size_t len) {
  std::vector<uint8_t> bytes;
  self->AppendSerialized(&bytes);
  size_t copy_len = std::min(len, bytes.size());
  memcpy(out, bytes.data(), copy_len);
}

Dispatchable* crdtp__Dispatchable__new(const uint8_t* data, size_t len) {
  return new Dispatchable(span<uint8_t>(data, len));
}

void crdtp__Dispatchable__DELETE(Dispatchable* self) { delete self; }

bool crdtp__Dispatchable__ok(const Dispatchable* self) { return self->ok(); }

int32_t crdtp__Dispatchable__callId(const Dispatchable* self) {
  return self->CallId();
}

bool crdtp__Dispatchable__hasCallId(const Dispatchable* self) {
  return self->HasCallId();
}

size_t crdtp__Dispatchable__methodLen(const Dispatchable* self) {
  return self->Method().size();
}

void crdtp__Dispatchable__methodCopy(const Dispatchable* self, uint8_t* out) {
  span<uint8_t> method = self->Method();
  memcpy(out, method.data(), method.size());
}

size_t crdtp__Dispatchable__sessionIdLen(const Dispatchable* self) {
  return self->SessionId().size();
}

void crdtp__Dispatchable__sessionIdCopy(const Dispatchable* self,
                                        uint8_t* out) {
  span<uint8_t> session_id = self->SessionId();
  memcpy(out, session_id.data(), session_id.size());
}

size_t crdtp__Dispatchable__paramsLen(const Dispatchable* self) {
  return self->Params().size();
}

void crdtp__Dispatchable__paramsCopy(const Dispatchable* self, uint8_t* out) {
  span<uint8_t> params = self->Params();
  memcpy(out, params.data(), params.size());
}

struct DispatchResponseWrapper {
  DispatchResponse inner;

  explicit DispatchResponseWrapper(DispatchResponse&& r)
      : inner(std::move(r)) {}
};

DispatchResponseWrapper* crdtp__DispatchResponse__Success() {
  return new DispatchResponseWrapper(DispatchResponse::Success());
}

DispatchResponseWrapper* crdtp__DispatchResponse__FallThrough() {
  return new DispatchResponseWrapper(DispatchResponse::FallThrough());
}

DispatchResponseWrapper* crdtp__DispatchResponse__ParseError(const char* msg,
                                                             size_t len) {
  return new DispatchResponseWrapper(
      DispatchResponse::ParseError(std::string(msg, len)));
}

DispatchResponseWrapper* crdtp__DispatchResponse__InvalidRequest(
    const char* msg, size_t len) {
  return new DispatchResponseWrapper(
      DispatchResponse::InvalidRequest(std::string(msg, len)));
}

DispatchResponseWrapper* crdtp__DispatchResponse__MethodNotFound(
    const char* msg, size_t len) {
  return new DispatchResponseWrapper(
      DispatchResponse::MethodNotFound(std::string(msg, len)));
}

DispatchResponseWrapper* crdtp__DispatchResponse__InvalidParams(const char* msg,
                                                                size_t len) {
  return new DispatchResponseWrapper(
      DispatchResponse::InvalidParams(std::string(msg, len)));
}

DispatchResponseWrapper* crdtp__DispatchResponse__ServerError(const char* msg,
                                                              size_t len) {
  return new DispatchResponseWrapper(
      DispatchResponse::ServerError(std::string(msg, len)));
}

void crdtp__DispatchResponse__DELETE(DispatchResponseWrapper* self) {
  delete self;
}

bool crdtp__DispatchResponse__isSuccess(const DispatchResponseWrapper* self) {
  return self->inner.IsSuccess();
}

bool crdtp__DispatchResponse__isError(const DispatchResponseWrapper* self) {
  return self->inner.IsError();
}

bool crdtp__DispatchResponse__isFallThrough(
    const DispatchResponseWrapper* self) {
  return self->inner.IsFallThrough();
}

int crdtp__DispatchResponse__code(const DispatchResponseWrapper* self) {
  return static_cast<int>(self->inner.Code());
}

size_t crdtp__DispatchResponse__messageLen(
    const DispatchResponseWrapper* self) {
  return self->inner.Message().size();
}

void crdtp__DispatchResponse__messageCopy(const DispatchResponseWrapper* self,
                                          char* out) {
  const std::string& msg = self->inner.Message();
  memcpy(out, msg.data(), msg.size());
}

UberDispatcher* crdtp__UberDispatcher__new(FrontendChannel* channel) {
  return new UberDispatcher(channel);
}

void crdtp__UberDispatcher__DELETE(UberDispatcher* self) { delete self; }

FrontendChannel* crdtp__UberDispatcher__channel(UberDispatcher* self) {
  return self->channel();
}

// Dispatch result wrapper
struct DispatchResultWrapper {
  UberDispatcher::DispatchResult inner;

  DispatchResultWrapper(UberDispatcher::DispatchResult&& r)
      : inner(std::move(r)) {}
};

DispatchResultWrapper* crdtp__UberDispatcher__Dispatch(
    UberDispatcher* self, const Dispatchable* dispatchable) {
  return new DispatchResultWrapper(self->Dispatch(*dispatchable));
}

void crdtp__DispatchResult__DELETE(DispatchResultWrapper* self) { delete self; }

bool crdtp__DispatchResult__MethodFound(const DispatchResultWrapper* self) {
  return self->inner.MethodFound();
}

void crdtp__DispatchResult__Run(DispatchResultWrapper* self) {
  self->inner.Run();
}

// Convert JSON to CBOR
bool crdtp__json__ConvertJSONToCBOR(const uint8_t* json_data, size_t json_len,
                                    std::vector<uint8_t>* cbor_out) {
  json::ConvertJSONToCBOR(span<uint8_t>(json_data, json_len), cbor_out);
  return !cbor_out->empty();
}

// Convert CBOR to JSON
bool crdtp__json__ConvertCBORToJSON(const uint8_t* cbor_data, size_t cbor_len,
                                    std::vector<uint8_t>* json_out) {
  std::string json_str;
  Status status =
      json::ConvertCBORToJSON(span<uint8_t>(cbor_data, cbor_len), &json_str);
  if (!status.ok()) {
    return false;
  }
  json_out->assign(json_str.begin(), json_str.end());
  return true;
}

std::vector<uint8_t>* crdtp__vec_u8__new() {
  return new std::vector<uint8_t>();
}

void crdtp__vec_u8__DELETE(std::vector<uint8_t>* self) { delete self; }

size_t crdtp__vec_u8__size(const std::vector<uint8_t>* self) {
  return self->size();
}

const uint8_t* crdtp__vec_u8__data(const std::vector<uint8_t>* self) {
  return self->data();
}

void crdtp__vec_u8__copy(const std::vector<uint8_t>* self, uint8_t* out) {
  memcpy(out, self->data(), self->size());
}

Serializable* crdtp__CreateErrorResponse(int call_id,
                                         DispatchResponseWrapper* response) {
  return CreateErrorResponse(call_id, std::move(response->inner)).release();
}

Serializable* crdtp__CreateResponse(int call_id, Serializable* params) {
  std::unique_ptr<Serializable> params_ptr(params);
  return CreateResponse(call_id, std::move(params_ptr)).release();
}

Serializable* crdtp__CreateNotification(const char* method,
                                        Serializable* params) {
  std::unique_ptr<Serializable> params_ptr(params);
  return CreateNotification(method, std::move(params_ptr)).release();
}

Serializable* crdtp__CreateErrorNotification(
    DispatchResponseWrapper* response) {
  return CreateErrorNotification(std::move(response->inner)).release();
}

}  // extern "C"
