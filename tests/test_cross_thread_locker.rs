use std::{
  sync::{Arc, Once},
  thread::{self, JoinHandle},
};

static INIT: Once = Once::new();

fn initialize_test() {
  INIT.call_once(|| {
    v8::V8::initialize_platform(
      v8::new_default_platform(0, false).make_shared(),
    );
    v8::V8::initialize();
  });
}

fn spawn_thread_locked<F, R>(
  isolate: &Arc<v8::SharedIsolate>,
  f: F,
) -> JoinHandle<R>
where
  F: FnOnce(&mut v8::Locker) -> R + Send + Sync + 'static,
  R: Send + 'static,
{
  let isolate = isolate.clone();
  thread::spawn(move || {
    let mut locker = isolate.lock();
    f(&mut locker)
  })
}

fn spawn_thread_with_scope<F, R>(
  isolate: &Arc<v8::SharedIsolate>,
  f: F,
) -> JoinHandle<R>
where
  F: FnOnce(&mut v8::HandleScope<v8::Context>) -> R + Send + Sync + 'static,
  R: Send + 'static,
{
  spawn_thread_locked(isolate, |locker| {
    let scope = &mut v8::HandleScope::new(locker.isolate_mut());
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    f(scope)
  })
}

#[test]
fn isolate_passed_between_threads_with_locker() {
  initialize_test();
  let isolate = Arc::new(v8::Isolate::new(Default::default()).to_shared());

  let global = spawn_thread_with_scope(&isolate, move |scope| {
    let name = v8::String::new(scope, "Thread 1 value").unwrap();
    v8::Global::new(scope, name)
  })
  .join()
  .unwrap();

  let found = spawn_thread_with_scope(&isolate, move |scope| {
    let name = v8::Local::new(scope, global);
    name.to_rust_string_lossy(scope)
  })
  .join()
  .unwrap();

  assert_eq!(found, "Thread 1 value");
}

fn single_isolate_cross_thread_operation_spam(isolate: Arc<v8::SharedIsolate>) {
  let global_handles = (0..100)
    .map(|i| {
      let val = i;
      spawn_thread_with_scope(&isolate, move |scope| {
        let name = v8::Number::new(scope, val as f64);
        v8::Global::new(scope, name)
      })
    })
    .collect::<Vec<_>>();

  let globals = global_handles
    .into_iter()
    .map(|h| h.join().unwrap())
    .collect::<Vec<_>>();

  let number_handles = globals
    .into_iter()
    .map(|global| {
      spawn_thread_with_scope(&isolate, move |scope| {
        let local = v8::Local::new(scope, global);
        local.number_value(scope).unwrap()
      })
    })
    .collect::<Vec<_>>();

  let numbers = number_handles
    .into_iter()
    .map(|h| h.join().unwrap())
    .collect::<Vec<_>>();

  for val in 0..100 {
    assert_eq!(val as f64, numbers[val]);
  }
}

#[test]
fn mass_spam_isolate() {
  initialize_test();

  // This is done multiple times to verify that disposal of an isolate doesn't raise errors.
  let t1 = thread::spawn(|| {
    single_isolate_cross_thread_operation_spam(Arc::new(
      v8::Isolate::new(Default::default()).to_shared(),
    ));
  });
  let t2 = thread::spawn(|| {
    single_isolate_cross_thread_operation_spam(Arc::new(
      v8::Isolate::new(Default::default()).to_shared(),
    ));
  });
  t1.join().unwrap();
  t2.join().unwrap();
}
