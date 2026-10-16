// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.

// Kept out of binding.cc because it needs V8's heap internals.

#include "v8-internal.h"
#include "v8.h"
#include "v8/src/execution/isolate.h"
#include "v8/src/heap/base-page.h"
#include "v8/src/heap/memory-chunk-inl.h"

namespace i = v8::internal;

extern "C" {

// Returns the isolate whose heap holds the value of `self`, or nullptr if the
// value is a Smi or lives in a space that isolates share (read-only space,
// writable shared space). Used by debug builds to check that handles are not
// used across isolates.
const v8::Isolate* v8__Data__GetOwnerIsolate(const v8::Data& self) {
  i::Address value = i::ValueHelper::ValueAsAddress(&self);
  if (!i::Internals::HasHeapObjectTag(value)) return nullptr;
  const i::MemoryChunk* chunk = i::MemoryChunk::FromAddress(value);
  if (chunk->InReadOnlySpace() || chunk->InWritableSharedSpace()) {
    return nullptr;
  }
  const i::Heap* heap = chunk->MetadataNoIsolateCheck()->heap();
  return reinterpret_cast<const v8::Isolate*>(i::Isolate::FromHeap(heap));
}

}  // extern "C"
