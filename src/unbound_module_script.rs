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
  #[inline(always)]
  pub fn create_code_cache(&self) -> Option<UniqueRef<CachedData<'static>>> {
    let code_cache = unsafe {
      UniqueRef::try_from_raw(v8__UnboundModuleScript__CreateCodeCache(self))
    };
    if let Some(code_cache) = &code_cache {
      debug_assert_eq!(
        code_cache.buffer_policy(),
        crate::script_compiler::BufferPolicy::BufferOwned
      );
    }
    code_cache
  }
}
