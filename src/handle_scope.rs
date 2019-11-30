use std::marker::PhantomData;
use std::mem::MaybeUninit;

use crate::isolate::CxxIsolate;
use crate::isolate::LockedIsolate;

extern "C" {
  fn v8__HandleScope__CONSTRUCT(
    buf: &mut MaybeUninit<HandleScope>,
    isolate: &mut CxxIsolate,
  );
  fn v8__HandleScope__DESTRUCT(this: &mut HandleScope);
  fn v8__HandleScope__GetIsolate<'sc>(
    this: &'sc HandleScope,
  ) -> &'sc mut CxxIsolate;
}

#[repr(C)]
pub struct HandleScope<'sc>([usize; 3], PhantomData<&'sc mut ()>);

impl<'sc> HandleScope<'sc> {
  pub fn enter<P>(parent: &mut P, mut f: impl FnMut(&mut HandleScope<'_>) -> ())
  where
    P: LockedIsolate,
  {
    let mut scope: MaybeUninit<Self> = MaybeUninit::uninit();
    unsafe { v8__HandleScope__CONSTRUCT(&mut scope, parent.cxx_isolate()) };
    let mut scope = unsafe { &mut *(&mut scope as *mut _ as *mut HandleScope) };
    f(&mut scope);

    unsafe { v8__HandleScope__DESTRUCT(&mut scope) };
  }
}

impl<'sc> LockedIsolate for HandleScope<'sc> {
  fn cxx_isolate(&mut self) -> &mut CxxIsolate {
    unsafe { v8__HandleScope__GetIsolate(self) }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::array_buffer::Allocator;
  use crate::isolate::*;
  use crate::Integer;
  use crate::Locker;
  use crate::Number;

  #[test]
  #[allow(clippy::float_cmp)]
  fn test_handle_scope() {
    let g = crate::test_util::setup();
    let mut params = CreateParams::new();
    params.set_array_buffer_allocator(Allocator::new_default_allocator());
    let mut isolate = Isolate::new(params);
    let mut locker = Locker::new(&mut isolate);
    HandleScope::enter(&mut locker, |scope| {
      let l1 = Integer::new(scope, -123);
      let l2 = Integer::new_from_unsigned(scope, 456);
      HandleScope::enter(scope, |scope2| {
        let l3 = Number::new(scope2, 78.9);
        assert_eq!(l1.value(), -123);
        assert_eq!(l2.value(), 456);
        assert_eq!(l3.value(), 78.9);
        assert_eq!(Number::value(&l1), -123f64);
        assert_eq!(Number::value(&l2), 456f64);
      });
    });
    drop(g);
  }
}
