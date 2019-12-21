// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.
use crate::support::Opaque;
use crate::Local;
use crate::Value;

extern "C" {
  fn v8__ScriptOrModule__GetResourceName(this: &ScriptOrModule) -> *mut Value;

  fn v8__ScriptOrModule__GetHostDefinedOptions(
    this: &ScriptOrModule,
  ) -> *mut Value;
}

/// A container type that holds relevant metadata for module loading.
///
/// This is passed back to the embedder as part of
/// HostImportModuleDynamicallyCallback for module loading.
#[repr(C)]
pub struct ScriptOrModule(Opaque);

impl ScriptOrModule {
  /// The name that was passed by the embedder as ResourceName to the
  /// ScriptOrigin. This can be either a v8::String or v8::Undefined.
  fn get_resource_name(&self) -> Local<'_, Value> {
    unsafe { Local::from_raw(v8__ScriptOrModule__GetResourceName(self)) }
  }

  /// The options that were passed by the embedder as HostDefinedOptions to the
  /// ScriptOrigin.
  fn get_host_defined_options(&self) -> Local<'_, PrimitiveArray> {
    unsafe { Local::from_raw(v8__ScriptOrModule__GetHostDefinedOptions(self)) }
  }
}
