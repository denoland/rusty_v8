extern crate rusty_v8 as v8;

pub fn main() {
  let context: v8::Local<v8::Context> = mock();
  let scope: &mut v8::scope::Entered<'_, v8::HandleScope> = mock();

  let _leaked = {
    let mut try_catch = v8::TryCatch::new(scope);
    let tc = try_catch.enter();
    let exception = tc.exception().unwrap();
    let stack_trace = tc.stack_trace(scope, context).unwrap();
    let message = tc.message().unwrap();
    (exception, stack_trace, message)
  };
}

fn mock<T>() -> T {
  unimplemented!()
}
