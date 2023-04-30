// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.
use crate::handle::UnsafeRefHandle;
use crate::isolate::BuildTypeIdHasher;
use crate::isolate::Isolate;
use crate::isolate::RawSlot;
use crate::support::int;
use crate::Context;
use crate::HandleScope;
use crate::Local;
use crate::Object;
use crate::ObjectTemplate;
use crate::Value;
use crate::Weak;
use std::any::TypeId;
use std::collections::HashMap;
use std::ffi::c_void;
use std::ptr::{null, null_mut};

extern "C" {
  fn v8__Context__New(
    isolate: *mut Isolate,
    templ: *const ObjectTemplate,
    global_object: *const Value,
  ) -> *const Context;
  fn v8__Context__GetIsolate(this: *const Context) -> *mut Isolate;
  fn v8__Context__Global(this: *const Context) -> *const Object;
  fn v8__Context__GetExtrasBindingObject(this: *const Context)
    -> *const Object;
  fn v8__Context__GetNumberOfEmbedderDataFields(this: *const Context) -> u32;
  fn v8__Context__GetAlignedPointerFromEmbedderData(
    this: *const Context,
    index: int,
  ) -> *mut c_void;
  fn v8__Context__SetAlignedPointerInEmbedderData(
    this: *const Context,
    index: int,
    value: *mut c_void,
  );
  fn v8__Context__FromSnapshot(
    isolate: *mut Isolate,
    context_snapshot_index: usize,
  ) -> *const Context;
  pub(super) fn v8__Context__GetSecurityToken(
    this: *const Context,
  ) -> *const Value;
  pub(super) fn v8__Context__SetSecurityToken(
    this: *const Context,
    value: *const Value,
  );
  pub(super) fn v8__Context__UseDefaultSecurityToken(this: *const Context);
  pub(super) fn v8__Context__AllowCodeGenerationFromStrings(
    this: *const Context,
    allow: bool,
  );
  pub(super) fn v8__Context_IsCodeGenerationFromStringsAllowed(
    this: *const Context,
  ) -> bool;
}

impl Context {
  const ANNEX_SLOT: int = 1;
  const INTERNAL_SLOT_COUNT: int = 1;

  /// Creates a new context.
  #[inline(always)]
  pub fn new<'s>(scope: &mut HandleScope<'s, ()>) -> Local<'s, Context> {
    // TODO: optional arguments;
    unsafe {
      scope
        .cast_local(|sd| v8__Context__New(sd.get_isolate_ptr(), null(), null()))
    }
    .unwrap()
  }

  /// Creates a new context using the object template as the template for
  /// the global object.
  #[inline(always)]
  pub fn new_from_template<'s>(
    scope: &mut HandleScope<'s, ()>,
    templ: Local<ObjectTemplate>,
  ) -> Local<'s, Context> {
    unsafe {
      scope.cast_local(|sd| {
        v8__Context__New(sd.get_isolate_ptr(), &*templ, null())
      })
    }
    .unwrap()
  }

  #[inline(always)]
  pub fn get_extras_binding_object<'s>(
    &self,
    scope: &mut HandleScope<'s, ()>,
  ) -> Local<'s, Object> {
    unsafe { scope.cast_local(|_| v8__Context__GetExtrasBindingObject(self)) }
      .unwrap()
  }

  /// Returns the global proxy object.
  ///
  /// Global proxy object is a thin wrapper whose prototype points to actual
  /// context's global object with the properties like Object, etc. This is done
  /// that way for security reasons (for more details see
  /// https://wiki.mozilla.org/Gecko:SplitWindow).
  ///
  /// Please note that changes to global proxy object prototype most probably
  /// would break VM---v8 expects only global object as a prototype of global
  /// proxy object.
  #[inline(always)]
  pub fn global<'s>(
    &self,
    scope: &mut HandleScope<'s, ()>,
  ) -> Local<'s, Object> {
    unsafe { scope.cast_local(|_| v8__Context__Global(self)) }.unwrap()
  }

  #[inline]
  fn get_annex_mut<'a>(
    &'a self,
    isolate: &'a mut Isolate,
    create_if_not_present: bool,
  ) -> Option<&'a mut ContextAnnex> {
    assert!(
      std::ptr::eq(isolate, unsafe { v8__Context__GetIsolate(self) }),
      "attempted to use Context slots with the wrong Isolate"
    );

    let num_data_fields =
      unsafe { v8__Context__GetNumberOfEmbedderDataFields(self) } as int;
    if num_data_fields > Self::ANNEX_SLOT {
      let annex_ptr = unsafe {
        v8__Context__GetAlignedPointerFromEmbedderData(self, Self::ANNEX_SLOT)
      } as *mut ContextAnnex;
      if !annex_ptr.is_null() {
        // SAFETY: This reference doesn't outlive the Context, so it can't outlive
        // the annex itself. Also, any mutations or accesses to the annex after
        // its creation require a mutable reference to the context's isolate, but
        // such a mutable reference is consumed by this reference during its
        // lifetime.
        return Some(unsafe { &mut *annex_ptr });
      }
    }

    if !create_if_not_present {
      return None;
    }

    let annex = Box::new(ContextAnnex {
      slots: Default::default(),
      // Gets replaced later in the method.
      self_weak: Weak::empty(isolate),
    });
    let annex_ptr = Box::into_raw(annex);
    unsafe {
      v8__Context__SetAlignedPointerInEmbedderData(
        self,
        Self::ANNEX_SLOT,
        annex_ptr as *mut _,
      )
    };
    assert!(
      unsafe { v8__Context__GetNumberOfEmbedderDataFields(self) } as int
        > Self::ANNEX_SLOT
    );

    // Make sure to drop the annex after the context is dropped, by creating a
    // weak handle with a finalizer that drops the annex, and storing the weak
    // in the annex itself.
    let weak = {
      // SAFETY: `self` can only have been derived from a `Local` or `Global`,
      // and assuming the caller is only using safe code, the `Local` or
      // `Global` must still be alive, so `self_ref_handle` won't outlive it.
      // We also check above that `isolate` is the context's isolate.
      let self_ref_handle = unsafe { UnsafeRefHandle::new(self, isolate) };

      Weak::with_guaranteed_finalizer(
        isolate,
        self_ref_handle,
        Box::new(move || {
          // SAFETY: The lifetimes of references to the annex returned by this
          // method are always tied to the context, and because this is the
          // context's finalizer, we know there are no living references to
          // the annex. And since the finalizer is only called once, the annex
          // can't have been dropped before.
          let _ = unsafe { Box::from_raw(annex_ptr) };
        }),
      )
    };

    // SAFETY: This reference doesn't outlive the Context, so it can't outlive
    // the annex itself. Also, any mutations or accesses to the annex after
    // its creation require a mutable reference to the context's isolate, but
    // such a mutable reference is consumed by this reference during its
    // lifetime.
    let annex_mut = unsafe { &mut *annex_ptr };
    annex_mut.self_weak = weak;
    Some(annex_mut)
  }

  /// Get a reference to embedder data added with [`Self::set_slot()`].
  #[inline(always)]
  pub fn get_slot<'a, T: 'static>(
    &'a self,
    isolate: &'a mut Isolate,
  ) -> Option<&'a T> {
    if let Some(annex) = self.get_annex_mut(isolate, false) {
      annex.slots.get(&TypeId::of::<T>()).map(|slot| {
        // SAFETY: `Self::set_slot` guarantees that only values of type T will be
        // stored with T's TypeId as their key.
        unsafe { slot.borrow::<T>() }
      })
    } else {
      None
    }
  }

  /// Get a mutable reference to embedder data added with [`Self::set_slot()`].
  #[inline(always)]
  pub fn get_slot_mut<'a, T: 'static>(
    &'a self,
    isolate: &'a mut Isolate,
  ) -> Option<&'a mut T> {
    if let Some(annex) = self.get_annex_mut(isolate, false) {
      annex.slots.get_mut(&TypeId::of::<T>()).map(|slot| {
        // SAFETY: `Self::set_slot` guarantees that only values of type T will be
        // stored with T's TypeId as their key.
        unsafe { slot.borrow_mut::<T>() }
      })
    } else {
      None
    }
  }

  /// Use with [`Context::get_slot`] and [`Context::get_slot_mut`] to associate
  /// state with a Context.
  ///
  /// This method gives ownership of value to the Context. Exactly one object of
  /// each type can be associated with a Context. If called more than once with
  /// an object of the same type, the earlier version will be dropped and
  /// replaced.
  ///
  /// Returns true if value was set without replacing an existing value.
  ///
  /// The value will be dropped when the context is garbage collected.
  #[inline(always)]
  pub fn set_slot<'a, T: 'static>(
    &'a self,
    isolate: &'a mut Isolate,
    value: T,
  ) -> bool {
    self
      .get_annex_mut(isolate, true)
      .unwrap()
      .slots
      .insert(TypeId::of::<T>(), RawSlot::new(value))
      .is_none()
  }

  /// Removes the embedder data added with [`Self::set_slot()`] and returns it
  /// if it exists.
  #[inline(always)]
  pub fn remove_slot<'a, T: 'static>(
    &'a self,
    isolate: &'a mut Isolate,
  ) -> Option<T> {
    if let Some(annex) = self.get_annex_mut(isolate, false) {
      annex.slots.remove(&TypeId::of::<T>()).map(|slot| {
        // SAFETY: `Self::set_slot` guarantees that only values of type T will be
        // stored with T's TypeId as their key.
        unsafe { slot.into_inner::<T>() }
      })
    } else {
      None
    }
  }

  /// Removes all embedder data added with [`Self::set_slot()`], and
  /// deletes any internal state needed to keep track of such slots.
  ///
  /// This is needed to make a snapshot with
  /// [`SnapshotCreator`](crate::SnapshotCreator), since the internal embedder
  /// state uses [`Weak`] handles, which cannot be alive at the time of
  /// snapshotting.
  #[inline(always)]
  pub fn clear_all_slots<'a>(&'a self, isolate: &'a mut Isolate) {
    if let Some(annex_mut) = self.get_annex_mut(isolate, false) {
      let annex_ptr = annex_mut as *mut ContextAnnex;
      let _ = unsafe { Box::from_raw(annex_ptr) };
      unsafe {
        v8__Context__SetAlignedPointerInEmbedderData(
          self,
          Self::ANNEX_SLOT,
          null_mut(),
        )
      };
    }
  }

  #[inline(always)]
  pub unsafe fn set_aligned_pointer_in_embedder_data(
    &self,
    slot: i32,
    data: *mut c_void,
  ) {
    v8__Context__SetAlignedPointerInEmbedderData(
      self,
      slot + Self::INTERNAL_SLOT_COUNT,
      data,
    )
  }

  #[inline(always)]
  pub fn get_aligned_pointer_from_embedder_data(
    &self,
    slot: i32,
  ) -> *mut c_void {
    unsafe {
      v8__Context__GetAlignedPointerFromEmbedderData(
        self,
        slot + Self::INTERNAL_SLOT_COUNT,
      )
    }
  }

  /// Create a new context from a (non-default) context snapshot. There
  /// is no way to provide a global object template since we do not create
  /// a new global object from template, but we can reuse a global object.
  pub fn from_snapshot<'s>(
    scope: &mut HandleScope<'s, ()>,
    context_snapshot_index: usize,
  ) -> Option<Local<'s, Context>> {
    unsafe {
      scope.cast_local(|sd| {
        v8__Context__FromSnapshot(sd.get_isolate_mut(), context_snapshot_index)
      })
    }
  }

  #[inline(always)]
  pub fn get_security_token<'s>(
    &self,
    scope: &mut HandleScope<'s, ()>,
  ) -> Local<'s, Value> {
    unsafe { scope.cast_local(|_| v8__Context__GetSecurityToken(self)) }
      .unwrap()
  }

  #[inline(always)]
  pub fn set_security_token(&self, token: Local<Value>) {
    unsafe {
      v8__Context__SetSecurityToken(self, &*token);
    }
  }

  #[inline(always)]
  pub fn use_default_security_token(&self) {
    unsafe {
      v8__Context__UseDefaultSecurityToken(self);
    }
  }

  pub fn set_allow_generation_from_strings(&self, allow: bool) {
    unsafe {
      v8__Context__AllowCodeGenerationFromStrings(self, allow);
    }
  }

  pub fn is_code_generation_from_strings_allowed(&self) -> bool {
    unsafe { v8__Context_IsCodeGenerationFromStringsAllowed(self) }
  }
}

struct ContextAnnex {
  slots: HashMap<TypeId, RawSlot, BuildTypeIdHasher>,
  // In order to run the finalizer that drops the ContextAnnex when the Context
  // is GC'd, the corresponding Weak must be kept alive until that time.
  self_weak: Weak<Context>,
}
