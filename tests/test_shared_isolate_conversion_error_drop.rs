// Copyright 2019-2026 the Deno authors. All rights reserved. MIT license.

#[test]
fn dropping_conversion_error_while_unwinding_leaks_safely() {
  v8::V8::set_flags_from_string("--no_freeze_flags_after_init");
  v8::V8::initialize_platform(
    v8::new_unprotected_default_platform(0, false).make_shared(),
  );
  v8::V8::initialize();

  let snapshot_creator = v8::Isolate::snapshot_creator(None, None);
  let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    // `unwrap` begins unwinding and then drops the error. The retained snapshot
    // creator cannot run `OwnedIsolate::drop`, so the error must leak it instead
    // of causing a second panic and aborting the process.
    let _ = unsafe { snapshot_creator.try_into_shared() }.unwrap();
  }));
  assert!(panic.is_err());
}
