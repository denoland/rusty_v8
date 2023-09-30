// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license

use crate::platform::Platform;
use crate::support::int;
use crate::support::Opaque;
use crate::support::SharedRef;
use crate::support::UniqueRef;

extern "C" {
  fn cppgc__initialize_process(platform: *mut Platform);
  fn cppgc__shutdown_process();

  fn cppgc__heap__create(
    platform: *mut Platform,
    wrappable_type_index: int,
    wrappable_instance_index: int,
    embedder_id_for_garbage_collected: u16,
  ) -> *mut Heap;
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

/// Describes how V8 wrapper objects maintain references to garbage-collected C++ objects.
pub struct WrapperDescriptor {
  /// Index of the wrappable type.
  pub wrappable_type_index: InternalFieldIndex,
  /// Index of the wrappable instance.
  pub wrappable_instance_index: InternalFieldIndex,
  /// Embedder id identifying instances of garbage-collected objects. It is expected that
  /// the first field of the wrappable type is a uint16_t holding the id. Only references
  /// to instances of wrappables types with an id of embedder_id_for_garbage_collected will
  /// be considered by Heap.
  pub embedder_id_for_garbage_collected: u16,
}

impl WrapperDescriptor {
  pub fn new(
    wrappable_type_index: InternalFieldIndex,
    wrappable_instance_index: InternalFieldIndex,
    embedder_id_for_garbage_collected: u16,
  ) -> Self {
    Self {
      wrappable_type_index,
      wrappable_instance_index,
      embedder_id_for_garbage_collected,
    }
  }
}

pub struct HeapCreateParams {
  wrapper_descriptor: WrapperDescriptor,
  /// Specifies which kind of marking are supported by the heap.
  pub marking_support: MarkingType,
  /// Specifies which kind of sweeping are supported by the heap.
  pub sweeping_support: SweepingType,
}

impl HeapCreateParams {
  pub fn new(wrapper_descriptor: WrapperDescriptor) -> Self {
    Self {
      wrapper_descriptor,
      marking_support: MarkingType::IncrementalAndConcurrent,
      sweeping_support: SweepingType::IncrementalAndConcurrent,
    }
  }
}

type TraceFn = unsafe extern "C" fn(*mut (), *mut Visitor);
type DestroyFn = unsafe extern "C" fn(*mut ());

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
  pub fn create(
    platform: SharedRef<Platform>,
    params: HeapCreateParams,
  ) -> UniqueRef<Heap> {
    let WrapperDescriptor {
      wrappable_type_index,
      wrappable_instance_index,
      embedder_id_for_garbage_collected,
    } = params.wrapper_descriptor;

    unsafe {
      UniqueRef::from_raw(cppgc__heap__create(
        &*platform as *const Platform as *mut _,
        wrappable_type_index,
        wrappable_instance_index,
        embedder_id_for_garbage_collected,
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
  pub handle: *mut (),
  ptr: *mut T,
}

impl<T: GarbageCollected> Member<T> {
  /// Returns a raw pointer to the object.
  ///
  /// # Safety
  ///
  /// There are no guarantees that the object is alive and not garbage collected.
  pub unsafe fn get(&self) -> &T {
    unsafe { &*self.ptr }
  }
}

impl<T: GarbageCollected> std::ops::Deref for Member<T> {
  type Target = T;

  fn deref(&self) -> &Self::Target {
    unsafe { &*self.ptr }
  }
}

/// Constructs an instance of T, which is a garbage collected type.
///
/// The object will be allocated on the heap and managed by cppgc. During
/// marking, the object will be traced by calling the `trace` method on it.
///
/// During sweeping, the destructor will be called and the memory will be
/// freed using `Box::from_raw`.
pub fn make_garbage_collected<T: GarbageCollected>(
  heap: &Heap,
  obj: Box<T>,
) -> Member<T> {
  unsafe extern "C" fn destroy<T>(obj: *mut ()) {
    let _ = Box::from_raw(obj as *mut T);
  }

  unsafe { make_garbage_collected_raw(heap, Box::into_raw(obj), destroy::<T>) }
}

/// # Safety
///
/// By calling this function, you are giving up ownership of `T` to the
/// garbage collector.
///
/// `obj` must be a pointer to a valid instance of T allocated on the heap.
///
/// `drop_fn` must be a function that drops the instance of T. This function
/// will be called when the object is garbage collected.
pub unsafe fn make_garbage_collected_raw<T: GarbageCollected>(
  heap: &Heap,
  obj: *mut T,
  destroy: DestroyFn,
) -> Member<T> {
  unsafe extern "C" fn trace<T: GarbageCollected>(
    obj: *mut (),
    visitor: *mut Visitor,
  ) {
    let obj = unsafe { &*(obj as *const T) };
    obj.trace(unsafe { &*visitor });
  }

  let handle = cppgc__make_garbage_collectable(
    heap as *const Heap as *mut _,
    obj as _,
    trace::<T>,
    destroy,
  );

  Member { handle, ptr: obj }
}
