use crate::support::Delete;
use crate::support::Opaque;
use crate::support::UniquePtr;

extern "C" {
  fn v8__ArrayBuffer__Allocator__NewDefaultAllocator() -> *mut Allocator;
  fn v8__ArrayBuffer__Allocator__DELETE(this: &'static mut Allocator);
}

// TODO: allow the user to implement their own Allocator.
#[repr(C)]
pub struct Allocator(Opaque);

impl Allocator {
  pub fn new_default_allocator() -> UniquePtr<Allocator> {
    unsafe {
      UniquePtr::from_raw(v8__ArrayBuffer__Allocator__NewDefaultAllocator())
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
