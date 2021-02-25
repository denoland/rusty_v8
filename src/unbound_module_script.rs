use crate::CachedData;
use crate::UnboundModuleScript;
use crate::UniqueRef;

extern "C" {
  fn v8__UnboundModuleScript__CreateCodeCache(
    script: *const UnboundModuleScript,
  ) -> *mut CachedData<'static>;
}

impl UnboundModuleScript {
  /// Creates and returns code cache for the specified unbound_module_script.
  /// This will return nullptr if the script cannot be serialized. The
  /// CachedData returned by this function should be owned by the caller.
  pub fn create_code_cache(&self) -> Option<UniqueRef<CachedData<'static>>> {
    unsafe {
      UniqueRef::try_from_raw(v8__UnboundModuleScript__CreateCodeCache(self))
    }
  }
}
