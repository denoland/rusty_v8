// Copyright 2019-2020 the Deno authors. All rights reserved. MIT license.
//! For compiling scripts.
use crate::Isolate;
use crate::Local;
use crate::Module;
use crate::ScriptOrigin;
use crate::String;
use std::mem::MaybeUninit;

extern "C" {
  fn v8__ScriptCompiler__Source__CONSTRUCT(
    buf: &mut MaybeUninit<Source>,
    source_string: &String,
    origin: &ScriptOrigin,
  );
  fn v8__ScriptCompiler__Source__DESTRUCT(this: &mut Source);

  fn v8__ScriptCompiler__CompileModule(
    isoate: &Isolate,
    source: &Source,
    options: CompileOptions,
    no_cache_reason: NoCacheReason,
  ) -> *mut Module;
}

#[repr(C)]
/// Source code which can be then compiled to a UnboundScript or Script.
pub struct Source([usize; 8]);

impl Source {
  // TODO(ry) cached_data
  pub fn new(source_string: Local<String>, origin: &ScriptOrigin) -> Self {
    let mut buf = MaybeUninit::<Self>::uninit();
    unsafe {
      v8__ScriptCompiler__Source__CONSTRUCT(&mut buf, &source_string, origin);
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
pub enum CompileOptions {
  NoCompileOptions = 0,
  ConsumeCodeCache,
  EagerCompile,
}

/// The reason for which we are not requesting or providing a code cache.
#[repr(C)]
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
pub fn compile_module<'a>(
  isolate: &Isolate,
  source: Source,
) -> Option<Local<'a, Module>> {
  compile_module2(
    isolate,
    source,
    CompileOptions::NoCompileOptions,
    NoCacheReason::NoReason,
  )
}

/// Same as compile_module with more options.
pub fn compile_module2<'a>(
  isolate: &Isolate,
  source: Source,
  options: CompileOptions,
  no_cache_reason: NoCacheReason,
) -> Option<Local<'a, Module>> {
  unsafe {
    Local::from_raw(v8__ScriptCompiler__CompileModule(
      isolate,
      &source,
      options,
      no_cache_reason,
    ))
  }
}
