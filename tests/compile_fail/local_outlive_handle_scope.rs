fn main() {
  let isolate = &mut v8::Isolate::new(Default::default());
  v8::make_handle_scope!(let scope1, isolate);

  let _local = {
    v8::make_handle_scope!(let scope, scope1);

    v8::Integer::new(scope, 123)
  };
}
