use crate::Local;
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
