use crate::isolate::CxxIsolate;
use crate::support::Opaque;
use crate::HandleScope;
use crate::Local;
use crate::Object;
use crate::ReturnValue;

extern "C" {
  fn v8__PropertyCallbackInfo__GetIsolate(
    info: &PropertyCallbackInfo,
  ) -> &mut CxxIsolate;
  fn v8__PropertyCallbackInfo__This(info: &PropertyCallbackInfo)
    -> *mut Object;
  fn v8__PropertyCallbackInfo__GetReturnValue(
    info: &PropertyCallbackInfo,
  ) -> *mut ReturnValue;
}

#[repr(C)]
pub struct PropertyCallbackInfo(Opaque);

impl PropertyCallbackInfo {
  pub fn get_return_value(&self) -> &mut ReturnValue {
    unsafe { &mut *v8__PropertyCallbackInfo__GetReturnValue(self) }
  }

  pub fn get_isolate(&self) -> &mut CxxIsolate {
    unsafe { v8__PropertyCallbackInfo__GetIsolate(self) }
  }

  pub fn this<'sc>(&self, _scope: &mut HandleScope<'sc>) -> Local<'sc, Object> {
    unsafe { Local::from_raw(v8__PropertyCallbackInfo__This(self)).unwrap() }
  }
}
