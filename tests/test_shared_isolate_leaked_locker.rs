// Copyright 2019-2026 the Deno authors. All rights reserved. MIT license.

#[test]
fn dropping_shared_isolate_after_forgetting_locker_leaks_safely() {
  v8::V8::set_flags_from_string("--no_freeze_flags_after_init");
  v8::V8::initialize_platform(
    v8::new_unprotected_default_platform(0, false).make_shared(),
  );
  v8::V8::initialize();

  let shared = unsafe {
    v8::Isolate::new(Default::default())
      .try_into_shared()
      .unwrap()
  };
  let locker = shared.lock();
  std::mem::forget(locker);

  // The leaked Locker owns the shared inner allocation. Dropping the public
  // handle must therefore leak the entered isolate, not try to dispose it.
  drop(shared);
}
