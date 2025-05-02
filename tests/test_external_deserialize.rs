use v8::MapFnTo;

fn callback(
  scope: &mut v8::HandleScope,
  args: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue<v8::Value>,
) {
  let data = args.data().cast::<v8::External>().value();
  let data = data as u64;
  rv.set(v8::BigInt::new_from_u64(scope, data).into());
}

#[test]
fn external_deserialize() {
  let platform = v8::new_default_platform(0, false).make_shared();
  v8::V8::initialize_platform(platform);
  v8::V8::initialize();

  let blob = {
    let external_references = v8::ExternalReferences::new(&[
      v8::ExternalReference {
        function: callback.map_fn_to(),
      },
      v8::ExternalReference { pointer: 1 as _ },
    ]);

    let mut isolate = v8::Isolate::snapshot_creator(
      Some(unsafe {
        std::mem::transmute::<
          &v8::ExternalReferences,
          &'static v8::ExternalReferences,
        >(&external_references)
      }),
      Some(v8::CreateParams::default()),
    );

    {
      let scope = &mut v8::HandleScope::new(&mut isolate);
      let context = v8::Context::new(scope, Default::default());
      scope.set_default_context(context);

      let scope = &mut v8::ContextScope::new(scope, context);
      let data = v8::External::new(scope, 1 as _);
      let ft = v8::FunctionTemplate::builder(callback)
        .data(data.into())
        .build(scope);
      let f = ft.get_function(scope).unwrap();

      let global = context.global(scope);
      let key = v8::String::new(scope, "f").unwrap();
      global.set(scope, key.into(), f.into()).unwrap();
    }

    isolate.create_blob(v8::FunctionCodeHandling::Keep).unwrap()
  };

  {
    let external_references = v8::ExternalReferences::new(&[
      v8::ExternalReference {
        function: callback.map_fn_to(),
      },
      v8::ExternalReference { pointer: 2 as _ },
    ]);

    let mut _isolate_a = v8::Isolate::new(
      v8::CreateParams::default()
        .snapshot_blob(blob.to_vec())
        .external_references(external_references),
    );

    let external_references = v8::ExternalReferences::new(&[
      v8::ExternalReference {
        function: callback.map_fn_to(),
      },
      v8::ExternalReference { pointer: 3 as _ },
    ]);

    let mut isolate_b = v8::Isolate::new(
      v8::CreateParams::default()
        .snapshot_blob(blob)
        .external_references(external_references),
    );

    {
      let scope = &mut v8::HandleScope::new(&mut isolate_b);
      let context = v8::Context::new(scope, Default::default());
      let scope = &mut v8::ContextScope::new(scope, context);

      let global = context.global(scope);
      let key = v8::String::new(scope, "f").unwrap();
      let f = global
        .get(scope, key.into())
        .unwrap()
        .cast::<v8::Function>();
      let null = v8::null(scope);
      let result = f.call(scope, null.into(), &[]);
      assert_eq!(result.unwrap().to_rust_string_lossy(scope), "3");
    }
  }

  unsafe {
    v8::V8::dispose();
  }
  v8::V8::dispose_platform();
}
