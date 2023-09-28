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
  fn cppgc__heap__force_garbage_collection_slow(
    heap: *mut Heap,
    stack_state: EmbedderStackState,
  );
}

pub fn initalize_process(platform: SharedRef<Platform>) {
  unsafe {
    cppgc__initialize_process(&*platform as *const Platform as *mut _);
  }
}

pub unsafe fn shutdown_process() {
  cppgc__shutdown_process();
}

#[repr(C)]
#[derive(Debug)]
pub struct RustObj(Opaque);

#[repr(C)]
#[derive(Debug)]
pub struct Visitor(Opaque);

#[repr(C)]
pub enum EmbedderStackState {
  /// Stack may contain interesting heap pointers.
  MayContainHeapPointers,
  /// Stack does not contain any interesting heap pointers.
  NoHeapPointers,
}

type TraceFn = extern "C" fn(*mut Visitor, *mut ());
type DestroyFn = extern "C" fn(*mut ());

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

  pub fn force_garbage_collection_slow(&self, stack_state: EmbedderStackState) {
    unsafe {
      cppgc__heap__force_garbage_collection_slow(
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

pub trait GarbageCollected {
  fn trace(&self, visitor: *mut Visitor) {}
}

pub fn make_garbage_collected<T: GarbageCollected>(
  heap: &Heap,
  obj: Box<T>,
) -> *mut T {
  extern "C" fn destroy<T>(obj: *mut ()) {
    let _ = unsafe { Box::from_raw(obj as *mut T) };
  }

  extern "C" fn trace<T: GarbageCollected>(
    visitor: *mut Visitor,
    obj: *mut (),
  ) {
    let obj = unsafe { &*(obj as *const T) };
    obj.trace(visitor);
  }

  unsafe {
    cppgc__make_garbage_collectable(
      heap as *const Heap as *mut _,
      Box::into_raw(obj) as _,
      trace::<T>,
      destroy::<T>,
    ) as *mut T
  }
}
