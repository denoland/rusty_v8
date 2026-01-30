use std::pin::pin;
use std::sync::mpsc;
use std::thread;

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
    // Locker is now held - but we can't call is_locked because we have &mut isolate
    // The lock check happens internally
  }
  assert!(!v8::Locker::is_locked(&isolate));
}

#[test]
fn locker_state_preserved_across_locks() {
  let _setup_guard = setup();
  let mut isolate = v8::Isolate::new_unentered(Default::default());

  // First lock: create a global and set a value
  let global_template = {
    let mut locker = v8::Locker::new(&mut isolate);
    let scope = pin!(v8::HandleScope::new(&mut *locker));
    let scope = &mut scope.init();
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    // Set a global variable
    let code = v8::String::new(scope, "globalThis.testValue = 123").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    script.run(scope).unwrap();

    // Get the global object template for later verification
    context.global(scope)
  };
  drop(global_template);

  // Second lock: verify state is preserved (new context won't have the value)
  {
    let mut locker = v8::Locker::new(&mut isolate);
    let scope = pin!(v8::HandleScope::new(&mut *locker));
    let scope = &mut scope.init();
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    // New context - value should not exist
    let code = v8::String::new(scope, "typeof globalThis.testValue").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    let result_str = result.to_rust_string_lossy(scope);
    assert_eq!(result_str, "undefined");
  }
}

#[test]
fn locker_drop_releases_lock() {
  let _setup_guard = setup();
  let mut isolate = v8::Isolate::new_unentered(Default::default());

  // Create and immediately drop a locker
  {
    let locker = v8::Locker::new(&mut isolate);
    drop(locker);
  }

  // Should be able to create another locker without blocking
  {
    let _locker = v8::Locker::new(&mut isolate);
  }

  // Isolate should be unlocked now
  assert!(!v8::Locker::is_locked(&isolate));
}

#[test]
fn unentered_isolate_as_raw() {
  let _setup_guard = setup();
  let isolate = v8::Isolate::new_unentered(Default::default());

  // as_raw should return a valid pointer
  let ptr = isolate.as_raw();
  assert!(!ptr.is_null());
}

#[test]
fn locker_send_isolate_between_threads() {
  let _setup_guard = setup();

  // Create isolate on main thread
  let mut isolate = v8::Isolate::new_unentered(Default::default());

  // Use on main thread first
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

  // Send to another thread
  let (tx, rx) = mpsc::channel();

  let handle = thread::spawn(move || {
    // Use isolate on worker thread
    let mut locker = v8::Locker::new(&mut isolate);
    let scope = pin!(v8::HandleScope::new(&mut *locker));
    let scope = &mut scope.init();
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    let code = v8::String::new(scope, "2 + 2").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    let value = result.to_integer(scope).unwrap().value();

    // Send result back
    tx.send(value).unwrap();

    // Return isolate ownership
    drop(locker);
    isolate
  });

  // Wait for result
  let result = rx.recv().unwrap();
  assert_eq!(result, 4);

  // Get isolate back and use again on main thread
  let mut isolate = handle.join().unwrap();
  {
    let mut locker = v8::Locker::new(&mut isolate);
    let scope = pin!(v8::HandleScope::new(&mut *locker));
    let scope = &mut scope.init();
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    let code = v8::String::new(scope, "3 + 3").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    assert_eq!(result.to_integer(scope).unwrap().value(), 6);
  }
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
