use std::marker::PhantomData;
use std::mem::MaybeUninit;
use std::ptr::null;

use crate::support::Opaque;
use crate::Boolean;
use crate::Context;
use crate::Integer;
use crate::Local;
use crate::String;
use crate::ToLocal;
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

/// A compiled JavaScript script, tied to a Context which was active when the
/// script was compiled.
#[repr(C)]
pub struct Script(Opaque);

impl Script {
  /// A shorthand for ScriptCompiler::Compile().
  pub fn compile<'sc>(
    scope: &mut impl ToLocal<'sc>,
    mut context: Local<Context>,
    mut source: Local<String>,
    origin: Option<&ScriptOrigin>,
  ) -> Option<Local<'sc, Script>> {
    // TODO: use the type system to enforce that a Context has been entered.
    // TODO: `context` and `source` probably shouldn't be mut.
    let ptr = unsafe {
      v8__Script__Compile(
        &mut *context,
        &mut *source,
        origin.map(|r| r as *const _).unwrap_or(null()),
      )
    };
    unsafe { scope.to_local(ptr) }
  }

  /// Runs the script returning the resulting value. It will be run in the
  /// context in which it was created (ScriptCompiler::CompileBound or
  /// UnboundScript::BindToCurrentContext()).
  pub fn run<'sc>(
    &mut self,
    scope: &mut impl ToLocal<'sc>,
    mut context: Local<Context>,
  ) -> Option<Local<'sc, Value>> {
    unsafe { scope.to_local(v8__Script__Run(self, &mut *context)) }
  }
}

/// The origin, within a file, of a script.
impl<'sc> ScriptOrigin<'sc> {
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    resource_name: impl Into<Local<'sc, Value>>,
    resource_line_offset: impl Into<Local<'sc, Integer>>,
    resource_column_offset: impl Into<Local<'sc, Integer>>,
    resource_is_shared_cross_origin: impl Into<Local<'sc, Boolean>>,
    script_id: impl Into<Local<'sc, Integer>>,
    source_map_url: impl Into<Local<'sc, Value>>,
    resource_is_opaque: impl Into<Local<'sc, Boolean>>,
    is_wasm: impl Into<Local<'sc, Boolean>>,
    is_module: impl Into<Local<'sc, Boolean>>,
  ) -> Self {
    unsafe {
      let mut buf = std::mem::MaybeUninit::<ScriptOrigin>::uninit();
      v8__ScriptOrigin__CONSTRUCT(
        &mut buf,
        &*resource_name.into(),
        &*resource_line_offset.into(),
        &*resource_column_offset.into(),
        &*resource_is_shared_cross_origin.into(),
        &*script_id.into(),
        &*source_map_url.into(),
        &*resource_is_opaque.into(),
        &*is_wasm.into(),
        &*is_module.into(),
      );
      buf.assume_init()
    }
  }
}
