use std::pin::pin;

#[test]
fn isolate_scope_basic() {
  let _setup_guard = setup();
  let mut isolate = v8::Isolate::new_unentered(Default::default());

  let mut locker = v8::Locker::new(&mut isolate);
  {
    let _scope = locker.enter();
    let scope = pin!(v8::HandleScope::new(&mut *locker));
    let scope = &mut scope.init();
    let _context = v8::Context::new(scope, Default::default());
  }
}

#[test]
fn isolate_scope_with_script() {
  let _setup_guard = setup();
  let mut isolate = v8::Isolate::new_unentered(Default::default());

  let mut locker = v8::Locker::new(&mut isolate);
  {
    let _scope = locker.enter();
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

/// Multiple IsolateScope enter/exit cycles on the same isolate.
/// Simulates the pattern: V8 work → drop scope → yield → V8 work → drop scope.
#[test]
fn isolate_scope_multiple_cycles() {
  let _setup_guard = setup();
  let mut isolate = v8::Isolate::new_unentered(Default::default());

  let mut locker = v8::Locker::new(&mut isolate);

  // First V8 work block
  {
    let _scope = locker.enter();
    let scope = pin!(v8::HandleScope::new(&mut *locker));
    let scope = &mut scope.init();
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    let code = v8::String::new(scope, "1 + 1").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    assert_eq!(result.to_integer(scope).unwrap().value(), 2);
  }
  // IsolateScope dropped — simulates a yield point

  // Second V8 work block (re-enter)
  {
    let _scope = locker.enter();
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

/// Two isolates with Lockers on the same thread.
/// IsolateScope ensures GetCurrent() returns the correct isolate for each
/// V8 work block — this is the core scenario the type was designed for.
#[test]
fn isolate_scope_two_isolates_interleaved() {
  let _setup_guard = setup();
  let mut isolate_a = v8::Isolate::new_unentered(Default::default());
  let mut isolate_b = v8::Isolate::new_unentered(Default::default());

  // Both Lockers alive simultaneously (same thread)
  let mut locker_a = v8::Locker::new(&mut isolate_a);
  let mut locker_b = v8::Locker::new(&mut isolate_b);

  // Work on A
  {
    let _scope = locker_a.enter();
    let scope = pin!(v8::HandleScope::new(&mut *locker_a));
    let scope = &mut scope.init();
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    let code = v8::String::new(scope, "'hello'").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    let result_str = result.to_rust_string_lossy(scope);
    assert_eq!(result_str, "hello");
  }

  // Switch to B (A's IsolateScope dropped)
  {
    let _scope = locker_b.enter();
    let scope = pin!(v8::HandleScope::new(&mut *locker_b));
    let scope = &mut scope.init();
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    let code = v8::String::new(scope, "'world'").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    let result_str = result.to_rust_string_lossy(scope);
    assert_eq!(result_str, "world");
  }

  // Back to A
  {
    let _scope = locker_a.enter();
    let scope = pin!(v8::HandleScope::new(&mut *locker_a));
    let scope = &mut scope.init();
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    let code = v8::String::new(scope, "42").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    assert_eq!(result.to_integer(scope).unwrap().value(), 42);
  }

  // Drop order doesn't matter for different isolates
  drop(locker_b);
  drop(locker_a);
}

/// Three isolates interleaving — stress test for the per-isolate entry stack.
#[test]
fn isolate_scope_three_isolates_round_robin() {
  let _setup_guard = setup();
  let mut isolate_a = v8::Isolate::new_unentered(Default::default());
  let mut isolate_b = v8::Isolate::new_unentered(Default::default());
  let mut isolate_c = v8::Isolate::new_unentered(Default::default());

  let mut locker_a = v8::Locker::new(&mut isolate_a);
  let mut locker_b = v8::Locker::new(&mut isolate_b);
  let mut locker_c = v8::Locker::new(&mut isolate_c);

  // Round-robin: A → B → C → A → B → C
  for i in 0..2 {
    {
      let _scope = locker_a.enter();
      let scope = pin!(v8::HandleScope::new(&mut *locker_a));
      let scope = &mut scope.init();
      let context = v8::Context::new(scope, Default::default());
      let scope = &mut v8::ContextScope::new(scope, context);

      let expr = format!("{} + 1", i * 3);
      let code = v8::String::new(scope, &expr).unwrap();
      let script = v8::Script::compile(scope, code, None).unwrap();
      let result = script.run(scope).unwrap();
      assert_eq!(result.to_integer(scope).unwrap().value(), i * 3 + 1);
    }

    {
      let _scope = locker_b.enter();
      let scope = pin!(v8::HandleScope::new(&mut *locker_b));
      let scope = &mut scope.init();
      let context = v8::Context::new(scope, Default::default());
      let scope = &mut v8::ContextScope::new(scope, context);

      let expr = format!("{} + 2", i * 3);
      let code = v8::String::new(scope, &expr).unwrap();
      let script = v8::Script::compile(scope, code, None).unwrap();
      let result = script.run(scope).unwrap();
      assert_eq!(result.to_integer(scope).unwrap().value(), i * 3 + 2);
    }

    {
      let _scope = locker_c.enter();
      let scope = pin!(v8::HandleScope::new(&mut *locker_c));
      let scope = &mut scope.init();
      let context = v8::Context::new(scope, Default::default());
      let scope = &mut v8::ContextScope::new(scope, context);

      let expr = format!("{} + 3", i * 3);
      let code = v8::String::new(scope, &expr).unwrap();
      let script = v8::Script::compile(scope, code, None).unwrap();
      let result = script.run(scope).unwrap();
      assert_eq!(result.to_integer(scope).unwrap().value(), i * 3 + 3);
    }
  }

  drop(locker_a);
  drop(locker_c);
  drop(locker_b);
}

/// Nested IsolateScope on the same isolate (re-entrant enter).
#[test]
fn isolate_scope_nested_same_isolate() {
  let _setup_guard = setup();
  let mut isolate = v8::Isolate::new_unentered(Default::default());

  let mut locker = v8::Locker::new(&mut isolate);

  let _outer = locker.enter();
  {
    let _inner = locker.enter();
    let scope = pin!(v8::HandleScope::new(&mut *locker));
    let scope = &mut scope.init();
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    let code = v8::String::new(scope, "99").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    assert_eq!(result.to_integer(scope).unwrap().value(), 99);
  }
  // Inner dropped, outer still active — should still work
  {
    let scope = pin!(v8::HandleScope::new(&mut *locker));
    let scope = &mut scope.init();
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    let code = v8::String::new(scope, "100").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    assert_eq!(result.to_integer(scope).unwrap().value(), 100);
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
