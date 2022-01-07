use std::borrow::Borrow;
use std::hash::Hash;
use std::hash::Hasher;
use std::marker::PhantomData;
use std::mem::transmute;
use std::ops::Deref;
use std::ptr::NonNull;
use std::sync::Arc;
use std::sync::Weak;

use crate::isolate::IsolateAnnex;
use crate::isolate::Locker;
use crate::Data;
use crate::HandleScope;
use crate::Isolate;
use crate::IsolateHandle;

extern "C" {
  fn v8__Local__New(isolate: *mut Isolate, other: *const Data) -> *const Data;
  fn v8__Global__New(isolate: *mut Isolate, data: *const Data) -> *const Data;
  fn v8__Global__Reset(data: *const Data);
}

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
/// only valid inside of the `HandleScope` active during their creation.
/// For passing a local handle to an outer `HandleScope`, an
/// `EscapableHandleScope` and its `Escape()` method must be used.
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
#[derive(Debug)]
pub struct Local<'s, T>(NonNull<T>, PhantomData<&'s ()>);

impl<'s, T> Local<'s, T> {
  /// Construct a new Local from an existing Handle.
  pub fn new(
    scope: &mut HandleScope<'s, ()>,
    handle: impl Handle<Data = T>,
  ) -> Self {
    let HandleInfo { data, host } = handle.get_handle_info();
    host.assert_match_isolate(scope);
    unsafe {
      scope.cast_local(|sd| {
        v8__Local__New(sd.get_isolate_ptr(), data.cast().as_ptr()) as *const T
      })
    }
    .unwrap()
  }

  /// Create a local handle by downcasting from one of its super types.
  /// This function is unsafe because the cast is unchecked.
  pub unsafe fn cast<A>(other: Local<'s, A>) -> Self
  where
    Local<'s, A>: From<Self>,
  {
    transmute(other)
  }

  pub(crate) unsafe fn from_raw(ptr: *const T) -> Option<Self> {
    NonNull::new(ptr as *mut _).map(|nn| Self::from_non_null(nn))
  }

  pub(crate) unsafe fn from_non_null(nn: NonNull<T>) -> Self {
    Self(nn, PhantomData)
  }

  pub(crate) fn as_non_null(self) -> NonNull<T> {
    self.0
  }

  pub(crate) fn slice_into_raw(slice: &[Self]) -> &[*const T] {
    unsafe { &*(slice as *const [Self] as *const [*const T]) }
  }
}

impl<'s, T> Copy for Local<'s, T> {}

impl<'s, T> Clone for Local<'s, T> {
  fn clone(&self) -> Self {
    *self
  }
}

impl<'s, T> Deref for Local<'s, T> {
  type Target = T;
  fn deref(&self) -> &T {
    unsafe { self.0.as_ref() }
  }
}

/// An object reference that is independent of any handle scope. Where
/// a Local handle only lives as long as the HandleScope in which it was
/// allocated, a global handle remains valid until it is explicitly
/// disposed using reset().
///
/// A global handle contains a reference to a storage cell within
/// the V8 engine which holds an object value and which is updated by
/// the garbage collector whenever the object is moved.
///
/// rusty_v8 note: Care must be taken to ensure the global handle is only used
/// in contexts where the holding Isolate is locked and entered. Otherwise, a
/// runtime assertion will be triggered, and the thread will panic. Unlike the
/// V8 C++ API, Global handles are reset when dropped. Extra care must be taken
/// to ensure Global handles are dropped while the associated Isolate is
/// entered, or after the Isolate has been dropped and disposed.
#[derive(Debug)]
pub struct Global<T: 'static> {
  data: NonNull<T>,
  isolate_handle: Weak<IsolateAnnex>,
}

// Global is marked as Send + Sync, but care must be taken to ensure the holding
// isolate is locked and entered before interacting with it.
unsafe impl<T> Send for Global<T> {}
unsafe impl<T> Sync for Global<T> {}

impl<T> Global<T> {
  /// Construct a new Global from an existing Handle.
  pub fn new(isolate: &mut Isolate, handle: impl Handle<Data = T>) -> Self {
    let HandleInfo { data, host } = handle.get_handle_info();
    host.assert_match_isolate(isolate);
    unsafe { Self::new_raw(isolate, data) }
  }

  /// Implementation helper function that contains the code that can be shared
  /// between `Global::new()` and `Global::clone()`.
  unsafe fn new_raw(isolate: *mut Isolate, data: NonNull<T>) -> Self {
    let data = data.cast().as_ptr();
    let data = v8__Global__New(isolate, data) as *const T;
    let data = NonNull::new_unchecked(data as *mut _);
    let isolate_handle = (*isolate).get_annex_weak();
    Self {
      data,
      isolate_handle,
    }
  }

  pub fn open<'a>(&'a self, scope: &mut Isolate) -> &'a T {
    Handle::open(self, scope)
  }

  #[deprecated = "use Global::open() instead"]
  pub fn get<'a>(&'a self, scope: &mut Isolate) -> &'a T {
    Handle::open(self, scope)
  }
}

impl<T> Global<T> {
  pub(crate) unsafe fn reset(this: *mut T) {
    v8__Global__Reset(this.cast());
  }
}

impl<T> Clone for Global<T> {
  fn clone(&self) -> Self {
    let HandleInfo { data, host } = self.get_handle_info();
    unsafe { Self::new_raw(host.get_isolate().as_mut(), data) }
  }
}

impl<T> Drop for Global<T> {
  fn drop(&mut self) {
    let HandleInfo { data, host } = self.get_handle_info();
    match host {
      // This `Global` handle is associated with an `Isolate` that has already
      // been disposed.
      HandleHost::DisposedIsolate => {}
      HandleHost::UnlockedIsolate(annex) => {
        annex
          .upgrade()
          .expect("invariant: expected annex for unlocked isolate")
          .mark_handle_data_for_disposal(self.data);
      }
      _ => unsafe { Self::reset(data.as_ptr()) },
    }
  }
}

pub trait Handle: Sized {
  type Data;

  #[doc(hidden)]
  fn get_handle_info(&self) -> HandleInfo<Self::Data>;

  /// Returns a reference to the V8 heap object that this handle represents.
  /// The handle does not get cloned, nor is it converted to a `Local` handle.
  ///
  /// # Panics
  ///
  /// This function panics in the following situations:
  /// - The handle is not hosted by the specified Isolate.
  /// - The Isolate that hosts this handle has been disposed.
  fn open<'a>(&'a self, isolate: &mut Isolate) -> &'a Self::Data {
    let HandleInfo { data, host } = self.get_handle_info();
    host.assert_match_isolate(isolate);
    unsafe { &*data.as_ptr() }
  }

  /// Reads the inner value contained in this handle, _without_ verifying that
  /// the this handle is hosted by the currently active `Isolate`.
  ///
  /// # Safety
  ///
  /// Using a V8 heap object with another `Isolate` than the `Isolate` that
  /// hosts it is not permitted under any circumstance. Doing so leads to
  /// undefined behavior, likely a crash.
  ///
  /// # Panics
  ///
  /// This function panics if the `Isolate` that hosts the handle has been
  /// disposed.
  unsafe fn get_unchecked(&self) -> &Self::Data {
    let HandleInfo { data, host } = self.get_handle_info();
    if let HandleHost::DisposedIsolate = host {
      panic!("attempt to access Handle hosted by disposed Isolate");
    }
    &*data.as_ptr()
  }
}

impl<'s, T> Handle for Local<'s, T> {
  type Data = T;
  fn get_handle_info(&self) -> HandleInfo<T> {
    HandleInfo::new(self.as_non_null(), HandleHost::Scope)
  }
}

impl<'a, 's: 'a, T> Handle for &'a Local<'s, T> {
  type Data = T;
  fn get_handle_info(&self) -> HandleInfo<T> {
    HandleInfo::new(self.as_non_null(), HandleHost::Scope)
  }
}

impl<T: Sized> Handle for Global<T> {
  type Data = T;
  fn get_handle_info(&self) -> HandleInfo<T> {
    HandleInfo::new(self.data, (&self.isolate_handle).into())
  }
}

impl<'a, T: Sized> Handle for &'a Global<T> {
  type Data = T;
  fn get_handle_info(&self) -> HandleInfo<T> {
    HandleInfo::new(self.data, (&self.isolate_handle).into())
  }
}

impl<T: Sized> Handle for Arc<Global<T>> {
  type Data = T;
  fn get_handle_info(&self) -> HandleInfo<T> {
    HandleInfo::new(self.data, (&self.isolate_handle).into())
  }
}

impl<'s, T> Borrow<T> for Local<'s, T> {
  fn borrow(&self) -> &T {
    &**self
  }
}

impl<T> Borrow<T> for Global<T> {
  fn borrow(&self) -> &T {
    let HandleInfo { data, host } = self.get_handle_info();
    if let HandleHost::DisposedIsolate = host {
      panic!("attempt to access Handle hosted by disposed Isolate");
    } else if let HandleHost::UnlockedIsolate(_) = host {
      panic!("attempt to access Handle outside of locked Isolate")
    }
    unsafe { &*data.as_ptr() }
  }
}

impl<'s, T> Eq for Local<'s, T> where T: Eq {}
impl<T> Eq for Global<T> where T: Eq {}

impl<'s, T: Hash> Hash for Local<'s, T> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    (&**self).hash(state)
  }
}

impl<T: Hash> Hash for Global<T> {
  fn hash<H: Hasher>(&self, state: &mut H) {
    (self.borrow() as &T).hash(state);
  }
}

impl<'s, T, Rhs: Handle> PartialEq<Rhs> for Local<'s, T>
where
  T: PartialEq<Rhs::Data>,
{
  fn eq(&self, other: &Rhs) -> bool {
    let i1 = self.get_handle_info();
    let i2 = other.get_handle_info();
    i1.host.match_host(i2.host, None)
      && unsafe { i1.data.as_ref() == i2.data.as_ref() }
  }
}

impl<'s, T, Rhs: Handle> PartialEq<Rhs> for Global<T>
where
  T: PartialEq<Rhs::Data>,
{
  fn eq(&self, other: &Rhs) -> bool {
    let i1 = self.get_handle_info();
    let i2 = other.get_handle_info();
    i1.host.match_host(i2.host, None)
      && unsafe { i1.data.as_ref() == i2.data.as_ref() }
  }
}

#[derive(Debug, Clone)]
pub struct HandleInfo<T> {
  data: NonNull<T>,
  host: HandleHost,
}

impl<T> HandleInfo<T> {
  fn new(data: NonNull<T>, host: HandleHost) -> Self {
    Self { data, host }
  }
}

#[derive(Debug, Clone)]
enum HandleHost {
  // Note: the `HandleHost::Scope` variant does not indicate that the handle
  // it applies to is not associated with an `Isolate`. It only means that
  // the handle is a `Local` handle that was unable to provide a pointer to
  // the `Isolate` that hosts it (the handle) and the currently entered
  // scope.
  Scope,
  Isolate(NonNull<Isolate>),
  UnlockedIsolate(Weak<IsolateAnnex>),
  DisposedIsolate,
}

impl From<&'_ mut Isolate> for HandleHost {
  fn from(isolate: &'_ mut Isolate) -> Self {
    Self::Isolate(NonNull::from(isolate))
  }
}

impl From<&'_ IsolateHandle> for HandleHost {
  fn from(isolate_handle: &IsolateHandle) -> Self {
    let isolate_ptr = unsafe { isolate_handle.get_isolate_ptr() };
    if isolate_ptr.is_null() {
      Self::DisposedIsolate
    } else if !isolate_handle.is_locked() {
      Self::UnlockedIsolate(unsafe {
        isolate_ptr.as_ref().unwrap().get_annex_weak()
      })
    } else {
      Self::Isolate(NonNull::new(isolate_ptr).unwrap())
    }
  }
}

impl From<&'_ mut Locker> for HandleHost {
  fn from(locker: &'_ mut Locker) -> Self {
    Self::Isolate(NonNull::from(locker.get_isolate()))
  }
}

impl From<&Weak<IsolateAnnex>> for HandleHost {
  fn from(annex: &Weak<IsolateAnnex>) -> Self {
    match annex.upgrade() {
      Some(annex) => {
        let isolate_ptr = unsafe { annex.get_isolate_ptr() };
        if isolate_ptr.is_null() {
          Self::DisposedIsolate
        } else if unsafe { !Locker::is_locked(isolate_ptr) } {
          Self::UnlockedIsolate(unsafe {
            isolate_ptr.as_ref().unwrap().get_annex_weak()
          })
        } else {
          Self::Isolate(NonNull::new(isolate_ptr).unwrap())
        }
      }
      None => Self::DisposedIsolate,
    }
  }
}

impl HandleHost {
  /// Compares two `HandleHost` values, returning `true` if they refer to the
  /// same `Isolate`, or `false` if they refer to different isolates.
  ///
  /// If the caller knows which `Isolate` the currently entered scope (if any)
  /// belongs to, it should pass on this information via the second argument
  /// (`scope_isolate_opt`).
  ///
  /// # Panics
  ///
  /// This function panics if one of the `HandleHost` values refers to an
  /// `Isolate` that has been disposed.
  ///
  /// # Safety / Bugs
  ///
  /// The current implementation is a bit too forgiving. If it cannot decide
  /// whether two hosts refer to the same `Isolate`, it just returns `true`.
  /// Note that this can only happen when the caller does _not_ provide a value
  /// for the `scope_isolate_opt` argument.
  fn match_host(
    self,
    other: Self,
    scope_isolate_opt: Option<&mut Isolate>,
  ) -> bool {
    let scope_isolate_opt_nn = scope_isolate_opt.map(NonNull::from);
    match (self, other, scope_isolate_opt_nn) {
      (Self::Scope, Self::Scope, _) => true,
      (Self::Isolate(ile1), Self::Isolate(ile2), _) => ile1 == ile2,
      (Self::Scope, Self::Isolate(ile1), Some(ile2)) => ile1 == ile2,
      (Self::Isolate(ile1), Self::Scope, Some(ile2)) => ile1 == ile2,
      // TODO(pisciaureus): If the caller didn't provide a `scope_isolate_opt`
      // value that works, we can't do a meaningful check. So all we do for now
      // is pretend the Isolates match and hope for the best. This eventually
      // needs to be tightened up.
      (Self::Scope, Self::Isolate(_), _) => true,
      (Self::Isolate(_), Self::Scope, _) => true,
      // Handles must be used within the scope of an active Locker and entered
      // isolate. Attempting to use a handle otherwise is unsafe.
      (Self::UnlockedIsolate(_), ..) | (_, Self::UnlockedIsolate(_), _) => {
        panic!("attempt to access Handle outside of locked Isolate")
      }
      // Handles hosted in an Isolate that has been disposed aren't good for
      // anything, even if a pair of handles used to to be hosted in the same
      // now-disposed isolate.
      (Self::DisposedIsolate, ..) | (_, Self::DisposedIsolate, _) => {
        panic!("attempt to access Handle hosted by disposed Isolate")
      }
    }
  }

  fn assert_match_host(self, other: Self, scope_opt: Option<&mut Isolate>) {
    assert!(
      self.match_host(other, scope_opt),
      "attempt to use Handle in an Isolate that is not its host"
    )
  }

  #[allow(dead_code)]
  fn match_isolate(self, isolate: &mut Isolate) -> bool {
    self.match_host(isolate.into(), Some(isolate))
  }

  fn assert_match_isolate(self, isolate: &mut Isolate) {
    self.assert_match_host(isolate.into(), Some(isolate))
  }

  fn get_isolate(self) -> NonNull<Isolate> {
    match self {
      Self::Scope => panic!("host Isolate for Handle not available"),
      Self::Isolate(ile) => ile,
      Self::UnlockedIsolate(_) => panic!("attempt to access unlocked Isolate"),
      Self::DisposedIsolate => panic!("attempt to access disposed Isolate"),
    }
  }

  #[allow(dead_code)]
  fn get_isolate_handle(self) -> Weak<IsolateAnnex> {
    unsafe { self.get_isolate().as_ref() }.get_annex_weak()
  }
}
