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
use crate::Scope;
use crate::Value;

extern "C" {
  fn v8__Object__New(isolate: *mut Isolate) -> *mut Object;
  fn v8__Object__New__with_prototype_and_properties(
    isolate: *mut Isolate,
    prototype_or_null: Local<Value>,
    names: *mut Local<Name>,
    values: *mut Local<Value>,
    length: usize,
  ) -> *mut Object;
  fn v8__Object__SetAccessor(
    self_: &Object,
    context: Local<Context>,
    name: Local<Name>,
    getter: AccessorNameGetterCallback,
  ) -> MaybeBool;
  fn v8__Object__Get(
    object: &Object,
    context: Local<Context>,
    key: Local<Value>,
  ) -> *mut Value;
  fn v8__Object__Set(
    object: &Object,
    context: Local<Context>,
    key: Local<Value>,
    value: Local<Value>,
  ) -> MaybeBool;
  fn v8__Object__CreateDataProperty(
    object: &Object,
    context: Local<Context>,
    key: Local<Name>,
    value: Local<Value>,
  ) -> MaybeBool;
  fn v8__Object__DefineOwnProperty(
    object: &Object,
    context: Local<Context>,
    key: Local<Name>,
    value: Local<Value>,
    attr: PropertyAttribute,
  ) -> MaybeBool;
  fn v8__Object__GetIdentityHash(object: &Object) -> int;
  fn v8__Object__CreationContext(object: &Object) -> *mut Context;

  fn v8__Array__New(isolate: *mut Isolate, length: int) -> *mut Array;
}

impl Object {
  /// Creates an empty object.
  pub fn new<'s>(scope: &mut Scope) -> Local<'s, Object> {
    let ptr = unsafe { v8__Object__New(scope.isolate()) };
    unsafe { scope.to_local(ptr) }.unwrap()
  }

  /// Creates a JavaScript object with the given properties, and
  /// a the given prototype_or_null (which can be any JavaScript
  /// value, and if it's null, the newly created object won't have
  /// a prototype at all). This is similar to Object.create().
  /// All properties will be created as enumerable, configurable
  /// and writable properties.
  pub fn with_prototype_and_properties<'s>(
    scope: &mut Scope,
    prototype_or_null: Local<Value>,
    names: &[Local<Name>],
    values: &[Local<Value>],
  ) -> Local<'s, Object> {
    assert_eq!(names.len(), values.len());
    unsafe {
      let object = v8__Object__New__with_prototype_and_properties(
        scope.isolate(),
        prototype_or_null,
        names.as_ptr() as *mut Local<Name>,
        values.as_ptr() as *mut Local<Value>,
        names.len(),
      );
      scope.to_local(object).unwrap()
    }
  }

  /// Set only return Just(true) or Empty(), so if it should never fail, use
  /// result.Check().
  pub fn set(
    &self,
    context: Local<Context>,
    key: Local<Value>,
    value: Local<Value>,
  ) -> Option<bool> {
    unsafe { v8__Object__Set(self, context, key, value) }.into()
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
  ) -> Option<bool> {
    unsafe { v8__Object__CreateDataProperty(self, context, key, value) }.into()
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
  ) -> Option<bool> {
    unsafe { v8__Object__DefineOwnProperty(self, context, key, value, attr) }
      .into()
  }

  pub fn get<'a>(
    &self,
    scope: &'a mut Scope,
    context: Local<'a, Context>,
    key: Local<'a, Value>,
  ) -> Option<Local<'a, Value>> {
    unsafe {
      let ptr = v8__Object__Get(self, context, key);
      scope.to_local(ptr)
    }
  }

  /// Note: SideEffectType affects the getter only, not the setter.
  pub fn set_accessor(
    &mut self,
    context: Local<Context>,
    name: Local<Name>,
    getter: impl for<'s> MapFnTo<AccessorNameGetterCallback<'s>>,
  ) -> Option<bool> {
    unsafe { v8__Object__SetAccessor(self, context, name, getter.map_fn_to()) }
      .into()
  }

  /// Returns the identity hash for this object. The current implementation
  /// uses a hidden property on the object to store the identity hash.
  ///
  /// The return value will never be 0. Also, it is not guaranteed to be
  /// unique.
  pub fn get_identity_hash(&self) -> int {
    unsafe { v8__Object__GetIdentityHash(self) }
  }

  /// Returns the context in which the object was created.
  pub fn creation_context<'a>(
    &self,
    scope: &'a mut Scope,
  ) -> Local<'a, Context> {
    unsafe {
      let ptr = v8__Object__CreationContext(self);
      scope.to_local(ptr).unwrap()
    }
  }
}

impl Array {
  /// Creates a JavaScript array with the given length. If the length
  /// is negative the returned array will have length 0.
  pub fn new<'s>(scope: &mut Scope, length: i32) -> Local<'s, Array> {
    let ptr = unsafe { v8__Array__New(scope.isolate(), length) };
    unsafe { scope.to_local(ptr) }.unwrap()
  }
}
