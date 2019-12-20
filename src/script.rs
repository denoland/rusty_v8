use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ptr::null;

use crate::support::Opaque;
use crate::Boolean;
use crate::Context;
use crate::HandleScope;
use crate::Integer;
use crate::Local;
use crate::String;
use crate::Value;

/// The origin, within a file, of a script.
#[repr(C)]
pub struct ScriptOrigin<'sc>([usize; 7], PhantomData<&'sc ()>);

extern "C" {
  fn v8__Script__Compile(
    context: *mut Context,
    source: *mut String,
    origin: *const ScriptOrigin,
  ) -> *mut Script;
  fn v8__Script__Run(this: &mut Script, context: *mut Context) -> *mut Value;

  fn v8__ScriptOrigin__CONSTRUCT(
    buf: &mut MaybeUninit<ScriptOrigin>,
    resource_name: *mut Value,
    resource_line_offset: *mut Integer,
    resource_column_offset: *mut Integer,
    resource_is_shared_cross_origin: *mut Boolean,
    script_id: *mut Integer,
    source_map_url: *mut Value,
    resource_is_opaque: *mut Boolean,
    is_wasm: *mut Boolean,
    is_module: *mut Boolean,
  );
}

/// A compiled JavaScript script, tied to a Context which was active when the
/// script was compiled.
#[repr(C)]
pub struct Script(Opaque);

impl Script {
  /// A shorthand for ScriptCompiler::Compile().
  pub fn compile<'sc>(
    _scope: &mut HandleScope<'sc>,
    mut context: Local<Context>,
    mut source: Local<String>,
    origin: Option<&ScriptOrigin>,
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

  /// Runs the script returning the resulting value. It will be run in the
  /// context in which it was created (ScriptCompiler::CompileBound or
  /// UnboundScript::BindToCurrentContext()).
  pub fn run<'sc>(
    &mut self,
    _scope: &mut HandleScope<'sc>,
    mut context: Local<Context>,
  ) -> Option<Local<'sc, Value>> {
    unsafe { Local::from_raw(v8__Script__Run(self, &mut *context)) }
  }
}

/// The origin, within a file, of a script.
impl<'sc> ScriptOrigin<'sc> {
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    mut resource_name: Local<'sc, Value>,
    mut resource_line_offset: Local<'sc, Integer>,
    mut resource_column_offset: Local<'sc, Integer>,
    mut resource_is_shared_cross_origin: Local<'sc, Boolean>,
    mut script_id: Local<'sc, Integer>,
    mut source_map_url: Local<'sc, Value>,
    mut resource_is_opaque: Local<'sc, Boolean>,
    mut is_wasm: Local<'sc, Boolean>,
    mut is_module: Local<'sc, Boolean>,
  ) -> Self {
    unsafe {
      let mut buf = std::mem::MaybeUninit::<ScriptOrigin>::uninit();
      v8__ScriptOrigin__CONSTRUCT(
        &mut buf,
        &mut *resource_name,
        &mut *resource_line_offset,
        &mut *resource_column_offset,
        &mut *resource_is_shared_cross_origin,
        &mut *script_id,
        &mut *source_map_url,
        &mut *resource_is_opaque,
        &mut *is_wasm,
        &mut *is_module,
      );
      buf.assume_init()
    }
  }
}
