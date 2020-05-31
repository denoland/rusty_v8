use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ptr::null;

use crate::Boolean;
use crate::Context;
use crate::Integer;
use crate::Local;
use crate::Script;
use crate::String;
use crate::ToLocal;
use crate::Value;

/// The origin, within a file, of a script.
#[repr(C)]
pub struct ScriptOrigin<'sc>([usize; 7], PhantomData<&'sc ()>);

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
  pub fn compile<'sc>(
    scope: &mut impl ToLocal<'sc>,
    context: Local<Context>,
    source: Local<String>,
    origin: Option<&ScriptOrigin>,
  ) -> Option<Local<'sc, Script>> {
    unsafe {
      scope.cast_local(|_| {
        v8__Script__Compile(
          &*context,
          &*source,
          origin.map(|r| r as *const _).unwrap_or(null()),
        )
      })
    }
  }

  /// Runs the script returning the resulting value. It will be run in the
  /// context in which it was created (ScriptCompiler::CompileBound or
  /// UnboundScript::BindToCurrentContext()).
  pub fn run<'sc>(
    &mut self,
    scope: &mut impl ToLocal<'sc>,
    context: Local<Context>,
  ) -> Option<Local<'sc, Value>> {
    unsafe { scope.cast_local(|_| v8__Script__Run(self, &*context)) }
  }
}

/// The origin, within a file, of a script.
impl<'sc> ScriptOrigin<'sc> {
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    resource_name: Local<'sc, Value>,
    resource_line_offset: Local<'sc, Integer>,
    resource_column_offset: Local<'sc, Integer>,
    resource_is_shared_cross_origin: Local<'sc, Boolean>,
    script_id: Local<'sc, Integer>,
    source_map_url: Local<'sc, Value>,
    resource_is_opaque: Local<'sc, Boolean>,
    is_wasm: Local<'sc, Boolean>,
    is_module: Local<'sc, Boolean>,
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
