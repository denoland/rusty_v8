use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ptr::null;

use crate::Boolean;
use crate::Context;
use crate::HandleScope;
use crate::Integer;
use crate::Local;
use crate::Script;
use crate::String;
use crate::Value;

/// The origin, within a file, of a script.
#[repr(C)]
#[derive(Debug)]
pub struct ScriptOrigin<'s>([usize; 7], PhantomData<&'s ()>);

extern "C" {
  fn v8__Script__Compile(
    context: *const Context,
    source: *const String,
    origin: *const ScriptOrigin,
  ) -> *const Script;
  fn v8__Script__Run(
    script: *const Script,
    context: *const Context,
  ) -> *const Value;

  fn v8__ScriptOrigin__CONSTRUCT(
    buf: *mut MaybeUninit<ScriptOrigin>,
    resource_name: *const Value,
    resource_line_offset: *const Integer,
    resource_column_offset: *const Integer,
    resource_is_shared_cross_origin: *const Boolean,
    script_id: *const Integer,
    source_map_url: *const Value,
    resource_is_opaque: *const Boolean,
    is_wasm: *const Boolean,
    is_module: *const Boolean,
  );
}

impl Script {
  /// A shorthand for ScriptCompiler::Compile().
  pub fn compile<'s>(
    scope: &mut HandleScope<'s>,
    source: Local<String>,
    origin: Option<&ScriptOrigin>,
  ) -> Option<Local<'s, Script>> {
    unsafe {
      scope.cast_local(|sd| {
        v8__Script__Compile(
          sd.get_current_context(),
          &*source,
          origin.map(|r| r as *const _).unwrap_or_else(null),
        )
      })
    }
  }

  /// Runs the script returning the resulting value. It will be run in the
  /// context in which it was created (ScriptCompiler::CompileBound or
  /// UnboundScript::BindToCurrentContext()).
  pub fn run<'s>(
    &self,
    scope: &mut HandleScope<'s>,
  ) -> Option<Local<'s, Value>> {
    unsafe {
      scope.cast_local(|sd| v8__Script__Run(self, sd.get_current_context()))
    }
  }
}

/// The origin, within a file, of a script.
impl<'s> ScriptOrigin<'s> {
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    resource_name: Local<'s, Value>,
    resource_line_offset: Local<'s, Integer>,
    resource_column_offset: Local<'s, Integer>,
    resource_is_shared_cross_origin: Local<'s, Boolean>,
    script_id: Local<'s, Integer>,
    source_map_url: Local<'s, Value>,
    resource_is_opaque: Local<'s, Boolean>,
    is_wasm: Local<'s, Boolean>,
    is_module: Local<'s, Boolean>,
  ) -> Self {
    unsafe {
      let mut buf = std::mem::MaybeUninit::<ScriptOrigin>::uninit();
      v8__ScriptOrigin__CONSTRUCT(
        &mut buf,
        &*resource_name,
        &*resource_line_offset,
        &*resource_column_offset,
        &*resource_is_shared_cross_origin,
        &*script_id,
        &*source_map_url,
        &*resource_is_opaque,
        &*is_wasm,
        &*is_module,
      );
      buf.assume_init()
    }
  }
}
