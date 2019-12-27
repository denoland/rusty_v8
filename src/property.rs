use crate::isolate::Isolate;
use crate::support::Opaque;
use crate::Local;
use crate::Object;
use crate::ReturnValue;

use std::mem::MaybeUninit;

/// The information passed to a property callback about the context
/// of the property access.
#[repr(C)]
pub struct PropertyCallbackInfo(Opaque);

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

impl PropertyCallbackInfo {
  /// \return The return value of the callback.
  /// Can be changed by calling Set().
  /// \code
  /// info.GetReturnValue().Set(...)
  /// \endcode
  pub fn get_return_value(&self) -> ReturnValue {
    let mut rv = MaybeUninit::<ReturnValue>::uninit();
    unsafe {
      v8__PropertyCallbackInfo__GetReturnValue(self, rv.as_mut_ptr());
      rv.assume_init()
    }
  }

  /// The isolate of the property access.
  pub fn get_isolate(&mut self) -> &mut Isolate {
    unsafe { v8__PropertyCallbackInfo__GetIsolate(self) }
  }

  /// \return The receiver. In many cases, this is the object on which the
  /// property access was intercepted. When using
  /// `Reflect.get`, `Function.prototype.call`, or similar functions, it is the
  /// object passed in as receiver or thisArg.
  pub fn this(&self) -> Local<Object> {
    unsafe { Local::from_raw(v8__PropertyCallbackInfo__This(self)).unwrap() }
  }
}
