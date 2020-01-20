use crate::isolate::Isolate;
use crate::support::int;
use crate::support::MapFnTo;
use crate::support::MaybeBool;
use crate::AccessorNameGetterCallback;
use crate::Array;
use crate::Context;
use crate::Local;
use crate::Name;
use crate::Object;
use crate::PropertyAttribute;
use crate::ToLocal;
use crate::Value;

extern "C" {
  fn v8__Object__New(isolate: *mut Isolate) -> *mut Object;
  fn v8__Object__New2(
    isolate: *mut Isolate,
    prototype_or_null: *mut Value,
    names: *mut *mut Name,
    values: *mut *mut Value,
    length: usize,
  ) -> *mut Object;
  fn v8__Object__SetAccessor(
    self_: &Object,
    context: *const Context,
    name: *const Name,
    getter: AccessorNameGetterCallback,
  ) -> MaybeBool;

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
  fn v8__Object__DefineOwnProperty(
    object: &Object,
    context: *const Context,
    key: *const Name,
    value: *const Value,
    attr: PropertyAttribute,
  ) -> MaybeBool;
  fn v8__Object__GetIdentityHash(object: &Object) -> int;

  fn v8__Array__New(isolate: *mut Isolate, length: int) -> *mut Array;
}

impl Object {
  /// Creates an empty object.
  pub fn new<'sc>(scope: &mut impl ToLocal<'sc>) -> Local<'sc, Object> {
    let ptr = unsafe { v8__Object__New(scope.isolate()) };
    unsafe { scope.to_local(ptr) }.unwrap()
  }

  /// Creates a JavaScript object with the given properties, and
  /// a the given prototype_or_null (which can be any JavaScript
  /// value, and if it's null, the newly created object won't have
  /// a prototype at all). This is similar to Object.create().
  /// All properties will be created as enumerable, configurable
  /// and writable properties.
  pub fn new2<'sc>(
    scope: &mut impl ToLocal<'sc>,
    mut prototype_or_null: Local<'sc, Value>,
    names: Vec<Local<'sc, Name>>,
    values: Vec<Local<'sc, Value>>,
  ) -> Local<'sc, Object> {
    let length = names.len();
    assert_eq!(length, values.len());
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
      v8__Object__New2(
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

  /// Implements DefineOwnProperty.
  ///
  /// In general, CreateDataProperty will be faster, however, does not allow
  /// for specifying attributes.
  ///
  /// Returns true on success.
  pub fn define_own_property(
    &self,
    context: Local<Context>,
    key: Local<Name>,
    value: Local<Value>,
    attr: PropertyAttribute,
  ) -> MaybeBool {
    unsafe {
      v8__Object__DefineOwnProperty(self, &*context, &*key, &*value, attr)
    }
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

  /// Note: SideEffectType affects the getter only, not the setter.
  pub fn set_accessor(
    &mut self,
    context: Local<Context>,
    name: Local<Name>,
    getter: impl for<'s> MapFnTo<AccessorNameGetterCallback<'s>>,
  ) -> MaybeBool {
    unsafe {
      v8__Object__SetAccessor(self, &*context, &*name, getter.map_fn_to())
    }
  }

  /// Returns the identity hash for this object. The current implementation
  /// uses a hidden property on the object to store the identity hash.
  ///
  /// The return value will never be 0. Also, it is not guaranteed to be
  /// unique.
  pub fn get_identity_hash(&self) -> int {
    unsafe { v8__Object__GetIdentityHash(self) }
  }
}

impl Array {
  /// Creates a JavaScript array with the given length. If the length
  /// is negative the returned array will have length 0.
  pub fn new<'sc>(
    scope: &mut impl ToLocal<'sc>,
    length: i32,
  ) -> Local<'sc, Array> {
    let ptr = unsafe { v8__Array__New(scope.isolate(), length) };
    unsafe { scope.to_local(ptr) }.unwrap()
  }
}
