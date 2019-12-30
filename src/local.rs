use std::marker::PhantomData;
use std::mem::transmute;
use std::ops::Deref;
use std::ops::DerefMut;
use std::ptr::NonNull;

/// An object reference managed by the v8 garbage collector.
///
/// All objects returned from v8 have to be tracked by the garbage
/// collector so that it knows that the objects are still alive.  Also,
/// because the garbage collector may move objects, it is unsafe to
/// point directly to an object.  Instead, all objects are stored in
/// handles which are known by the garbage collector and updated
/// whenever an object moves.  Handles should always be passed by value
/// (except in cases like out-parameters) and they should never be
/// allocated on the heap.
///
/// There are two types of handles: local and persistent handles.
///
/// Local handles are light-weight and transient and typically used in
/// local operations.  They are managed by HandleScopes. That means that a
/// HandleScope must exist on the stack when they are created and that they are
/// only valid inside of the HandleScope active during their creation.
/// For passing a local handle to an outer HandleScope, an EscapableHandleScope
/// and its Escape() method must be used.
///
/// Persistent handles can be used when storing objects across several
/// independent operations and have to be explicitly deallocated when they're no
/// longer used.
///
/// It is safe to extract the object stored in the handle by
/// dereferencing the handle (for instance, to extract the *Object from
/// a Local<Object>); the value will still be governed by a handle
/// behind the scenes and the same rules apply to these values as to
/// their handles.
///
/// Note: Local handles in Rusty V8 differ from the V8 C++ API in that they are
/// never empty. In situations where empty handles are needed, use
/// Option<Local>.
#[repr(C)]
pub struct Local<'sc, T>(NonNull<T>, PhantomData<&'sc ()>);

impl<'sc, T> Copy for Local<'sc, T> {}

impl<'sc, T> Clone for Local<'sc, T> {
  fn clone(&self) -> Self {
    *self
  }
}

impl<'sc, T> Local<'sc, T> {
  /// Create a local handle by downcasting from one of its super types.
  /// This function is unsafe because the cast is unchecked.
  pub unsafe fn cast<A>(other: Local<'sc, A>) -> Self
  where
    Local<'sc, A>: From<Self>,
  {
    transmute(other)
  }

  pub(crate) unsafe fn from_raw(ptr: *mut T) -> Option<Self> {
    Some(Self(NonNull::new(ptr)?, PhantomData))
  }

  pub(crate) fn as_non_null(self) -> NonNull<T> {
    self.0
  }

  pub(crate) fn as_ptr(self) -> *mut T {
    self.0.as_ptr()
  }
}

impl<'sc, T> Deref for Local<'sc, T> {
  type Target = T;
  fn deref(&self) -> &T {
    unsafe { self.0.as_ref() }
  }
}

impl<'sc, T> DerefMut for Local<'sc, T> {
  fn deref_mut(&mut self) -> &mut T {
    unsafe { self.0.as_mut() }
  }
}

#[test]
fn test_size_of_local() {
  use crate::Value;
  use std::mem::size_of;
  assert_eq!(size_of::<Local<Value>>(), size_of::<*const Value>());
  assert_eq!(size_of::<Option<Local<Value>>>(), size_of::<*const Value>());
}
