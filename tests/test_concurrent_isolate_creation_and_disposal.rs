// This is flaky on cross (QEMU bug)
// but otherwise works fine on real device.
#![cfg(not(target_os = "android"))]

use std::iter::repeat_with;
use std::thread;

#[test]
fn concurrent_isolate_creation_and_disposal() {
  let platform = v8::new_single_threaded_default_platform(false).make_shared();
  v8::V8::initialize_platform(platform);
  v8::V8::initialize();

  for round in 0..1000 {
    eprintln!("round {}", round);

    let threads = repeat_with(|| {
      thread::spawn(|| {
        v8::Isolate::new(Default::default());
      })
    })
    .take(16)
    .collect::<Vec<_>>();

    for join_handle in threads {
      join_handle.join().unwrap();
    }
  }

  unsafe { v8::V8::dispose() };
  v8::V8::dispose_platform();
}
