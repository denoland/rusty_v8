use crate::support::Delete;
use crate::support::Opaque;
use crate::support::UniqueRef;
use crate::HandleScope;
use crate::Isolate;
use crate::Local;

extern "C" {
  fn v8__ArrayBuffer__Allocator__NewDefaultAllocator() -> *mut Allocator;
  fn v8__ArrayBuffer__Allocator__DELETE(this: &'static mut Allocator);

  fn v8__ArrayBuffer__New(
    isolate: *mut Isolate,
    byte_length: usize,
  ) -> *mut ArrayBuffer;
  fn v8__ArrayBuffer__ByteLength(self_: *const ArrayBuffer) -> usize;
}

/// A thread-safe allocator that V8 uses to allocate |ArrayBuffer|'s memory.
/// The allocator is a global V8 setting. It has to be set via
/// Isolate::CreateParams.
///
/// Memory allocated through this allocator by V8 is accounted for as external
/// memory by V8. Note that V8 keeps track of the memory for all internalized
/// |ArrayBuffer|s. Responsibility for tracking external memory (using
/// Isolate::AdjustAmountOfExternalAllocatedMemory) is handed over to the
/// embedder upon externalization and taken over upon internalization (creating
/// an internalized buffer from an existing buffer).
///
/// Note that it is unsafe to call back into V8 from any of the allocator
/// functions.
///
/// This is called v8::ArrayBuffer::Allocator in C++. Rather than use the
/// namespace array_buffer, which will contain only the Allocator we opt in Rust
/// to allow it to live in the top level: v8::Allocator
#[repr(C)]
pub struct Allocator(Opaque);

impl Allocator {
  /// malloc/free based convenience allocator.
  ///
  /// Caller takes ownership, i.e. the returned object needs to be freed using
  /// |delete allocator| once it is no longer in use.
  pub fn new_default_allocator() -> UniqueRef<Allocator> {
    unsafe {
      UniqueRef::from_raw(v8__ArrayBuffer__Allocator__NewDefaultAllocator())
    }
  }
}

#[test]
fn test_default_allocator() {
  Allocator::new_default_allocator();
}

impl Delete for Allocator {
  fn delete(&'static mut self) {
    unsafe { v8__ArrayBuffer__Allocator__DELETE(self) };
  }
}

/// An instance of the built-in ArrayBuffer constructor (ES6 draft 15.13.5).
#[repr(C)]
pub struct ArrayBuffer(Opaque);

impl ArrayBuffer {
  /// Create a new ArrayBuffer. Allocate |byte_length| bytes.
  /// Allocated memory will be owned by a created ArrayBuffer and
  /// will be deallocated when it is garbage-collected,
  /// unless the object is externalized.
  pub fn new<'sc>(
    scope: &mut HandleScope<'sc>,
    byte_length: usize,
  ) -> Local<'sc, ArrayBuffer> {
    unsafe {
      let ptr = v8__ArrayBuffer__New(scope.as_mut(), byte_length);
      Local::from_raw(ptr).unwrap()
    }
  }

  /// Data length in bytes.
  pub fn byte_length(&self) -> usize {
    unsafe { v8__ArrayBuffer__ByteLength(self) }
  }
}
