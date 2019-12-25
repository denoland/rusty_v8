use crate::isolate::Isolate;
use crate::support::Opaque;
use crate::Local;
use crate::Object;
use crate::ReturnValue;

use std::mem::MaybeUninit;

extern "C" {
  fn v8__PropertyCallbackInfo__GetIsolate(
    info: &PropertyCallbackInfo,
  ) -> &mut Isolate;
  fn v8__PropertyCallbackInfo__This(info: &PropertyCallbackInfo)
    -> *mut Object;
  fn v8__PropertyCallbackInfo__GetReturnValue(
    info: &PropertyCallbackInfo,
    out: *mut ReturnValue,
  );
}

#[repr(C)]
pub struct PropertyCallbackInfo(Opaque);

impl PropertyCallbackInfo {
  pub fn get_return_value(&self) -> ReturnValue {
    let mut rv = MaybeUninit::<ReturnValue>::uninit();
    unsafe {
      v8__PropertyCallbackInfo__GetReturnValue(self, rv.as_mut_ptr());
      rv.assume_init()
    }
  }

  pub fn get_isolate(&mut self) -> &mut Isolate {
    unsafe { v8__PropertyCallbackInfo__GetIsolate(self) }
  }

  pub fn this(&self) -> Local<Object> {
    unsafe { Local::from_raw(v8__PropertyCallbackInfo__This(self)).unwrap() }
  }
}
