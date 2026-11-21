// A context slot keeps a `Weak<Context>` in the context's annex, with a
// guaranteed finalizer that frees the annex. Dropping the isolate drains that
// finalizer after the isolate handle is nulled, so the `Weak` is dropped while
// V8 still holds its weak global. With marking in progress, V8's teardown GC
// then runs the weak callback on the `WeakData`, which must still be alive.
#[test]
fn context_slot_weak_survives_isolate_teardown() {
  v8::V8::set_flags_from_string("--stress-compaction --stress-marking=1");
  v8::V8::initialize_platform(v8::new_default_platform(0, false).make_shared());
  v8::V8::initialize();

  for i in 0..10 {
    let mut isolate = v8::Isolate::new(Default::default());
    {
      v8::scope!(let scope, &mut isolate);
      let context = v8::Context::new(scope, Default::default());
      context.set_slot(std::rc::Rc::new(i));
    }
    drop(isolate);
  }
}
