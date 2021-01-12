use crate::HandleScope;
use crate::Local;
use crate::Script;
use crate::UnboundScript;

extern "C" {
  fn v8__UnboundScript__BindToCurrentContext(
    script: *const UnboundScript,
  ) -> *const Script;
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
}
