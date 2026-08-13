// Copyright 2018-2025 the Deno authors. All rights reserved. MIT license.

#[test]
#[should_panic(expected = "Invalid global state")]
fn isolate_group_try_create_requires_initialized_v8() {
  let _ = v8::IsolateGroup::try_create();
}
