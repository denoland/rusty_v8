fn main() {
  let isolate = &mut v8::Isolate::new(Default::default());
  let scope1 = std::pin::pin!(v8::HandleScope::new(&mut *isolate));
  let scope1 = &mut scope1.init();

  let _local = {
    let scope = std::pin::pin!(v8::HandleScope::new(scope1));
    let scope = &mut scope.init();

    v8::Integer::new(scope, 123)
  };
}
