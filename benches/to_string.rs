fn main() {
  // Skip running benchmarks in debug or CI.
  if cfg!(debug_assertions) || std::env::var("CI").is_ok() {
    return;
  }
  v8::V8::set_flags_from_string(
    "--turbo_fast_api_calls --allow_natives_syntax",
  );
  let platform = v8::new_default_platform(0, false).make_shared();
  v8::V8::initialize_platform(platform);
  v8::V8::initialize();
  let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
  let handle_scope = &mut v8::HandleScope::new(isolate);
  let context = v8::Context::new(handle_scope, Default::default());
  let scope = &mut v8::ContextScope::new(handle_scope, context);
  let global = context.global(scope);
  {
    let func = v8::Function::new(
      scope,
      |scope: &mut v8::HandleScope,
       arg: v8::FunctionCallbackArguments,
       mut rv: v8::ReturnValue| {
        let first = arg.get(0);
        let string = first.to_string(scope).unwrap();
        let view = v8::ValueView::new(scope, string);
        if let v8::ValueViewData::OneByte(data) = view.data() {
          if data.is_ascii() {
            rv.set_uint32(data.len() as u32);
            return;
          }
        };

        let mut buffer = [std::mem::MaybeUninit::<u8>::uninit(); 1024];
        let out = string.to_rust_cow_lossy(scope, &mut buffer);

        rv.set_uint32(out.len() as u32);
      },
    )
    .unwrap();
    let name = v8::String::new(scope, "valueview").unwrap();
    global.set(scope, name.into(), func.into()).unwrap();
  }

  {
    let func = v8::Function::new(
      scope,
      |scope: &mut v8::HandleScope,
       arg: v8::FunctionCallbackArguments,
       mut rv: v8::ReturnValue| {
        let first = arg.get(0);
        let string = first.to_string(scope).unwrap();

        let mut buffer = [std::mem::MaybeUninit::<u8>::uninit(); 1024];
        let out = string.to_rust_cow_lossy(scope, &mut buffer);

        rv.set_uint32(out.len() as u32);
      },
    )
    .unwrap();
    let name =
      v8::String::new(scope, "to_rust_cow_lossy_stack_buffer").unwrap();
    global.set(scope, name.into(), func.into()).unwrap();
  }

  {
    let func = v8::Function::new(
      scope,
      |scope: &mut v8::HandleScope,
       arg: v8::FunctionCallbackArguments,
       mut rv: v8::ReturnValue| {
        let first = arg.get(0);
        let string = first.to_string(scope).unwrap();

        let mut buffer = [std::mem::MaybeUninit::<u8>::uninit(); 0];
        let out = string.to_rust_cow_lossy(scope, &mut buffer);

        rv.set_uint32(out.len() as u32);
      },
    )
    .unwrap();
    let name = v8::String::new(scope, "to_rust_cow_lossy").unwrap();
    global.set(scope, name.into(), func.into()).unwrap();
  }

  {
    fn fast_fn(
      _recv: v8::Local<v8::Object>,
      data: *const v8::fast_api::FastApiOneByteString,
    ) -> u32 {
      let data = unsafe { &*data }.as_bytes();
      data.len() as u32
    }
    const FAST_CALL: v8::fast_api::FastFunction =
      v8::fast_api::FastFunction::new(
        &[
          v8::fast_api::Type::V8Value,
          v8::fast_api::Type::SeqOneByteString,
        ],
        v8::fast_api::CType::Uint32,
        fast_fn as _,
      );
    let template = v8::FunctionTemplate::builder(
      |scope: &mut v8::HandleScope,
       arg: v8::FunctionCallbackArguments,
       mut rv: v8::ReturnValue| {
        let first = arg.get(0);
        let string = first.to_string(scope).unwrap();
        let view = v8::ValueView::new(scope, string);
        if let v8::ValueViewData::OneByte(data) = view.data() {
          if data.is_ascii() {
            rv.set_uint32(data.len() as u32);
            return;
          }
        };

        let mut buffer = [std::mem::MaybeUninit::<u8>::uninit(); 1024];
        let out = string.to_rust_cow_lossy(scope, &mut buffer);

        rv.set_uint32(out.len() as u32);
      },
    )
    .build_fast(scope, &FAST_CALL, None, None, None);
    let name = v8::String::new(scope, "fast_seqonebytestr").unwrap();
    let value = template.get_function(scope).unwrap();

    global.set(scope, name.into(), value.into()).unwrap();
  }

  let runs = 100_000_000;

  for x in &[
    "to_rust_cow_lossy",
    "to_rust_cow_lossy_stack_buffer",
    "valueview",
    "fast_seqonebytestr",
  ] {
    let code = format!(
      "
            function bench() {{ return {}('Hello'); }};
            runs = {};
            start = Date.now();
            for (i = 0; i < runs; i++) bench();
            Date.now() - start;
          ",
      x, runs
    );

    let r = eval(scope, &code).unwrap();
    let number = r.to_number(scope).unwrap();
    let total_ms = number.number_value(scope).unwrap();
    let total_ns = 1e6 * total_ms;
    let ns_per_run = total_ns / (runs as f64);
    let mops_per_sec = (runs as f64) / (total_ms / 1000.0) / 1e6;
    println!(
      "  {:.1} ns per run {:.1} million ops/sec → {}",
      ns_per_run, mops_per_sec, x
    );
  }
}

fn eval<'s>(
  scope: &mut v8::HandleScope<'s>,
  code: &str,
) -> Option<v8::Local<'s, v8::Value>> {
  let scope = &mut v8::EscapableHandleScope::new(scope);
  let source = v8::String::new(scope, code).unwrap();
  let script = v8::Script::compile(scope, source, None).unwrap();
  let r = script.run(scope);
  r.map(|v| scope.escape(v))
}
