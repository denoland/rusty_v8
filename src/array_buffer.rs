use crate::support::Delete;
use crate::support::Opaque;
use crate::support::UniqueRef;

extern "C" {
  fn v8__ArrayBuffer__Allocator__NewDefaultAllocator() -> *mut Allocator;
  fn v8__ArrayBuffer__Allocator__DELETE(this: &'static mut Allocator);
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

impl Delete for Allocator {
  fn delete(&'static mut self) {
    unsafe { v8__ArrayBuffer__Allocator__DELETE(self) };
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_default_allocator() {
    Allocator::new_default_allocator();
  }
}
