//! Tests for Locker safety with non-LIFO drop ordering and IsolateScope.
//!
//! These tests validate the fix where `Locker` is a pure mutex (no
//! `Isolate::Enter()`/`Exit()`) and `IsolateScope` manages the per-thread
//! `GetCurrent()` thread-local at fine-grained boundaries.
//!
//! The original bug: when two Lockers coexist on the same thread (cooperative
//! scheduling), non-LIFO drop ordering causes V8's `entry_stack_` to restore
//! a stale `previous_isolate`, leading to a NULL dereference on the next
//! `Isolate::Enter()`.

use std::pin::pin;

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

/// Helper: execute a JS expression under an existing Locker and return the
/// integer result. Creates its own IsolateScope, HandleScope, Context, and
/// ContextScope.
fn eval_js(locker: &mut v8::Locker, expr: &str) -> i64 {
  let _isolate_scope = locker.enter();
  let scope = pin!(v8::HandleScope::new(&mut **locker));
  let scope = &mut scope.init();
  let context = v8::Context::new(scope, Default::default());
  let scope = &mut v8::ContextScope::new(scope, context);

  let code = v8::String::new(scope, expr).unwrap();
  let script = v8::Script::compile(scope, code, None).unwrap();
  let result = script.run(scope).unwrap();
  result.to_integer(scope).unwrap().value()
}

// ---------------------------------------------------------------------------
// Test 1: Non-LIFO Locker Drop (the original segfault)
// ---------------------------------------------------------------------------

/// Create two isolates A and B. `Locker::new(A)`, then `Locker::new(B)`.
/// Drop A before B (non-LIFO). Then create a new `Locker::new(A)`.
///
/// Before the fix, this sequence caused a segfault because:
/// 1. Locker(A) → Enter(A) → thread-local = A
/// 2. Locker(B) → Enter(B) → B.entry_stack_ stores prev=A
/// 3. Drop Locker(A) → Exit(A) → thread-local = null, A.entry_stack_ = null
/// 4. Drop Locker(B) → Exit(B) → restores thread-local = A (stale!)
/// 5. Locker(A) → Enter(A) → sees current==A → re-entry → deref null entry_stack_ → SEGFAULT
///
/// After the fix (Locker = pure mutex, no Enter/Exit), this is safe.
#[test]
fn non_lifo_locker_drop() {
  let _setup_guard = setup();
  let mut isolate_a = v8::Isolate::new_unentered(Default::default());
  let mut isolate_b = v8::Isolate::new_unentered(Default::default());

  // Both lockers alive simultaneously
  let locker_a = v8::Locker::new(&mut isolate_a);
  let locker_b = v8::Locker::new(&mut isolate_b);

  // Non-LIFO: drop A first, then B
  drop(locker_a);
  drop(locker_b);

  // Re-acquire A — this was the crash point before the fix
  let locker_a = v8::Locker::new(&mut isolate_a);
  drop(locker_a);
}

// ---------------------------------------------------------------------------
// Test 2: Locker + IsolateScope + Basic JS Execution
// ---------------------------------------------------------------------------

/// Standard usage pattern: Locker → IsolateScope → HandleScope → JS.
#[test]
fn locker_isolate_scope_basic_js() {
  let _setup_guard = setup();
  let mut isolate = v8::Isolate::new_unentered(Default::default());

  let mut locker = v8::Locker::new(&mut isolate);
  let result = eval_js(&mut locker, "1 + 1");
  assert_eq!(result, 2);
}

// ---------------------------------------------------------------------------
// Test 3: Multiple IsolateScope Cycles Under a Single Locker
// ---------------------------------------------------------------------------

/// One Locker, then N times: create IsolateScope → HandleScope → execute JS
/// → drop IsolateScope. Simulates the runtime pattern where
/// `trigger_fetch_event`, `process_single_callback`, and
/// `pump_and_checkpoint` each create/drop an IsolateScope within a single
/// Locker lifetime.
#[test]
fn multiple_isolate_scope_cycles_single_locker() {
  let _setup_guard = setup();
  let mut isolate = v8::Isolate::new_unentered(Default::default());

  let mut locker = v8::Locker::new(&mut isolate);

  for i in 0..10 {
    let expected = i + 1;
    let expr = format!("{} + 1", i);
    let result = eval_js(&mut locker, &expr);
    assert_eq!(result, expected);
  }
}

// ---------------------------------------------------------------------------
// Test 4: Sequential Locker Reuse on Same Isolate (Warm Reuse)
// ---------------------------------------------------------------------------

/// Locker → IsolateScope → JS → drop all → repeat 10 times.
/// This is the basic warm reuse pattern for isolate pooling.
#[test]
fn sequential_locker_reuse_warm() {
  let _setup_guard = setup();
  let mut isolate = v8::Isolate::new_unentered(Default::default());

  for i in 0..10 {
    let mut locker = v8::Locker::new(&mut isolate);
    let expected = (i + 1) * 10;
    let expr = format!("{} * 10", i + 1);
    let result = eval_js(&mut locker, &expr);
    assert_eq!(result, expected);
  }
}

// ---------------------------------------------------------------------------
// Test 5: Two Concurrent Lockers, LIFO Drop Order
// ---------------------------------------------------------------------------

/// Locker(A) and Locker(B) both alive. Each does IsolateScope cycles.
/// Drop B first, then A (LIFO). Must work.
#[test]
fn two_concurrent_lockers_lifo_drop() {
  let _setup_guard = setup();
  let mut isolate_a = v8::Isolate::new_unentered(Default::default());
  let mut isolate_b = v8::Isolate::new_unentered(Default::default());

  let mut locker_a = v8::Locker::new(&mut isolate_a);
  let mut locker_b = v8::Locker::new(&mut isolate_b);

  // Work on A
  assert_eq!(eval_js(&mut locker_a, "10 + 5"), 15);

  // Work on B
  assert_eq!(eval_js(&mut locker_b, "20 + 5"), 25);

  // LIFO drop: B first, then A
  drop(locker_b);
  drop(locker_a);
}

// ---------------------------------------------------------------------------
// Test 6: Two Concurrent Lockers, Non-LIFO Drop, Then Reuse with JS
// ---------------------------------------------------------------------------

/// The exact production scenario:
/// Locker(A) → Locker(B) → A does IsolateScope + JS → B does IsolateScope + JS
/// → drop A → drop B (non-LIFO) → Locker(A) → IsolateScope → JS → verify.
#[test]
fn two_concurrent_lockers_non_lifo_drop_then_reuse() {
  let _setup_guard = setup();
  let mut isolate_a = v8::Isolate::new_unentered(Default::default());
  let mut isolate_b = v8::Isolate::new_unentered(Default::default());

  let mut locker_a = v8::Locker::new(&mut isolate_a);
  let mut locker_b = v8::Locker::new(&mut isolate_b);

  // Work on A under IsolateScope
  assert_eq!(eval_js(&mut locker_a, "100 + 11"), 111);

  // Work on B under IsolateScope
  assert_eq!(eval_js(&mut locker_b, "200 + 22"), 222);

  // Non-LIFO drop: A first, then B
  drop(locker_a);
  drop(locker_b);

  // Reuse A — must not segfault
  let mut locker_a = v8::Locker::new(&mut isolate_a);
  let result = eval_js(&mut locker_a, "333 + 444");
  assert_eq!(result, 777);
}

// ---------------------------------------------------------------------------
// Test 7: Stress Test — 100 Sequential Warm Reuse Cycles
// ---------------------------------------------------------------------------

/// One isolate. 100 iterations of: Locker → IsolateScope → HandleScope →
/// ContextScope → execute JS → drop everything. Verify no corruption
/// accumulates.
#[test]
fn stress_100_sequential_warm_reuse() {
  let _setup_guard = setup();
  let mut isolate = v8::Isolate::new_unentered(Default::default());

  for i in 0..100 {
    let mut locker = v8::Locker::new(&mut isolate);
    let expr = format!("{} + 1", i);
    let result = eval_js(&mut locker, &expr);
    assert_eq!(result, i + 1, "Failed at iteration {}", i);
  }
}

// ---------------------------------------------------------------------------
// Test 8: Reentrant Locker on Same Isolate
// ---------------------------------------------------------------------------

/// V8's C++ Locker supports recursive locking on the same thread.
///
/// Note: The Rust borrow checker prevents creating two `Locker`s for the
/// same `UnenteredIsolate` simultaneously (compile_fail/locker_double_borrow.rs
/// tests this). However, V8's internal C++ Locker does support recursive
/// locking when the same thread already holds the lock.
///
/// This test verifies sequential reuse works correctly (which exercises the
/// same V8 code path as recursive locking would).
#[test]
fn reentrant_locker_sequential() {
  let _setup_guard = setup();
  let mut isolate = v8::Isolate::new_unentered(Default::default());

  // First Locker
  {
    let mut locker = v8::Locker::new(&mut isolate);
    assert_eq!(eval_js(&mut locker, "1 + 1"), 2);
  }

  // Second Locker (sequential, same isolate)
  {
    let mut locker = v8::Locker::new(&mut isolate);
    assert_eq!(eval_js(&mut locker, "2 + 2"), 4);
  }

  // Third Locker — verify no state corruption
  {
    let mut locker = v8::Locker::new(&mut isolate);
    assert_eq!(eval_js(&mut locker, "3 + 3"), 6);
  }
}

// ---------------------------------------------------------------------------
// Test 9: v8::Global Handles Across Locker Lifetimes
// ---------------------------------------------------------------------------

/// Under Locker 1: create a v8::Context, store it as v8::Global<Context>.
/// Drop Locker 1. Under Locker 2: use the Global<Context> to create a
/// ContextScope, execute JS.
///
/// This simulates the runtime's context warm reuse pattern where a Global
/// handle to a context outlives individual Locker lifetimes.
#[test]
fn global_handles_across_locker_lifetimes() {
  let _setup_guard = setup();
  let mut isolate = v8::Isolate::new_unentered(Default::default());

  // Under Locker 1: create a context and store as Global
  let global_context: v8::Global<v8::Context>;
  {
    let mut locker = v8::Locker::new(&mut isolate);
    let _isolate_scope = locker.enter();
    let scope = pin!(v8::HandleScope::new(&mut *locker));
    let scope = &mut scope.init();
    let context = v8::Context::new(scope, Default::default());
    global_context = v8::Global::new(scope, context);
  }

  // Under Locker 2: reuse the Global<Context>
  {
    let mut locker = v8::Locker::new(&mut isolate);
    let _isolate_scope = locker.enter();
    let scope = pin!(v8::HandleScope::new(&mut *locker));
    let scope = &mut scope.init();
    let context = v8::Local::new(scope, &global_context);
    let scope = &mut v8::ContextScope::new(scope, context);

    let code = v8::String::new(scope, "42 * 2").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    assert_eq!(result.to_integer(scope).unwrap().value(), 84);
  }

  // Under Locker 3: verify the context still works after multiple reuses
  {
    let mut locker = v8::Locker::new(&mut isolate);
    let _isolate_scope = locker.enter();
    let scope = pin!(v8::HandleScope::new(&mut *locker));
    let scope = &mut scope.init();
    let context = v8::Local::new(scope, &global_context);
    let scope = &mut v8::ContextScope::new(scope, context);

    let code = v8::String::new(scope, "'reused'").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    let result_str = result.to_rust_string_lossy(scope);
    assert_eq!(result_str, "reused");
  }
}

// ---------------------------------------------------------------------------
// Test 10: Non-LIFO Drop With Interleaved IsolateScope Work
// ---------------------------------------------------------------------------

/// Maximizes interleaving. Create Lockers for A and B. Then alternately:
/// IsolateScope(A) → JS → drop scope → IsolateScope(B) → JS → drop scope
/// → repeat 5 times. Then drop A before B. Then Locker(A) → JS.
#[test]
fn non_lifo_drop_interleaved_isolate_scope_work() {
  let _setup_guard = setup();
  let mut isolate_a = v8::Isolate::new_unentered(Default::default());
  let mut isolate_b = v8::Isolate::new_unentered(Default::default());

  let mut locker_a = v8::Locker::new(&mut isolate_a);
  let mut locker_b = v8::Locker::new(&mut isolate_b);

  // Interleave IsolateScope work between A and B
  for i in 0..5 {
    // Work on A
    {
      let expr_a = format!("{} + 100", i);
      let result_a = eval_js(&mut locker_a, &expr_a);
      assert_eq!(result_a, i + 100, "A failed at iteration {}", i);
    }

    // Work on B
    {
      let expr_b = format!("{} + 200", i);
      let result_b = eval_js(&mut locker_b, &expr_b);
      assert_eq!(result_b, i + 200, "B failed at iteration {}", i);
    }
  }

  // Non-LIFO drop: A first, then B
  drop(locker_a);
  drop(locker_b);

  // Reuse A after non-LIFO drop — must not segfault
  let mut locker_a = v8::Locker::new(&mut isolate_a);
  let result = eval_js(&mut locker_a, "999");
  assert_eq!(result, 999);
}
