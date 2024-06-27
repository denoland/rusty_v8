// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license

use crate::platform::Platform;
use crate::support::int;
use crate::support::Opaque;
use crate::support::SharedRef;
use crate::support::UniqueRef;
use crate::Data;
use crate::TracedReference;
use std::marker::PhantomData;

extern "C" {
  fn cppgc__initialize_process(platform: *mut Platform);
  fn cppgc__shutdown_process();

  fn cppgc__heap__create(
    platform: *mut Platform,
    marking_support: MarkingType,
    sweeping_support: SweepingType,
  ) -> *mut Heap;
  fn cppgc__heap__DELETE(heap: *mut Heap);
  fn cppgc__make_garbage_collectable(
    heap: *mut Heap,
    size: usize,
    trace: TraceFn,
    destroy: DestroyFn,
  ) -> *mut RustObj;

  fn cppgc__heap__enable_detached_garbage_collections_for_testing(
    heap: *mut Heap,
  );
  fn cppgc__heap__collect_garbage_for_testing(
    heap: *mut Heap,
    stack_state: EmbedderStackState,
  );

  fn cppgc__Visitor__Trace__Member(
    visitor: *const Visitor,
    member: *const MemberInner,
  );
  fn cppgc__Visitor__Trace__WeakMember(
    visitor: *const Visitor,
    member: *const WeakMemberInner,
  );
  fn cppgc__Visitor__Trace__TracedReference(
    visitor: *const Visitor,
    reference: *const TracedReference<Data>,
  );

  fn cppgc__Member__CONSTRUCT(member: *mut MemberInner, obj: *mut RustObj);
  fn cppgc__Member__DESTRUCT(member: *mut MemberInner);
  fn cppgc__Member__Get(member: *const MemberInner) -> *mut RustObj;
  fn cppgc__Member__Assign(member: *mut MemberInner, other: *mut RustObj);

  fn cppgc__WeakMember__CONSTRUCT(
    member: *mut WeakMemberInner,
    obj: *mut RustObj,
  );
  fn cppgc__WeakMember__DESTRUCT(member: *mut WeakMemberInner);
  fn cppgc__WeakMember__Get(member: *const WeakMemberInner) -> *mut RustObj;
  fn cppgc__WeakMember__Assign(
    member: *mut WeakMemberInner,
    other: *mut RustObj,
  );

  fn cppgc__Persistent__CONSTRUCT() -> *mut PersistentInner;
  fn cppgc__Persistent__DESTRUCT(this: *mut PersistentInner);
  fn cppgc__Persistent__Assign(this: *mut PersistentInner, ptr: *mut RustObj);
  fn cppgc__Persistent__Get(this: *const PersistentInner) -> *mut RustObj;

  fn cppgc__WeakPersistent__CONSTRUCT() -> *mut WeakPersistentInner;
  fn cppgc__WeakPersistent__DESTRUCT(this: *mut WeakPersistentInner);
  fn cppgc__WeakPersistent__Assign(
    this: *mut WeakPersistentInner,
    ptr: *mut RustObj,
  );
  fn cppgc__WeakPersistent__Get(
    this: *const WeakPersistentInner,
  ) -> *mut RustObj;
}

type TraceFn = unsafe extern "C" fn(*const RustObj, *mut Visitor);
type DestroyFn = unsafe extern "C" fn(*const RustObj);

#[doc(hidden)]
#[repr(C)]
pub struct RustObj {
  trace: TraceFn,
  destroy: DestroyFn,
}

fn object_offset_for_rust_obj<T: GarbageCollected>() -> usize {
  #[repr(C)]
  struct Calc<T> {
    header: RustObj,
    data: T,
  }

  std::mem::offset_of!(Calc<T>, data)
}

fn get_object_from_rust_obj<T: GarbageCollected>(
  rust_obj: *const RustObj,
) -> *mut T {
  unsafe { rust_obj.byte_add(object_offset_for_rust_obj::<T>()) as *mut T }
}

/// Process-global initialization of the garbage collector. Must be called before
/// creating a Heap.
///
/// Can be called multiple times when paired with `ShutdownProcess()`.
pub fn initalize_process(platform: SharedRef<Platform>) {
  unsafe {
    cppgc__initialize_process(&*platform as *const Platform as *mut _);
  }
}

/// # Safety
///
/// Must be called after destroying the last used heap. Some process-global
/// metadata may not be returned and reused upon a subsequent
/// `initalize_process()` call.
pub unsafe fn shutdown_process() {
  cppgc__shutdown_process();
}

/// Visitor passed to trace methods. All managed pointers must have called the
/// Visitor's trace method on them.
///
/// ```
/// use v8::cppgc::{Member, Visitor, GarbageCollected};
///
/// struct Foo { foo: Member<Foo> }
///
/// impl GarbageCollected for Foo {
///   fn trace(&self, visitor: &Visitor) {
///     visitor.trace(&self.foo);
///   }
/// }
/// ```
#[repr(C)]
#[derive(Debug)]
pub struct Visitor(Opaque);

impl Visitor {
  #[inline(always)]
  pub fn trace(&self, member: &impl Traced) {
    member.trace(self);
  }
}

#[doc(hidden)]
pub trait Traced {
  fn trace(&self, visitor: &Visitor);
}

impl<T: GarbageCollected> Traced for Member<T> {
  fn trace(&self, visitor: &Visitor) {
    unsafe { cppgc__Visitor__Trace__Member(visitor, &self.inner) }
  }
}

impl<T: GarbageCollected> Traced for WeakMember<T> {
  fn trace(&self, visitor: &Visitor) {
    unsafe { cppgc__Visitor__Trace__WeakMember(visitor, &self.inner) }
  }
}

impl<T> Traced for TracedReference<T> {
  fn trace(&self, visitor: &Visitor) {
    unsafe {
      cppgc__Visitor__Trace__TracedReference(
        visitor,
        self as *const TracedReference<T> as *const TracedReference<Data>,
      )
    }
  }
}

#[repr(C)]
pub enum EmbedderStackState {
  /// Stack may contain interesting heap pointers.
  MayContainHeapPointers,
  /// Stack does not contain any interesting heap pointers.
  NoHeapPointers,
}

/// Specifies supported marking types.
#[repr(u8)]
pub enum MarkingType {
  /// Atomic stop-the-world marking. This option does not require any write barriers but is the most intrusive in terms of jank.
  Atomic,
  /// Incremental marking interleaves marking with the rest of the application workload on the same thread.
  Incremental,
  /// Incremental and concurrent marking.
  IncrementalAndConcurrent,
}

/// Specifies supported sweeping types.
#[repr(u8)]
pub enum SweepingType {
  /// Atomic stop-the-world sweeping. All of sweeping is performed at once.
  Atomic,
  /// Incremental sweeping interleaves sweeping with the rest of the application workload on the same thread.
  Incremental,
  /// Incremental and concurrent sweeping. Sweeping is split and interleaved with the rest of the application.
  IncrementalAndConcurrent,
}

pub type InternalFieldIndex = int;

pub struct HeapCreateParams {
  /// Specifies which kind of marking are supported by the heap.
  pub marking_support: MarkingType,
  /// Specifies which kind of sweeping are supported by the heap.
  pub sweeping_support: SweepingType,
}

impl Default for HeapCreateParams {
  fn default() -> Self {
    Self {
      marking_support: MarkingType::IncrementalAndConcurrent,
      sweeping_support: SweepingType::IncrementalAndConcurrent,
    }
  }
}

/// A heap for allocating managed C++ objects.
///
/// Similar to v8::Isolate, the heap may only be accessed from one thread at a
/// time.
#[repr(C)]
#[derive(Debug)]
pub struct Heap(Opaque);

impl Drop for Heap {
  fn drop(&mut self) {
    unsafe { cppgc__heap__DELETE(self) }
  }
}

impl Heap {
  pub fn create(
    platform: SharedRef<Platform>,
    params: HeapCreateParams,
  ) -> UniqueRef<Heap> {
    unsafe {
      UniqueRef::from_raw(cppgc__heap__create(
        &*platform as *const Platform as *mut _,
        params.marking_support,
        params.sweeping_support,
      ))
    }
  }

  pub unsafe fn collect_garbage_for_testing(
    &self,
    stack_state: EmbedderStackState,
  ) {
    unsafe {
      cppgc__heap__collect_garbage_for_testing(
        self as *const Heap as *mut _,
        stack_state,
      );
    }
  }

  pub fn enable_detached_garbage_collections_for_testing(&self) {
    unsafe {
      cppgc__heap__enable_detached_garbage_collections_for_testing(
        self as *const Heap as *mut _,
      );
    }
  }
}

/// Base trait for managed objects.
pub trait GarbageCollected {
  fn trace(&self, _visitor: &Visitor) {}
}

/// Constructs an instance of T, which is a garbage collected type.
///
/// The object will be allocated on the heap and managed by cppgc. During
/// marking, the object will be traced by calling the `trace` method on it.
///
/// During sweeping, the destructor will be called and the memory will be
/// freed.
///
/// # Safety
///
/// The caller must ensure that the returned pointer is always stored on
/// the stack, or moved into one of the Persistent types.
pub unsafe fn make_garbage_collected<T: GarbageCollected>(
  heap: &Heap,
  obj: T,
) -> Member<T> {
  unsafe extern "C" fn trace<T: GarbageCollected>(
    obj: *const RustObj,
    visitor: *mut Visitor,
  ) {
    let obj = unsafe { &*get_object_from_rust_obj::<T>(obj) };
    obj.trace(unsafe { &*visitor });
  }

  unsafe extern "C" fn destroy<T: GarbageCollected>(obj: *const RustObj) {
    let obj = get_object_from_rust_obj::<T>(obj);
    std::ptr::drop_in_place(obj);
  }

  let additional_bytes = (object_offset_for_rust_obj::<T>()
    - std::mem::size_of::<RustObj>())
    + std::mem::size_of::<T>();

  let handle = unsafe {
    cppgc__make_garbage_collectable(
      heap as *const Heap as *mut _,
      additional_bytes,
      trace::<T>,
      destroy::<T>,
    )
  };

  unsafe {
    get_object_from_rust_obj::<T>(handle).write(obj);
  }

  Member::new(handle)
}

#[doc(hidden)]
pub trait GetRustObj<T: GarbageCollected> {
  fn get_rust_obj(&self) -> *mut RustObj;
}

macro_rules! member {
  ($( # $attr:tt )* $name:ident) => {
    paste::paste! {
      #[repr(transparent)]
      struct [< $name Inner >]([u8; crate::binding:: [< RUST_cppgc__ $name _SIZE >]]);

      impl [< $name Inner >] {
        fn new(ptr: *mut RustObj) -> Self {
          let mut this = std::mem::MaybeUninit::uninit();
          unsafe {
            [< cppgc__ $name __CONSTRUCT >](this.as_mut_ptr(), ptr);
            this.assume_init()
          }
        }

        #[inline(always)]
        fn get(&self) -> *mut RustObj {
          // Member may be a compressed pointer, so just read it from C++
          unsafe { [< cppgc__ $name __Get >](self) }
        }

        #[inline(always)]
        fn assign(&mut self, ptr: *mut RustObj) {
          // Assignment has write barriers in the GC, so call into C++
          unsafe {
            [< cppgc__ $name __Assign >](self, ptr);
          }
        }
      }

      impl Drop for [< $name Inner >] {
        fn drop(&mut self) {
          unsafe {
            [< cppgc__ $name __DESTRUCT >](self);
          }
        }
      }

      $( # $attr )*
      #[repr(transparent)]
      pub struct $name<T: GarbageCollected> {
        inner: [< $name Inner >],
        _phantom: PhantomData<T>,
      }

      impl<T: GarbageCollected> $name<T> {
        pub(crate) fn new(obj: *mut RustObj) -> Self {
          Self {
            inner: [< $name Inner >]::new(obj),
            _phantom: PhantomData,
          }
        }

        #[doc = "Create a new empty "]
        #[doc = stringify!($name)]
        #[doc = " which may be set later."]
        pub fn empty() -> Self {
          Self::new(std::ptr::null_mut())
        }

        #[doc = "Set the object pointed to by this "]
        #[doc = stringify!($name)]
        #[doc = "."]
        pub fn set(&mut self, other: &impl GetRustObj<T>) {
          let ptr = other.get_rust_obj();
          self.inner.assign(ptr);
        }

        #[doc = "Borrow the object pointed to by this "]
        #[doc = stringify!($name)]
        #[doc = "."]
        pub fn borrow(&self) -> Option<&T> {
          let ptr = self.inner.get();
          if ptr.is_null() {
            None
          } else {
            // SAFETY: Either this is a strong reference and the pointer is always valid
            // or this is a weak reference and the ptr will be null if it was collected.
            Some(unsafe { &*get_object_from_rust_obj(ptr) })
          }
        }
      }

      impl<T: GarbageCollected> GetRustObj<T> for $name<T> {
        fn get_rust_obj(&self) -> *mut RustObj {
          self.inner.get()
        }
      }

      impl<T: GarbageCollected> std::fmt::Debug for $name<T> {
        fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
          fmt.debug_struct(stringify!($name)).finish()
        }
      }
    }
  }
}

member! {
  /// Members are used in classes to contain strong pointers to other garbage
  /// collected objects. All Member fields of a class must be traced in the class'
  /// trace method.
  Member
}

member! {
  /// WeakMember is similar to Member in that it is used to point to other garbage
  /// collected objects. However instead of creating a strong pointer to the
  /// object, the WeakMember creates a weak pointer, which does not keep the
  /// pointee alive. Hence if all pointers to to a heap allocated object are weak
  /// the object will be garbage collected. At the time of GC the weak pointers
  /// will automatically be set to null.
  WeakMember
}

macro_rules! persistent {
  ($( # $attr:tt )* $name:ident) => {
    paste::paste! {
      // PersistentBase is extremely particular about move and copy semantics,
      // so we allocate it on the heap and only interact with it via calls to C++.
      #[repr(C)]
      struct [< $name Inner >](Opaque);

      $( # $attr )*
      pub struct $name<T: GarbageCollected> {
        inner: *mut [< $name Inner >],
        _phantom: PhantomData<T>,
      }

      impl<T: GarbageCollected> $name<T> {
        #[doc = "Create a new empty "]
        #[doc = stringify!($name)]
        #[doc = " which may be set later."]
        pub fn empty() -> Self {
          let this = unsafe { [< cppgc__ $name __CONSTRUCT >]() };
          Self {
            inner: this,
            _phantom: PhantomData,
          }
        }

        #[doc = "Set the object pointed to by this "]
        #[doc = stringify!($name)]
        #[doc = "."]
        pub fn set(&mut self, other: &impl GetRustObj<T>) {
          let ptr = other.get_rust_obj();
          self.assign(ptr);
        }

        #[doc = "Borrow the object pointed to by this "]
        #[doc = stringify!($name)]
        #[doc = "."]
        pub fn borrow(&self) -> Option<&T> {
          let ptr = self.get();
          if ptr.is_null() {
            None
          } else {
            // SAFETY: Either this is a strong reference and the pointer is always valid
            // or this is a weak reference and the ptr will be null if it was collected.
            Some(unsafe { &*get_object_from_rust_obj(ptr) })
          }
        }

        #[inline(always)]
        fn assign(&mut self, ptr: *mut RustObj) {
          unsafe {
            [< cppgc__ $name __Assign >](self.inner, ptr);
          }
        }

        #[inline(always)]
        fn get(&self) -> *mut RustObj {
          unsafe {
            [< cppgc__ $name __Get >](self.inner)
          }
        }

      }

      impl<T: GarbageCollected> Drop for $name<T> {
        fn drop(&mut self) {
          unsafe {
            [< cppgc__ $name __DESTRUCT >](self.inner);
          }
        }
      }

      impl<T: GarbageCollected> std::fmt::Debug for $name<T> {
        fn fmt(&self, fmt: &mut std::fmt::Formatter) -> std::fmt::Result {
          fmt.debug_struct(stringify!($name)).finish()
        }
      }

      impl<T: GarbageCollected> GetRustObj<T> for $name<T> {
        fn get_rust_obj(&self) -> *mut RustObj {
          self.get()
        }
      }
    }
  };
}

persistent! {
  /// Persistent is a way to create a strong pointer from an off-heap object to
  /// another on-heap object. As long as the Persistent handle is alive the GC will
  /// keep the object pointed to alive. The Persistent handle is always a GC root
  /// from the point of view of the GC. Persistent must be constructed and
  /// destructed in the same thread.
  Persistent
}

persistent! {
  /// WeakPersistent is a way to create a weak pointer from an off-heap object to
  /// an on-heap object. The pointer is automatically cleared when the pointee gets
  /// collected. WeakPersistent must be constructed and destructed in the same
  /// thread.
  WeakPersistent
}
