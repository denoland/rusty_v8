// These tests mock out an organizational pattern that we hope to use in Deno.
// There we want to wrap v8::Isolate to provide extra functionality at multiple
// layers: v8::Isolate -> CoreIsolate -> EsIsolate
// This demonstrates how this can be done in a safe way.

use std::ops::Deref;
use std::ops::DerefMut;
use std::rc::Rc;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::Once;

struct CoreIsolate(v8::OwnedIsolate);

struct CoreIsolateState {
  drop_count: Rc<AtomicUsize>,
  i: usize,
}

impl Drop for CoreIsolateState {
  fn drop(&mut self) {
    self.drop_count.fetch_add(1, Ordering::SeqCst);
  }
}

impl CoreIsolate {
  fn new(drop_count: Rc<AtomicUsize>) -> CoreIsolate {
    static START: Once = Once::new();
    START.call_once(|| {
      v8::V8::initialize_platform(
        v8::new_default_platform(0, false).make_shared(),
      );
      v8::V8::initialize();
    });
    let mut isolate = v8::Isolate::new(Default::default());
    let state = CoreIsolateState { drop_count, i: 0 };
    isolate.set_slot(state);
    CoreIsolate(isolate)
  }

  // Returns false if there was an error.
  fn execute(&mut self, code: &str) -> bool {
    let scope = &mut v8::HandleScope::new(&mut self.0);
    let context = v8::Context::new(scope);
    let scope = &mut v8::ContextScope::new(scope, context);
    let source = v8::String::new(scope, code).unwrap();
    let script = v8::Script::compile(scope, source, None).unwrap();
    let r = script.run(scope);
    r.is_some()
  }

  fn get_i(&self) -> usize {
    let s = self.0.get_slot::<CoreIsolateState>().unwrap();
    s.i
  }

  fn set_i(&mut self, i: usize) {
    let s = self.0.get_slot_mut::<CoreIsolateState>().unwrap();
    s.i = i;
  }
}

impl Deref for CoreIsolate {
  type Target = v8::Isolate;

  fn deref(&self) -> &v8::Isolate {
    &self.0
  }
}

impl DerefMut for CoreIsolate {
  fn deref_mut(&mut self) -> &mut v8::Isolate {
    &mut self.0
  }
}

struct EsIsolate(CoreIsolate);

struct EsIsolateState {
  drop_count: Rc<AtomicUsize>,
  x: bool,
}

impl Drop for EsIsolateState {
  fn drop(&mut self) {
    self.drop_count.fetch_add(1, Ordering::SeqCst);
  }
}

impl EsIsolate {
  fn new(drop_count: Rc<AtomicUsize>) -> Self {
    let mut core_isolate = CoreIsolate::new(drop_count.clone());
    let state = EsIsolateState {
      drop_count,
      x: false,
    };
    core_isolate.set_slot(state);
    EsIsolate(core_isolate)
  }

  fn get_x(&self) -> bool {
    let state = self.0.get_slot::<EsIsolateState>().unwrap();
    state.x
  }

  fn set_x(&mut self, x: bool) {
    let state = self.0.get_slot_mut::<EsIsolateState>().unwrap();
    state.x = x;
  }
}

impl Deref for EsIsolate {
  type Target = CoreIsolate;

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl DerefMut for EsIsolate {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.0
  }
}

#[test]
fn slots_layer1() {
  let drop_count = Rc::new(AtomicUsize::new(0));
  let mut core_isolate = CoreIsolate::new(drop_count.clone());
  // The existence of a IsolateHandle that outlives the isolate should not
  // inhibit dropping of slot contents.
  let isolate_handle = core_isolate.thread_safe_handle();
  assert!(core_isolate.execute("1 + 1"));
  assert!(!core_isolate.execute("throw 'foo'"));
  assert_eq!(0, core_isolate.get_i());
  core_isolate.set_i(123);
  assert_eq!(123, core_isolate.get_i());
  assert_eq!(drop_count.load(Ordering::SeqCst), 0);
  // Check that we can deref CoreIsolate by running a random v8::Isolate method
  core_isolate.perform_microtask_checkpoint();
  drop(core_isolate);
  assert_eq!(drop_count.load(Ordering::SeqCst), 1);
  drop(isolate_handle);
}

#[test]
fn slots_layer2() {
  let drop_count = Rc::new(AtomicUsize::new(0));
  let mut es_isolate = EsIsolate::new(drop_count.clone());
  // We can deref to CoreIsolate and use execute...
  assert!(es_isolate.execute("1 + 1"));
  assert!(!es_isolate.execute("throw 'bar'"));
  // We can use get_x set_x
  assert!(!es_isolate.get_x());
  es_isolate.set_x(true);
  assert!(es_isolate.get_x());
  // Check that we can deref all the way to a v8::Isolate method
  es_isolate.perform_microtask_checkpoint();

  // When we drop, both CoreIsolateState and EsIsolateState should be dropped.
  assert_eq!(drop_count.load(Ordering::SeqCst), 0);
  drop(es_isolate);
  assert_eq!(drop_count.load(Ordering::SeqCst), 2);
}

// General test for the slots system, not specific for the Deno pattern.

struct TestState(i32);

#[test]
fn slots_general_1() {
  let mut core_isolate = CoreIsolate::new(Rc::new(AtomicUsize::new(0)));

  // Set a value in the slots system.
  let first_add = core_isolate.set_slot::<TestState>(TestState(0));

  // Verify that this was the first time a value of this type was added.
  assert!(first_add);

  let second_add = core_isolate.set_slot::<TestState>(TestState(1));

  // Verify that the set operation cause an existing value to be replaced and dropped.
  assert!(!second_add);

  // Increase the value stored.
  core_isolate.get_slot_mut::<TestState>().unwrap().0 += 5;

  // Verify the value has changed,
  // and that it was really replaced (if it were not, the result would be 5).
  assert_eq!(core_isolate.get_slot::<TestState>().unwrap().0, 6);

  // Remove the value out of the slot.
  let value = core_isolate.remove_slot::<TestState>().unwrap();

  // Verify that we got the proper value.
  assert_eq!(value.0, 6);

  // Verify that the slot is empty now.
  assert!(core_isolate.remove_slot::<TestState>().is_none());
}

#[test]
fn slots_general_2() {
  let drop_count = Rc::new(AtomicUsize::new(0));
  let mut core_isolate = CoreIsolate::new(drop_count.clone());

  let state: CoreIsolateState =
    core_isolate.remove_slot::<CoreIsolateState>().unwrap();
  drop(core_isolate);

  // The state should not be dropped with the isolate because we own it now.
  assert_eq!(drop_count.load(Ordering::SeqCst), 0);

  // We're dropping it now on purpose.
  drop(state);
  assert_eq!(drop_count.load(Ordering::SeqCst), 1);
}
