struct Resource {
  name: String,
}

impl Drop for Resource {
  fn drop(&mut self) {
    println!("Dropping {}", self.name);
  }
}

fn main() {
  let platform = v8::new_default_platform(0, false).make_shared();
  v8::V8::initialize_platform(platform.clone());
  v8::V8::initialize();

  v8::cppgc::initalize_process(platform.clone());
  {
    let heap = v8::cppgc::Heap::create(platform);

    let obj = make_object(&*heap, "hello");

    heap.enable_detached_garbage_collections_for_testing();

    heap.force_garbage_collection_slow(
      v8::cppgc::EmbedderStackState::MayContainHeapPointers,
    );
    heap.force_garbage_collection_slow(
      v8::cppgc::EmbedderStackState::NoHeapPointers,
    );
  }
  unsafe { v8::cppgc::shutdown_process() };
}

fn make_object(heap: &v8::cppgc::Heap, name: &str) -> *mut Resource {
  extern "C" fn trace(visitor: *mut v8::cppgc::Visitor, obj: *mut ()) {
    let obj = unsafe { &*(obj as *const Resource) };
    println!("Trace {}", obj.name);
  }

  let val = Box::new(Resource {
    name: name.to_string(),
  });
  let obj = v8::cppgc::make_garbage_collected(heap, val, trace);
  return obj;
}
