// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.
//
// This sample program shows how to set up a stand-alone cppgc heap.
use std::ops::Deref;

// Simple string rope to illustrate allocation and garbage collection below.
// The rope keeps the next parts alive via regular managed reference.
struct Rope {
  part: String,
  next: Option<v8::cppgc::Member<Rope>>,
}

impl std::fmt::Display for Rope {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.part)?;
    if let Some(next) = &self.next {
      write!(f, "{}", next.deref())?;
    }
    Ok(())
  }
}

impl Rope {
  pub fn new(part: String, next: Option<v8::cppgc::Member<Rope>>) -> Box<Rope> {
    Box::new(Self { part, next })
  }
}

impl v8::cppgc::GarbageCollected for Rope {
  fn trace(&self, visitor: &v8::cppgc::Visitor) {
    if let Some(member) = &self.next {
      visitor.trace(member);
    }
  }
}

impl Drop for Rope {
  fn drop(&mut self) {
    println!("Dropping {}", self.part);
  }
}

const DEFAULT_CPP_GC_EMBEDDER_ID: u16 = 0xde90;

fn main() {
  let platform = v8::new_default_platform(0, false).make_shared();
  v8::V8::initialize_platform(platform.clone());
  v8::V8::initialize();
  v8::cppgc::initalize_process(platform.clone());

  {
    // Create a managed heap.
    let heap = v8::cppgc::Heap::create(
      platform,
      v8::cppgc::HeapCreateParams::new(v8::cppgc::WrapperDescriptor::new(
        0,
        1,
        DEFAULT_CPP_GC_EMBEDDER_ID,
      )),
    );

    // Allocate a string rope on the managed heap.
    let rope = v8::cppgc::make_garbage_collected(
      &heap,
      Rope::new(
        String::from("Hello "),
        Some(v8::cppgc::make_garbage_collected(
          &heap,
          Rope::new(String::from("World!"), None),
        )),
      ),
    );

    println!("{}", unsafe { rope.get() });
    // Manually trigger garbage collection.
    heap.enable_detached_garbage_collections_for_testing();
    heap.collect_garbage_for_testing(
      v8::cppgc::EmbedderStackState::MayContainHeapPointers,
    );
    heap.collect_garbage_for_testing(
      v8::cppgc::EmbedderStackState::NoHeapPointers,
    );
  }

  // Gracefully shutdown the process.
  unsafe {
    v8::cppgc::shutdown_process();
    v8::V8::dispose();
  }
  v8::V8::dispose_platform();
}
