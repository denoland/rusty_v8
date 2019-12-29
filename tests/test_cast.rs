use rusty_v8 as v8;
use rusty_v8::array_buffer_view::ArrayBufferView;
use rusty_v8::{
  Boolean, Context, Function, Integer, Local, Number, Object, ToLocal, Value,
};

#[test]
#[allow(clippy::float_cmp)]
fn test_downcast() {
  let platform = v8::platform::new_default_platform();
  v8::V8::initialize_platform(platform);
  v8::V8::initialize();

  let mut create_params = v8::Isolate::create_params();
  create_params.set_array_buffer_allocator(v8::new_default_allocator());
  let isolate = v8::Isolate::new(create_params);
  let mut locker = v8::Locker::new(&isolate);

  {
    let mut handle_scope = v8::HandleScope::new(&mut locker);
    let scope = handle_scope.enter();

    let mut context = v8::Context::new(scope);
    context.enter();

    let string: Local<v8::String> = eval(scope, context, "'hello'").into();
    assert!(string.is_string());
    assert_eq!(string.to_rust_string_lossy(scope), "hello");

    let boolean: Local<Boolean> = eval(scope, context, "true").into();
    assert!(boolean.is_boolean());

    eval(scope, context, "var obj = { prop:'value' }");
    let obj_key = v8::String::new(scope, "obj").unwrap();
    let obj = context
        .global(scope)
        .get(scope, context, obj_key.into())
        .unwrap();
    let obj: Local<Object> = obj.into();
    assert!(obj.is_object());
    let prop_key = v8::String::new(scope, "prop").unwrap();
    assert!(obj
        .get(scope, context, prop_key.into())
        .unwrap()
        .is_string());

    let integer: Local<Integer> = eval(scope, context, "66").into();
    assert!(integer.is_number());
    assert_eq!(integer.value(), 66);

    let number: Local<Number> = eval(scope, context, "3.5").into();
    assert!(number.is_number());
    assert_eq!(number.value(), 3.5);

    let uint8array: Local<ArrayBufferView> =
        eval(scope, context, "new Uint8Array([10])").into();
    assert_eq!(uint8array.byte_length(), 1);
    assert!(uint8array.is_object());

    let func: Local<Function> = eval(scope, context, "() => undefined").into();
    assert!(func.is_object());
    assert!(func.is_function());

    context.exit();
  }

  drop(locker);
}

fn eval<'sc>(
  scope: &mut impl ToLocal<'sc>,
  context: Local<Context>,
  code: &str,
) -> v8::Local<'sc, Value> {
  let source = v8::String::new(scope, code).unwrap();
  let mut script = v8::Script::compile(scope, context, source, None).unwrap();
  script.run(scope, context).unwrap()
}
