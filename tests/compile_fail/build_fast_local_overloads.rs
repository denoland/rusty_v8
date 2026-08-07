// Copyright 2019-2026 the Deno authors. All rights reserved. MIT license.
//
// Regression test for denoland/rusty_v8#1989 (Orchid #160).
//
// Since crrev.com/c/7828135, V8 stores the raw `v8::CFunction` pointers
// passed to `NewWithCFunctionOverloads` directly inside `FunctionTemplateInfo`
// instead of copying them into a managed heap object. The pointed-to storage
// must therefore outlive the resulting `FunctionTemplate`, which in practice
// means it must be `'static`. `FunctionBuilder::build_fast` enforces this at
// the type level by requiring `&'static [v8::fast_api::CFunction]`, so a
// stack-local `Vec` of overloads must be rejected.

fn slow_fn(
  _: &mut v8::PinScope,
  _: v8::FunctionCallbackArguments,
  _: v8::ReturnValue<v8::Value>,
) {
}

fn rejects_local_overloads(scope: &mut v8::PinScope<'_, '_>) {
  let overloads: Vec<v8::fast_api::CFunction> = Vec::new();
  let _ = v8::FunctionTemplate::builder(slow_fn).build_fast(scope, &overloads);
}

pub fn main() {
  rejects_local_overloads(mock());
}

fn mock<T>() -> T {
  unimplemented!()
}
