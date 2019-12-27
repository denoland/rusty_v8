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
  ///
  /// \code
  ///  void GetterCallback(Local<Name> name,
  ///                      const v8::PropertyCallbackInfo<v8::Value>& info) {
  ///     auto context = info.GetIsolate()->GetCurrentContext();
  ///
  ///     v8::Local<v8::Value> a_this =
  ///         info.This()
  ///             ->GetRealNamedProperty(context, v8_str("a"))
  ///             .ToLocalChecked();
  ///     v8::Local<v8::Value> a_holder =
  ///         info.Holder()
  ///             ->GetRealNamedProperty(context, v8_str("a"))
  ///             .ToLocalChecked();
  ///
  ///    CHECK(v8_str("r")->Equals(context, a_this).FromJust());
  ///    CHECK(v8_str("obj")->Equals(context, a_holder).FromJust());
  ///
  ///    info.GetReturnValue().Set(name);
  ///  }
  ///
  ///  v8::Local<v8::FunctionTemplate> templ =
  ///  v8::FunctionTemplate::New(isolate);
  ///  templ->InstanceTemplate()->SetHandler(
  ///      v8::NamedPropertyHandlerConfiguration(GetterCallback));
  ///  LocalContext env;
  ///  env->Global()
  ///      ->Set(env.local(), v8_str("obj"), templ->GetFunction(env.local())
  ///                                           .ToLocalChecked()
  ///                                           ->NewInstance(env.local())
  ///                                           .ToLocalChecked())
  ///      .FromJust();
  ///
  ///  CompileRun("obj.a = 'obj'; var r = {a: 'r'}; Reflect.get(obj, 'x', r)");
  /// \endcode
  pub fn this(&self) -> Local<Object> {
    unsafe { Local::from_raw(v8__PropertyCallbackInfo__This(self)).unwrap() }
  }
}
