use std::mem::drop;
use std::mem::MaybeUninit;

use crate::isolate::CxxIsolate;
use crate::isolate::LockedIsolate;
use crate::support::Scope;

extern "C" {
  fn v8__HandleScope__CONSTRUCT(
    buf: &mut MaybeUninit<CxxHandleScope>,
    isolate: &mut CxxIsolate,
  );
  fn v8__HandleScope__DESTRUCT(this: &mut CxxHandleScope);
}

#[repr(C)]
pub struct CxxHandleScope([usize; 3]);

pub struct HandleScope<'a, P> {
  parent: &'a mut P,
  cxx_handle_scope: MaybeUninit<CxxHandleScope>,
}

impl<'a, 'b, P> LockedIsolate for Scope<'a, HandleScope<'b, P>>
where
  P: LockedIsolate,
{
  fn cxx_isolate(&mut self) -> &mut CxxIsolate {
    self.0.parent.cxx_isolate()
  }
}

impl<'a, P> HandleScope<'a, P>
where
  P: LockedIsolate,
{
  pub fn new(parent: &'a mut P) -> Self {
    Self {
      parent,
      cxx_handle_scope: MaybeUninit::uninit(),
    }
  }

  pub fn enter(&mut self, mut f: impl FnMut(&mut Scope<Self>) -> ()) {
    unsafe {
      v8__HandleScope__CONSTRUCT(
        &mut self.cxx_handle_scope,
        self.parent.cxx_isolate(),
      )
    };

    let mut scope = Scope::new(self);
    f(&mut scope);
    drop(scope);

    unsafe {
      v8__HandleScope__DESTRUCT(
        &mut *(&mut self.cxx_handle_scope as *mut _ as *mut CxxHandleScope),
      )
    };
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::array_buffer::Allocator;
  use crate::isolate::*;
  use crate::platform::*;
  use crate::Locker;
  use crate::V8::*;

  #[test]
  fn test_handle_scope() {
    initialize_platform(new_default_platform());
    initialize();
    let mut params = CreateParams::new();
    params.set_array_buffer_allocator(Allocator::new_default_allocator());
    let isolate = Isolate::new(params);
    let mut locker = Locker::new(&isolate);
    HandleScope::new(&mut locker).enter(|scope| {
      HandleScope::new(scope).enter(|_scope| {});
    });
  }
}
