use crate::support::Opaque;
use crate::Context;
use crate::Local;

extern "C" {
  fn v8__Promise__Resolver__New(context: *mut Context) -> *mut PromiseResolver;
}

#[repr(C)]
pub struct PromiseResolver(Opaque);

impl PromiseResolver {
  /// Create a new resolver, along with an associated promise in pending state.
  pub fn new(
    mut context: Local<'_, Context>,
  ) -> Option<Local<'_, PromiseResolver>> {
    unsafe { Local::from_raw(v8__Promise__Resolver__New(&mut *context)) }
  }
}
