// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license

use crate::platform::Platform;
use crate::support::Opaque;
use crate::support::SharedRef;
use crate::support::UniqueRef;

extern "C" {
  fn cppgc__initialize_process(platform: *mut Platform);
  fn cppgc__shutdown_process();

  fn cppgc__heap__create(platform: *mut Platform) -> *mut Heap;
  fn cppgc__heap__DELETE(heap: *mut Heap);
  fn cppgc__make_garbage_collectable(
    heap: *mut Heap,
    obj: *mut (),
    trace: TraceFn,
    destroy: DestroyFn,
  ) -> *mut ();

  fn cppgc__heap__enable_detached_garbage_collections_for_testing(
    heap: *mut Heap,
  );
  fn cppgc__heap__collect_garbage_for_testing(
    heap: *mut Heap,
    stack_state: EmbedderStackState,
  );

  fn cppgc__visitor__trace(visitor: *const Visitor, member: *const ());
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

/// # SAFETY
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
/// ```no_run
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
  pub fn trace<T: GarbageCollected>(&self, member: &Member<T>) {
    unsafe { cppgc__visitor__trace(self, member.handle) }
  }
}

#[repr(C)]
pub enum EmbedderStackState {
  /// Stack may contain interesting heap pointers.
  MayContainHeapPointers,
  /// Stack does not contain any interesting heap pointers.
  NoHeapPointers,
}

type TraceFn = extern "C" fn(*mut Visitor, *mut ());
type DestroyFn = extern "C" fn(*mut ());

/// A heap for allocating managed C++ objects.
///
/// Similar to v8::Isolate, the heap may only be accessed from one thread at a
/// time.
#[repr(C)]
#[derive(Debug)]
pub struct Heap(Opaque);

impl Drop for Heap {
  fn drop(&mut self) {
    unsafe { cppgc__heap__DELETE(self as *mut Heap) }
  }
}

impl Heap {
  pub fn create(platform: SharedRef<Platform>) -> UniqueRef<Heap> {
    unsafe {
      UniqueRef::from_raw(cppgc__heap__create(
        &*platform as *const Platform as *mut _,
      ))
    }
  }

  pub fn collect_garbage_for_testing(&self, stack_state: EmbedderStackState) {
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

/// Members are used to contain strong pointers to other garbage
/// collected objects. All members fields on garbage collected objects
/// must be trace in the `trace` method.
pub struct Member<T: GarbageCollected> {
  handle: *mut (),
  ptr: *mut T,
}

impl<T: GarbageCollected> std::ops::Deref for Member<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    unsafe { &*self.ptr }
  }
}

/// Constructs an instance of T, which is a garbage collected type.
pub fn make_garbage_collected<T: GarbageCollected>(
  heap: &Heap,
  obj: Box<T>,
) -> Member<T> {
  extern "C" fn destroy<T>(obj: *mut ()) {
    let _ = unsafe { Box::from_raw(obj as *mut T) };
  }

  extern "C" fn trace<T: GarbageCollected>(
    visitor: *mut Visitor,
    obj: *mut (),
  ) {
    let obj = unsafe { &*(obj as *const T) };
    obj.trace(unsafe { &*visitor });
  }

  let ptr = Box::into_raw(obj);
  let handle = unsafe {
    cppgc__make_garbage_collectable(
      heap as *const Heap as *mut _,
      ptr as _,
      trace::<T>,
      destroy::<T>,
    )
  };

  Member { handle, ptr }
}
