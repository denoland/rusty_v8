// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.

//! Checks that handles which remember their `Isolate` tell a disposed isolate
//! apart from a later one allocated at the same address. This is a separate
//! test binary so that no other test allocates isolates at the same time.

fn addr(isolate: v8::UnsafeRawIsolatePtr) -> usize {
  // SAFETY: `UnsafeRawIsolatePtr` is a transparent wrapper around a pointer.
  unsafe { std::mem::transmute(isolate) }
}

#[test]
fn eternal_of_disposed_isolate_panics_in_isolate_at_same_address() {
  v8::V8::initialize_platform(v8::new_default_platform(0, false).make_shared());
  v8::V8::initialize();

  let eternal = v8::Eternal::<v8::String>::empty();
  let address = {
    let a = &mut v8::Isolate::new(Default::default());
    let address = addr(unsafe { a.as_raw_isolate_ptr() });
    v8::scope!(let scope, a);
    let s = v8::String::new(scope, "from a").unwrap();
    eternal.set(scope, s);
    address
  };

  // Allocators tend to hand a freed isolate's memory to the next isolate.
  // Isolates that land elsewhere are kept alive so that the allocator moves
  // on, and are dropped in the reverse order of creation.
  let mut isolates = Vec::new();
  let mut reused = false;
  for _ in 0..16 {
    let isolate = v8::Isolate::new(Default::default());
    reused = addr(unsafe { isolate.as_raw_isolate_ptr() }) == address;
    isolates.push(isolate);
    if reused {
      break;
    }
  }
  let result = reused.then(|| {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
      v8::scope!(let scope, isolates.last_mut().unwrap());
      eternal.get(scope);
    }))
  });
  while let Some(isolate) = isolates.pop() {
    drop(isolate);
  }

  let Some(result) = result else {
    eprintln!("skipped: no isolate was allocated at a disposed one's address");
    return;
  };
  let message = result.expect_err("get() should panic");
  assert_eq!(
    message.downcast_ref::<&str>().copied(),
    Some(
      "attempt to get a handle in an Isolate other than the one it was set in"
    )
  );
}
