fn main() {
  let isolate = &mut v8::Isolate::new(Default::default());
  v8::scope!(let scope1, isolate);

  let _local = {
    v8::scope!(let scope, scope1);

    v8::Integer::new(scope, 123)
  };
}
