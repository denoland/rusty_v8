
#[test]
fn test_isolate_pooling() {
  v8::V8::initialize();
  let mut iso1 = v8::Isolate::new(Default::default());
  let mut iso2 = v8::Isolate::new(Default::default());

  {
    let scope1 = std::pin::pin!(v8::HandleScope::new(&mut iso1));
    let mut scope1 = scope1.init();
    let _ctx1 = v8::Context::new(&mut scope1, Default::default());
  }

  {
    let scope2 = std::pin::pin!(v8::HandleScope::new(&mut iso2));
    let mut scope2 = scope2.init();
    let _ctx2 = v8::Context::new(&mut scope2, Default::default());
  }
  
  // Test interleaved usage
  {
      let scope1 = std::pin::pin!(v8::HandleScope::new(&mut iso1));
      let mut scope1 = scope1.init();
      let _ctx1 = v8::Context::new(&mut scope1, Default::default());
      
      let scope2 = std::pin::pin!(v8::HandleScope::new(&mut iso2));
      let mut scope2 = scope2.init();
      let _ctx2 = v8::Context::new(&mut scope2, Default::default());
  }
}
