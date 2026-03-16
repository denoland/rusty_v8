use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

struct TestPlatformImpl {
  post_task_count: Arc<AtomicUsize>,
  post_delayed_task_count: Arc<AtomicUsize>,
  /// Tasks queued for later execution (simulating an event loop).
  task_queue: Arc<Mutex<VecDeque<v8::Task>>>,
}

impl v8::PlatformImpl for TestPlatformImpl {
  fn post_task(&self, _isolate_ptr: *mut std::ffi::c_void, task: v8::Task) {
    self.post_task_count.fetch_add(1, Ordering::SeqCst);
    self.task_queue.lock().unwrap().push_back(task);
  }

  fn post_non_nestable_task(
    &self,
    _isolate_ptr: *mut std::ffi::c_void,
    task: v8::Task,
  ) {
    self.post_task_count.fetch_add(1, Ordering::SeqCst);
    self.task_queue.lock().unwrap().push_back(task);
  }

  fn post_delayed_task(
    &self,
    _isolate_ptr: *mut std::ffi::c_void,
    task: v8::Task,
    _delay_in_seconds: f64,
  ) {
    self.post_delayed_task_count.fetch_add(1, Ordering::SeqCst);
    // For testing, run delayed tasks immediately too.
    self.task_queue.lock().unwrap().push_back(task);
  }

  fn post_non_nestable_delayed_task(
    &self,
    _isolate_ptr: *mut std::ffi::c_void,
    task: v8::Task,
    _delay_in_seconds: f64,
  ) {
    self.post_delayed_task_count.fetch_add(1, Ordering::SeqCst);
    self.task_queue.lock().unwrap().push_back(task);
  }

  fn post_idle_task(
    &self,
    _isolate_ptr: *mut std::ffi::c_void,
    task: v8::IdleTask,
  ) {
    self.post_task_count.fetch_add(1, Ordering::SeqCst);
    task.run(0.0);
  }
}

#[test]
fn custom_platform_foreground_task_ownership() {
  let post_task_count = Arc::new(AtomicUsize::new(0));
  let post_delayed_task_count = Arc::new(AtomicUsize::new(0));
  let task_queue: Arc<Mutex<VecDeque<v8::Task>>> =
    Arc::new(Mutex::new(VecDeque::new()));

  let platform_impl = TestPlatformImpl {
    post_task_count: post_task_count.clone(),
    post_delayed_task_count: post_delayed_task_count.clone(),
    task_queue: task_queue.clone(),
  };

  v8::V8::set_flags_from_string("--allow-natives-syntax");
  v8::V8::initialize_platform(
    v8::new_custom_platform(0, false, true, platform_impl).make_shared(),
  );
  v8::V8::initialize();

  {
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    // Basic JS execution should work with the custom platform.
    let source = v8::String::new(scope, "1 + 2").unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    let result = script.run(scope).unwrap();
    let result = result.uint32_value(scope).unwrap();
    assert_eq!(result, 3);

    // Reset counters before the Atomics test.
    post_task_count.store(0, Ordering::SeqCst);

    // Atomics.waitAsync posts a foreground task when notified.
    // This verifies the custom platform receives the task and can run it.
    let source = r#"
      const sab = new SharedArrayBuffer(16);
      const i32a = new Int32Array(sab);
      const result = Atomics.waitAsync(i32a, 0, 0);
      Atomics.notify(i32a, 0, 1);
    "#;
    let source = v8::String::new(scope, source).unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    script.run(scope).unwrap();

    // Drain the task queue — this is the embedder's event loop.
    // Tasks are run by calling task.run() directly, NOT via PumpMessageLoop.
    while let Some(task) = task_queue.lock().unwrap().pop_front() {
      task.run();
    }
  }

  // The custom platform should have received at least one foreground task
  // from the Atomics.waitAsync/notify sequence.
  let tasks = post_task_count.load(Ordering::SeqCst);
  assert!(
    tasks > 0,
    "expected at least one post_task callback, got {tasks}"
  );

  unsafe { v8::V8::dispose() };
  v8::V8::dispose_platform();
}
