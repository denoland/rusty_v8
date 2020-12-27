// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
use std::mem::MaybeUninit;

use crate::HandleScope;
use crate::Isolate;
use crate::Local;
use crate::Module;
use crate::ScriptOrigin;
use crate::String;

extern "C" {
  fn v8__ScriptCompiler__Source__CONSTRUCT(
    buf: *mut MaybeUninit<Source>,
    source_string: *const String,
    origin: *const ScriptOrigin,
  );
  fn v8__ScriptCompiler__Source__DESTRUCT(this: *mut Source);

  fn v8__ScriptCompiler__CompileModule(
    isolate: *mut Isolate,
    source: *mut Source,
    options: CompileOptions,
    no_cache_reason: NoCacheReason,
  ) -> *const Module;
}

/// Source code which can then be compiled to a UnboundScript or Script.
#[repr(C)]
#[derive(Debug)]
pub struct Source([usize; 8]);

impl Source {
  // TODO(ry) cached_data
  pub fn new(source_string: Local<String>, origin: &ScriptOrigin) -> Self {
    let mut buf = MaybeUninit::<Self>::uninit();
    unsafe {
      v8__ScriptCompiler__Source__CONSTRUCT(&mut buf, &*source_string, origin);
      buf.assume_init()
    }
  }
}

impl Drop for Source {
  fn drop(&mut self) {
    unsafe { v8__ScriptCompiler__Source__DESTRUCT(self) }
  }
}

#[repr(C)]
#[derive(Debug)]
pub enum CompileOptions {
  NoCompileOptions = 0,
  ConsumeCodeCache,
  EagerCompile,
}

/// The reason for which we are not requesting or providing a code cache.
#[repr(C)]
#[derive(Debug)]
pub enum NoCacheReason {
  NoReason = 0,
  BecauseCachingDisabled,
  BecauseNoResource,
  BecauseInlineScript,
  BecauseModule,
  BecauseStreamingSource,
  BecauseInspector,
  BecauseScriptTooSmall,
  BecauseCacheTooCold,
  BecauseV8Extension,
  BecauseExtensionModule,
  BecausePacScript,
  BecauseInDocumentWrite,
  BecauseResourceWithNoCacheHandler,
  BecauseDeferredProduceCodeCache,
}

/// Compile an ES module, returning a Module that encapsulates the compiled
/// code.
///
/// Corresponds to the ParseModule abstract operation in the ECMAScript
/// specification.
pub fn compile_module<'s>(
  scope: &mut HandleScope<'s>,
  source: Source,
) -> Option<Local<'s, Module>> {
  compile_module2(
    scope,
    source,
    CompileOptions::NoCompileOptions,
    NoCacheReason::NoReason,
  )
}

/// Same as compile_module with more options.
pub fn compile_module2<'s>(
  scope: &mut HandleScope<'s>,
  mut source: Source,
  options: CompileOptions,
  no_cache_reason: NoCacheReason,
) -> Option<Local<'s, Module>> {
  unsafe {
    scope.cast_local(|sd| {
      v8__ScriptCompiler__CompileModule(
        sd.get_isolate_ptr(),
        &mut source,
        options,
        no_cache_reason,
      )
    })
  }
}
