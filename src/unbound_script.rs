use crate::CachedData;
use crate::Local;
use crate::Script;
use crate::UnboundScript;
use crate::{HandleScope, UniqueRef};

extern "C" {
  fn v8__UnboundScript__BindToCurrentContext(
    script: *const UnboundScript,
  ) -> *const Script;
  fn v8__UnboundScript__CreateCodeCache(
    script: *const UnboundScript,
  ) -> *mut CachedData<'static>;
}

impl UnboundScript {
  /// Binds the script to the currently entered context.
  pub fn bind_to_current_context<'s>(
    &self,
    scope: &mut HandleScope<'s>,
  ) -> Local<'s, Script> {
    unsafe {
      scope.cast_local(|_| v8__UnboundScript__BindToCurrentContext(self))
    }
    .unwrap()
  }

  /// Creates and returns code cache for the specified unbound_script.
  /// This will return nullptr if the script cannot be serialized. The
  /// CachedData returned by this function should be owned by the caller.
  pub fn create_code_cache(&self) -> Option<UniqueRef<CachedData<'static>>> {
    unsafe { UniqueRef::try_from_raw(v8__UnboundScript__CreateCodeCache(self)) }
  }
}
