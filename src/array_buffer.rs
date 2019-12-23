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

  fn v8__ArrayBuffer__NewBackingStore(
    isolate: *mut Isolate,
    byte_length: usize,
  ) -> *mut BackingStore;
  fn v8__BackingStore__ByteLength(self_: &BackingStore) -> usize;
  fn v8__BackingStore__IsShared(self_: &BackingStore) -> bool;
  fn v8__BackingStore__DELETE(self_: &mut BackingStore);
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

/// A wrapper around the backing store (i.e. the raw memory) of an array buffer.
/// See a document linked in http://crbug.com/v8/9908 for more information.
///
/// The allocation and destruction of backing stores is generally managed by
/// V8. Clients should always use standard C++ memory ownership types (i.e.
/// std::unique_ptr and std::shared_ptr) to manage lifetimes of backing stores
/// properly, since V8 internal objects may alias backing stores.
///
/// This object does not keep the underlying |ArrayBuffer::Allocator| alive by
/// default. Use Isolate::CreateParams::array_buffer_allocator_shared when
/// creating the Isolate to make it hold a reference to the allocator itself.
#[repr(C)]
pub struct BackingStore([usize; 6]);

impl BackingStore {
  /// Return a pointer to the beginning of the memory block for this backing
  /// store. The pointer is only valid as long as this backing store object
  /// lives.
  pub fn data(&self) -> std::ffi::c_void {
    unimplemented!()
  }

  /// The length (in bytes) of this backing store.
  pub fn byte_length(&self) -> usize {
    unsafe { v8__BackingStore__ByteLength(self) }
  }

  /// Indicates whether the backing store was created for an ArrayBuffer or
  /// a SharedArrayBuffer.
  pub fn is_shared(&self) -> bool {
    unsafe { v8__BackingStore__IsShared(self) }
  }
}

impl Delete for BackingStore {
  fn delete(&mut self) {
    unsafe { v8__BackingStore__DELETE(self) };
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

  /// Returns a new standalone BackingStore that is allocated using the array
  /// buffer allocator of the isolate. The result can be later passed to
  /// ArrayBuffer::New.
  ///
  /// If the allocator returns nullptr, then the function may cause GCs in the
  /// given isolate and re-try the allocation. If GCs do not help, then the
  /// function will crash with an out-of-memory error.
  pub fn new_backing_store<'sc>(
    scope: &mut HandleScope<'sc>,
    byte_length: usize,
  ) -> UniqueRef<BackingStore> {
    unsafe {
      UniqueRef::from_raw(v8__ArrayBuffer__NewBackingStore(
        scope.as_mut(),
        byte_length,
      ))
    }
  }
}
