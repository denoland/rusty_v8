use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;

struct TestPlatformImpl {
  post_task_count: Arc<AtomicUsize>,
  post_delayed_task_count: Arc<AtomicUsize>,
}

impl v8::PlatformImpl for TestPlatformImpl {
  fn post_task(&self, _isolate_ptr: *mut std::ffi::c_void, _task: v8::Task) {
    self.post_task_count.fetch_add(1, Ordering::SeqCst);
    // Task is dropped without running — this tests that the platform
    // receives ownership and can safely drop tasks (e.g. for tasks that
    // arrive after isolate shutdown). In a real embedder, tasks would be
    // scheduled on the isolate's event loop via tokio::spawn etc.
  }

  fn post_non_nestable_task(
    &self,
    _isolate_ptr: *mut std::ffi::c_void,
    _task: v8::Task,
  ) {
    self.post_task_count.fetch_add(1, Ordering::SeqCst);
  }

  fn post_delayed_task(
    &self,
    _isolate_ptr: *mut std::ffi::c_void,
    _task: v8::Task,
    _delay_in_seconds: f64,
  ) {
    self.post_delayed_task_count.fetch_add(1, Ordering::SeqCst);
  }

  fn post_non_nestable_delayed_task(
    &self,
    _isolate_ptr: *mut std::ffi::c_void,
    _task: v8::Task,
    _delay_in_seconds: f64,
  ) {
    self.post_delayed_task_count.fetch_add(1, Ordering::SeqCst);
  }

  fn post_idle_task(
    &self,
    _isolate_ptr: *mut std::ffi::c_void,
    _task: v8::IdleTask,
  ) {
    self.post_task_count.fetch_add(1, Ordering::SeqCst);
  }
}

#[test]
fn custom_platform_foreground_task_ownership() {
  let post_task_count = Arc::new(AtomicUsize::new(0));
  let post_delayed_task_count = Arc::new(AtomicUsize::new(0));

  let platform_impl = TestPlatformImpl {
    post_task_count: post_task_count.clone(),
    post_delayed_task_count: post_delayed_task_count.clone(),
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
    // This verifies the custom platform receives task ownership.
    let source = r#"
      const sab = new SharedArrayBuffer(16);
      const i32a = new Int32Array(sab);
      const result = Atomics.waitAsync(i32a, 0, 0);
      Atomics.notify(i32a, 0, 1);
    "#;
    let source = v8::String::new(scope, source).unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    script.run(scope).unwrap();

    // Give V8 background threads time to post tasks.
    std::thread::sleep(std::time::Duration::from_millis(100));
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
