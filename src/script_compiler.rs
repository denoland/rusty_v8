// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.
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
  NoCacheNoReason = 0,
  NoCacheBecauseCachingDisabled,
  NoCacheBecauseNoResource,
  NoCacheBecauseInlineScript,
  NoCacheBecauseModule,
  NoCacheBecauseStreamingSource,
  NoCacheBecauseInspector,
  NoCacheBecauseScriptTooSmall,
  NoCacheBecauseCacheTooCold,
  NoCacheBecauseV8Extension,
  NoCacheBecauseExtensionModule,
  NoCacheBecausePacScript,
  NoCacheBecauseInDocumentWrite,
  NoCacheBecauseResourceWithNoCacheHandler,
  NoCacheBecauseDeferredProduceCodeCache,
}

/// Compile an ES module, returning a Module that encapsulates the compiled
/// code.
///
/// Corresponds to the ParseModule abstract operation in the ECMAScript
/// specification.
pub fn compile_module(
  isolate: &Isolate,
  source: Source,
  options: CompileOptions,
  no_cache_reason: NoCacheReason,
) -> Option<Local<Module>> {
  unsafe {
    Local::from_raw(v8__ScriptCompiler__CompileModule(
      isolate,
      &source,
      options,
      no_cache_reason,
    ))
  }
}
