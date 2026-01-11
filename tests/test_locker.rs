use std::pin::pin;

#[test]
fn locker_basic() {
  let _setup_guard = setup();
  let mut isolate = v8::Isolate::new_unentered(Default::default());
  {
    let mut locker = v8::Locker::new(&mut isolate);
    let scope = pin!(v8::HandleScope::new(&mut *locker));
    let scope = &mut scope.init();
    let _context = v8::Context::new(scope, Default::default());
  }
}

#[test]
fn locker_with_script() {
  let _setup_guard = setup();
  let mut isolate = v8::Isolate::new_unentered(Default::default());
  {
    let mut locker = v8::Locker::new(&mut isolate);
    let scope = pin!(v8::HandleScope::new(&mut *locker));
    let scope = &mut scope.init();
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    let code = v8::String::new(scope, "40 + 2").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    assert_eq!(result.to_integer(scope).unwrap().value(), 42);
  }
}

#[test]
fn unentered_isolate_no_lifo_constraint() {
  let _setup_guard = setup();
  let isolate1 = v8::Isolate::new_unentered(Default::default());
  let isolate2 = v8::Isolate::new_unentered(Default::default());
  let isolate3 = v8::Isolate::new_unentered(Default::default());
  drop(isolate2);
  drop(isolate1);
  drop(isolate3);
}

#[test]
fn locker_multiple_lock_unlock() {
  let _setup_guard = setup();
  let mut isolate = v8::Isolate::new_unentered(Default::default());

  {
    let mut locker = v8::Locker::new(&mut isolate);
    let scope = pin!(v8::HandleScope::new(&mut *locker));
    let scope = &mut scope.init();
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    let code = v8::String::new(scope, "1 + 1").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    assert_eq!(result.to_integer(scope).unwrap().value(), 2);
  }

  {
    let mut locker = v8::Locker::new(&mut isolate);
    let scope = pin!(v8::HandleScope::new(&mut *locker));
    let scope = &mut scope.init();
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    let code = v8::String::new(scope, "2 + 2").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    assert_eq!(result.to_integer(scope).unwrap().value(), 4);
  }
}

#[test]
fn locker_is_locked() {
  let _setup_guard = setup();
  let mut isolate = v8::Isolate::new_unentered(Default::default());

  assert!(!v8::Locker::is_locked(&isolate));
  {
    let _locker = v8::Locker::new(&mut isolate);
  }
  assert!(!v8::Locker::is_locked(&isolate));
}

fn setup() -> impl Drop {
  use std::sync::Once;
  static INIT: Once = Once::new();
  INIT.call_once(|| {
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
  });
  struct Guard;
  impl Drop for Guard {
    fn drop(&mut self) {}
  }
  Guard
}
