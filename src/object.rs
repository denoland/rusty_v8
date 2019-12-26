use std::ops::Deref;

use crate::isolate::Isolate;
use crate::support::MaybeBool;
use crate::support::Opaque;
use crate::Context;
use crate::Local;
use crate::Name;
use crate::ToLocal;
use crate::Value;

/// A JavaScript object (ECMA-262, 4.3.3)
#[repr(C)]
pub struct Object(Opaque);

extern "C" {
  fn v8__Object__New(
    isolate: *mut Isolate,
    prototype_or_null: *mut Value,
    names: *mut *mut Name,
    values: *mut *mut Value,
    length: usize,
  ) -> *mut Object;
  fn v8__Object__GetIsolate(object: &Object) -> &mut Isolate;

  fn v8__Object__Get(
    object: &Object,
    context: *const Context,
    key: *const Value,
  ) -> *mut Value;
  fn v8__Object__Set(
    object: &Object,
    context: *const Context,
    key: *const Value,
    value: *const Value,
  ) -> MaybeBool;
  fn v8__Object__CreateDataProperty(
    object: &Object,
    context: *const Context,
    key: *const Name,
    value: *const Value,
  ) -> MaybeBool;
}

impl Object {
  /// Creates a JavaScript object with the given properties, and
  /// a the given prototype_or_null (which can be any JavaScript
  /// value, and if it's null, the newly created object won't have
  /// a prototype at all). This is similar to Object.create().
  /// All properties will be created as enumerable, configurable
  /// and writable properties.
  pub fn new<'sc>(
    scope: &mut impl ToLocal<'sc>,
    mut prototype_or_null: Local<'sc, Value>,
    names: Vec<Local<'sc, Name>>,
    values: Vec<Local<'sc, Value>>,
    length: usize,
  ) -> Local<'sc, Object> {
    let mut names_: Vec<*mut Name> = vec![];
    for mut name in names {
      let n = &mut *name;
      names_.push(n);
    }

    let mut values_: Vec<*mut Value> = vec![];
    for mut value in values {
      let n = &mut *value;
      values_.push(n);
    }
    let ptr = unsafe {
      v8__Object__New(
        scope.isolate(),
        &mut *prototype_or_null,
        names_.as_mut_ptr(),
        values_.as_mut_ptr(),
        length,
      )
    };
    unsafe { scope.to_local(ptr) }.unwrap()
  }

  /// Set only return Just(true) or Empty(), so if it should never fail, use
  /// result.Check().
  pub fn set(
    &self,
    context: Local<Context>,
    key: Local<Value>,
    value: Local<Value>,
  ) -> MaybeBool {
    unsafe { v8__Object__Set(self, &*context, &*key, &*value) }
  }

  /// Implements CreateDataProperty (ECMA-262, 7.3.4).
  ///
  /// Defines a configurable, writable, enumerable property with the given value
  /// on the object unless the property already exists and is not configurable
  /// or the object is not extensible.
  ///
  /// Returns true on success.
  pub fn create_data_property(
    &self,
    context: Local<Context>,
    key: Local<Name>,
    value: Local<Value>,
  ) -> MaybeBool {
    unsafe { v8__Object__CreateDataProperty(self, &*context, &*key, &*value) }
  }

  pub fn get<'a>(
    &self,
    scope: &mut impl ToLocal<'a>,
    context: Local<Context>,
    key: Local<Value>,
  ) -> Option<Local<'a, Value>> {
    unsafe {
      let ptr = v8__Object__Get(self, &*context, &*key);
      scope.to_local(ptr)
    }
  }

  /// Return the isolate to which the Object belongs to.
  pub fn get_isolate(&mut self) -> &Isolate {
    unsafe { v8__Object__GetIsolate(self) }
  }
}

impl Deref for Object {
  type Target = Value;
  fn deref(&self) -> &Self::Target {
    unsafe { &*(self as *const _ as *const Value) }
  }
}
