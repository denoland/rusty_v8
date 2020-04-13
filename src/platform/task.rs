use std::mem::drop;
use std::mem::forget;

use crate::support::CxxVTable;
use crate::support::Delete;
use crate::support::FieldOffset;
use crate::support::Opaque;
use crate::support::RustVTable;
use crate::support::UniquePtr;

// class Task {
//  public:
//   virtual ~Task() = default;
//   virtual void Run() = 0;
// };

extern "C" {
  fn v8__Task__BASE__CONSTRUCT(buf: *mut std::mem::MaybeUninit<Task>) -> ();
  fn v8__Task__DELETE(this: *mut Task) -> ();
  fn v8__Task__Run(this: *mut Task) -> ();
}

#[no_mangle]
pub unsafe extern "C" fn v8__Task__BASE__DELETE(this: &mut Task) {
  drop(TaskBase::dispatch_box(this))
}

#[no_mangle]
pub unsafe extern "C" fn v8__Task__BASE__Run(this: &mut Task) {
  TaskBase::dispatch_mut(this).run()
}

#[repr(C)]
pub struct Task {
  _cxx_vtable: CxxVTable,
}

impl Task {
  pub fn run(&mut self) {
    unsafe { v8__Task__Run(self) }
  }
}

impl Delete for Task {
  fn delete(&'static mut self) {
    unsafe { v8__Task__DELETE(self) }
  }
}

pub trait AsTask {
  fn as_task(&self) -> &Task;
  fn as_task_mut(&mut self) -> &mut Task;

  // TODO: this should be a trait in itself.
  fn into_unique_ptr(mut self: Box<Self>) -> UniquePtr<Task>
  where
    Self: 'static,
  {
    let task = self.as_task_mut() as *mut Task;
    forget(self);
    unsafe { UniquePtr::from_raw(task) }
  }
}

impl AsTask for Task {
  fn as_task(&self) -> &Task {
    self
  }
  fn as_task_mut(&mut self) -> &mut Task {
    self
  }
}

impl<T> AsTask for T
where
  T: TaskImpl,
{
  fn as_task(&self) -> &Task {
    &self.base().cxx_base
  }
  fn as_task_mut(&mut self) -> &mut Task {
    &mut self.base_mut().cxx_base
  }
}

pub trait TaskImpl: AsTask {
  fn base(&self) -> &TaskBase;
  fn base_mut(&mut self) -> &mut TaskBase;
  fn run(&mut self) -> ();
}

pub struct TaskBase {
  cxx_base: Task,
  offset_within_embedder: FieldOffset<Self>,
  rust_vtable: RustVTable<&'static dyn TaskImpl>,
}

impl TaskBase {
  fn construct_cxx_base() -> Task {
    unsafe {
      let mut buf = std::mem::MaybeUninit::<Task>::uninit();
      v8__Task__BASE__CONSTRUCT(&mut buf);
      buf.assume_init()
    }
  }

  fn get_cxx_base_offset() -> FieldOffset<Task> {
    let buf = std::mem::MaybeUninit::<Self>::uninit();
    FieldOffset::from_ptrs(buf.as_ptr(), unsafe { &(*buf.as_ptr()).cxx_base })
  }

  fn get_offset_within_embedder<T>() -> FieldOffset<Self>
  where
    T: TaskImpl,
  {
    let buf = std::mem::MaybeUninit::<T>::uninit();
    let embedder_ptr: *const T = buf.as_ptr();
    let self_ptr: *const Self = unsafe { (*embedder_ptr).base() };
    FieldOffset::from_ptrs(embedder_ptr, self_ptr)
  }

  fn get_rust_vtable<T>() -> RustVTable<&'static dyn TaskImpl>
  where
    T: TaskImpl,
  {
    let buf = std::mem::MaybeUninit::<T>::uninit();
    let embedder_ptr = buf.as_ptr();
    let trait_object: *const dyn TaskImpl = embedder_ptr;
    let (data_ptr, vtable): (*const T, RustVTable<_>) =
      unsafe { std::mem::transmute(trait_object) };
    assert_eq!(data_ptr, embedder_ptr);
    vtable
  }

  pub fn new<T>() -> Self
  where
    T: TaskImpl,
  {
    Self {
      cxx_base: Self::construct_cxx_base(),
      offset_within_embedder: Self::get_offset_within_embedder::<T>(),
      rust_vtable: Self::get_rust_vtable::<T>(),
    }
  }

  pub unsafe fn dispatch(task: &Task) -> &dyn TaskImpl {
    let this = Self::get_cxx_base_offset().to_embedder::<Self>(task);
    let embedder = this.offset_within_embedder.to_embedder::<Opaque>(this);
    std::mem::transmute((embedder, this.rust_vtable))
  }

  pub unsafe fn dispatch_mut(task: &mut Task) -> &mut dyn TaskImpl {
    let this = Self::get_cxx_base_offset().to_embedder_mut::<Self>(task);
    let vtable = this.rust_vtable;
    let embedder = this.offset_within_embedder.to_embedder_mut::<Opaque>(this);
    std::mem::transmute((embedder, vtable))
  }

  pub unsafe fn dispatch_box(task: &mut Task) -> Box<dyn TaskImpl> {
    std::mem::transmute(Self::dispatch_mut(task))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::atomic::AtomicUsize;
  use std::sync::atomic::Ordering::SeqCst;

  static RUN_COUNT: AtomicUsize = AtomicUsize::new(0);
  static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

  // Using repr(C) to preserve field ordering and test that everything works
  // when the TaskBase field is not the first element of the struct.
  #[repr(C)]
  struct TestTask {
    field1: i32,
    base: TaskBase,
    field2: f64,
  }

  impl TestTask {
    pub fn new() -> Self {
      Self {
        base: TaskBase::new::<Self>(),
        field1: -42,
        field2: 4.2,
      }
    }
  }

  impl TaskImpl for TestTask {
    fn base(&self) -> &TaskBase {
      &self.base
    }
    fn base_mut(&mut self) -> &mut TaskBase {
      &mut self.base
    }
    fn run(&mut self) {
      RUN_COUNT.fetch_add(1, SeqCst);
    }
  }

  impl Drop for TestTask {
    fn drop(&mut self) {
      DROP_COUNT.fetch_add(1, SeqCst);
    }
  }

  #[test]
  fn test_task() {
    {
      TestTask::new().run();
    }
    assert_eq!(RUN_COUNT.swap(0, SeqCst), 1);
    assert_eq!(DROP_COUNT.swap(0, SeqCst), 1);

    {
      Box::new(TestTask::new()).into_unique_ptr();
    }
    assert_eq!(RUN_COUNT.swap(0, SeqCst), 0);
    assert_eq!(DROP_COUNT.swap(0, SeqCst), 1);
  }
}
