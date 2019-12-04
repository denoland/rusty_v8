use std::ptr::null;

use crate::support::Opaque;
use crate::Context;
use crate::HandleScope;
use crate::Local;
use crate::String;
use crate::Value;

extern "C" {
  fn v8__Script__Compile(
    context: *mut Context,
    source: *mut String,
    origin: *const ScriptOrigin,
  ) -> *mut Script;
  fn v8__Script__Run(this: &mut Script, context: *mut Context) -> *mut Value;
}

#[repr(C)]
pub struct Script(Opaque);
#[repr(C)]
pub struct ScriptOrigin(Opaque);

impl Script {
  pub fn compile<'sc>(
    _scope: &mut HandleScope<'sc>,
    mut context: Local<'_, Context>,
    mut source: Local<'_, String>,
    origin: Option<&'_ ScriptOrigin>,
  ) -> Option<Local<'sc, Script>> {
    // TODO: use the type system to enforce that a Context has been entered.
    // TODO: `context` and `source` probably shouldn't be mut.
    unsafe {
      Local::from_raw(v8__Script__Compile(
        &mut *context,
        &mut *source,
        origin.map(|r| r as *const _).unwrap_or(null()),
      ))
    }
  }

  pub fn run<'sc>(
    &mut self,
    _scope: &mut HandleScope<'sc>,
    mut context: Local<'_, Context>,
  ) -> Option<Local<Value>> {
    unsafe { Local::from_raw(v8__Script__Run(self, &mut *context)) }
  }
}
