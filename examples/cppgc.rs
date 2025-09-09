// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.
//
// This sample program shows how to set up a stand-alone cppgc heap.

// Simple string rope to illustrate allocation and garbage collection below.
// The rope keeps the next parts alive via regular managed reference.

struct Rope {
  part: String,
  next: v8::cppgc::Member<Rope>,
}

impl std::fmt::Display for Rope {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.part)?;
    unsafe {
      // SAFETY: `next` is visited in `trace()`.
      if let Some(next) = self.next.get() {
        write!(f, "{next}")?;
      }
    }
    Ok(())
  }
}

impl Rope {
  pub fn new(part: String, next: Option<v8::cppgc::UnsafePtr<Rope>>) -> Rope {
    let next = match next {
      Some(p) => v8::cppgc::Member::new(&p),
      None => v8::cppgc::Member::empty(),
    };
    Self { part, next }
  }
}

unsafe impl v8::cppgc::GarbageCollected for Rope {
  fn trace(&self, visitor: &mut v8::cppgc::Visitor) {
    visitor.trace(&self.next);
  }

  fn get_name(&self) -> &'static std::ffi::CStr {
    c"Rope"
  }
}

impl Drop for Rope {
  fn drop(&mut self) {
    println!("Dropping: {}", self.part);
  }
}

fn main() {
  let platform = v8::new_default_platform(0, false).make_shared();
  v8::V8::initialize_platform(platform.clone());
  v8::V8::initialize();
  v8::cppgc::initialize_process(platform.clone());

  {
    // Create a managed heap.
    let heap =
      v8::cppgc::Heap::create(platform, v8::cppgc::HeapCreateParams::default());

    // Allocate a string rope on the managed heap.
    let rope = unsafe {
      v8::cppgc::make_garbage_collected(
        &heap,
        Rope::new(
          String::from("Hello "),
          Some(v8::cppgc::make_garbage_collected(
            &heap,
            Rope::new(String::from("World!"), None),
          )),
        ),
      )
    };

    println!("{}", unsafe { rope.as_ref() });

    // Manually trigger garbage collection.
    heap.enable_detached_garbage_collections_for_testing();

    println!("Collect: MayContainHeapPointers");
    unsafe {
      heap.collect_garbage_for_testing(
        v8::cppgc::EmbedderStackState::MayContainHeapPointers,
      );
    }

    // Should still be live here:
    println!("{}", unsafe { rope.as_ref() });

    println!("Collect: NoHeapPointers");
    unsafe {
      heap.collect_garbage_for_testing(
        v8::cppgc::EmbedderStackState::NoHeapPointers,
      );
    }

    // Should be dead now.
  }

  // Gracefully shutdown the process.
  unsafe {
    v8::cppgc::shutdown_process();
    v8::V8::dispose();
  }
  v8::V8::dispose_platform();
}
