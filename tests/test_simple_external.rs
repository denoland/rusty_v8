#[cfg(test)]
mod test_simple_external {
  #[test]
  fn test() {
    v8::V8::set_flags_from_string(
      "--no_freeze_flags_after_init --expose_gc --harmony-shadow-realm --allow_natives_syntax --turbo_fast_api_calls --js-source-phase-imports",
    );
    v8::V8::initialize_platform(
      v8::new_default_platform(0, false).make_shared(),
    );
    v8::V8::initialize();
    let isolate = &mut v8::Isolate::new(Default::default());
    v8::scope!(let scope, isolate);

    let ex1_value =
      Box::into_raw(Box::new(1234567usize)) as *mut std::ffi::c_void;
    let ex1_handle_a = v8::External::new(scope, ex1_value);
    assert_eq!(ex1_handle_a.value(), ex1_value);

    let b_value =
      Box::into_raw(Box::new(2334567usize)) as *mut std::ffi::c_void;
    let ex1_handle_b = v8::External::new(scope, b_value);
    assert_eq!(ex1_handle_b.value(), b_value);

    let ex2_value =
      Box::into_raw(Box::new(2334567usize)) as *mut std::ffi::c_void;
    let ex3_value = Box::into_raw(Box::new(-2isize)) as *mut std::ffi::c_void;

    let ex2_handle_a = v8::External::new(scope, ex2_value);
    let ex3_handle_a = v8::External::new(scope, ex3_value);

    assert!(ex1_handle_a != ex2_handle_a);
    assert!(ex2_handle_a != ex3_handle_a);
    assert!(ex3_handle_a != ex1_handle_a);

    assert_ne!(ex2_value, ex3_value);
    assert_eq!(ex2_handle_a.value(), ex2_value);
    assert_eq!(ex3_handle_a.value(), ex3_value);

    drop(unsafe { Box::from_raw(ex1_value as *mut usize) });
    drop(unsafe { Box::from_raw(b_value as *mut usize) });
    drop(unsafe { Box::from_raw(ex2_value as *mut usize) });
    drop(unsafe { Box::from_raw(ex3_value as *mut isize) });
  }
}
