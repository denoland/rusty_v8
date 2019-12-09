use std::ops::Deref;

use crate::isolate::CxxIsolate;
use crate::isolate::LockedIsolate;
use crate::support::Opaque;
use crate::HandleScope;
use crate::Local;
use crate::Name;
use crate::Value;

/// A JavaScript object (ECMA-262, 4.3.3)
#[repr(C)]
pub struct Object(Opaque);

extern "C" {
  fn v8__Object__New(isolate: *mut CxxIsolate, prototype_or_null: *mut Value, names: **mut Name, values: **mut Value, length: usize) -> *mut Object;


}

impl Object {
 /// Creates a JavaScript object with the given properties, and
 /// a the given prototype_or_null (which can be any JavaScript
 /// value, and if it's null, the newly created object won't have
 /// a prototype at all). This is similar to Object.create().
 /// All properties will be created as enumerable, configurable
 /// and writable properties.
  pub fn new<'sc>(
    scope: &mut HandleScope<'sc>,
    prototype_or_null: Local<'sc, Value>,
    names: Vec<Local<'sc, Name>>,
    values: Vec<Local<'sc, Value>>,
    length: usize,
  ) -> Local<'sc, Object> {
    unsafe {
      Local::from_raw(v8__Object__New(
        scope.cxx_isolate(),
        &mut *prototype_or_null,
        &mut **names,
        &mut **values,
        length
      ))
    }
  }
}


impl Deref for Object {
  type Target = Value;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Value) }
  }
}
