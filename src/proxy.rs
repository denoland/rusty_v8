use crate::Context;
use crate::Local;
use crate::Object;
use crate::Proxy;
use crate::ToLocal;
use crate::Value;

extern "C" {
  fn v8__Proxy__New(
    context: *mut Context,
    target: *mut Object,
    handler: *mut Object,
  ) -> *mut Proxy;
  fn v8__Proxy__GetHandler(proxy: *mut Proxy) -> *mut Value;
  fn v8__Proxy__GetTarget(proxy: *mut Proxy) -> *mut Value;
  fn v8__Proxy__IsRevoked(proxy: *mut Proxy) -> bool;
  fn v8__Proxy__Revoke(proxy: *mut Proxy);
}

impl Proxy {
  pub fn new<'sc>(
    scope: &mut impl ToLocal<'sc>,
    mut context: Local<Context>,
    mut target: Local<Object>,
    mut handler: Local<Object>,
  ) -> Option<Local<'sc, Proxy>> {
    unsafe {
      let ptr = v8__Proxy__New(&mut *context, &mut *target, &mut *handler);
      scope.to_local(ptr)
    }
  }

  pub fn get_handler<'sc>(
    &mut self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Local<'sc, Value> {
    unsafe { scope.to_local(v8__Proxy__GetHandler(&mut *self)) }.unwrap()
  }

  pub fn get_target<'sc>(
    &mut self,
    scope: &mut impl ToLocal<'sc>,
  ) -> Local<'sc, Value> {
    unsafe { scope.to_local(v8__Proxy__GetTarget(&mut *self)) }.unwrap()
  }

  pub fn is_revoked(&mut self) -> bool {
    unsafe { v8__Proxy__IsRevoked(self) }
  }

  pub fn revoke(&mut self) {
    unsafe { v8__Proxy__Revoke(self) };
  }
}
