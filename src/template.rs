use crate::data::Data;
use crate::data::FunctionTemplate;
use crate::data::Name;
use crate::data::ObjectTemplate;
use crate::data::Template;
use crate::isolate::Isolate;
use crate::support::int;
use crate::support::MapFnTo;
use crate::AccessorNameGetterCallback;
use crate::AccessorNameSetterCallback;
use crate::CFunction;
use crate::ConstructorBehavior;
use crate::Context;
use crate::Function;
use crate::FunctionBuilder;
use crate::FunctionCallback;
use crate::HandleScope;
use crate::Local;
use crate::Object;
use crate::PropertyAttribute;
use crate::SideEffectType;
use crate::Signature;
use crate::String;
use crate::Value;
use crate::NONE;
use std::convert::TryFrom;
use std::ptr::null;

extern "C" {
  fn v8__Template__Set(
    this: *const Template,
    key: *const Name,
    value: *const Data,
    attr: PropertyAttribute,
  );
  fn v8__Signature__New(
    isolate: *mut Isolate,
    templ: *const FunctionTemplate,
  ) -> *const Signature;
  fn v8__FunctionTemplate__New(
    isolate: *mut Isolate,
    callback: FunctionCallback,
    data_or_null: *const Value,
    signature_or_null: *const Signature,
    length: i32,
    constructor_behavior: ConstructorBehavior,
    side_effect_type: SideEffectType,
    c_function_or_null: *const CFunction,
  ) -> *const FunctionTemplate;
  fn v8__FunctionTemplate__GetFunction(
    this: *const FunctionTemplate,
    context: *const Context,
  ) -> *const Function;
  fn v8__FunctionTemplate__PrototypeTemplate(
    this: *const FunctionTemplate,
  ) -> *const ObjectTemplate;
  fn v8__FunctionTemplate__SetClassName(
    this: *const FunctionTemplate,
    name: *const String,
  );
  fn v8__FunctionTemplate__Inherit(
    this: *const FunctionTemplate,
    parent: *const FunctionTemplate,
  );
  fn v8__FunctionTemplate__ReadOnlyPrototype(this: *const FunctionTemplate);
  fn v8__FunctionTemplate__RemovePrototype(this: *const FunctionTemplate);

  fn v8__ObjectTemplate__New(
    isolate: *mut Isolate,
    templ: *const FunctionTemplate,
  ) -> *const ObjectTemplate;
  fn v8__ObjectTemplate__NewInstance(
    this: *const ObjectTemplate,
    context: *const Context,
  ) -> *const Object;
  fn v8__ObjectTemplate__InternalFieldCount(this: *const ObjectTemplate)
    -> int;
  fn v8__ObjectTemplate__SetInternalFieldCount(
    this: *const ObjectTemplate,
    value: int,
  );
  fn v8__ObjectTemplate__SetAccessor(
    this: *const ObjectTemplate,
    key: *const Name,
    getter: AccessorNameGetterCallback,
  );
  fn v8__ObjectTemplate__SetAccessorWithSetter(
    this: *const ObjectTemplate,
    key: *const Name,
    getter: AccessorNameGetterCallback,
    setter: AccessorNameSetterCallback,
  );
}

impl Template {
  /// Adds a property to each instance created by this template.
  pub fn set(&self, key: Local<Name>, value: Local<Data>) {
    self.set_with_attr(key, value, NONE)
  }

  /// Adds a property to each instance created by this template with
  /// the specified property attributes.
  pub fn set_with_attr(
    &self,
    key: Local<Name>,
    value: Local<Data>,
    attr: PropertyAttribute,
  ) {
    unsafe { v8__Template__Set(self, &*key, &*value, attr) }
  }
}

impl<'s> FunctionBuilder<'s, FunctionTemplate> {
  /// Set the function call signature. The default is no signature.
  pub fn signature(mut self, signature: Local<'s, Signature>) -> Self {
    self.signature = Some(signature);
    self
  }

  /// Creates the function template.
  pub fn build(
    self,
    scope: &mut HandleScope<'s, ()>,
  ) -> Local<'s, FunctionTemplate> {
    unsafe {
      scope.cast_local(|sd| {
        v8__FunctionTemplate__New(
          sd.get_isolate_ptr(),
          self.callback,
          self.data.map_or_else(null, |p| &*p),
          self.signature.map_or_else(null, |p| &*p),
          self.length,
          self.constructor_behavior,
          self.side_effect_type,
          null(),
        )
      })
    }
    .unwrap()
  }
}

/// A Signature specifies which receiver is valid for a function.
///
/// A receiver matches a given signature if the receiver (or any of its
/// hidden prototypes) was created from the signature's FunctionTemplate, or
/// from a FunctionTemplate that inherits directly or indirectly from the
/// signature's FunctionTemplate.
impl Signature {
  pub fn new<'s>(
    scope: &mut HandleScope<'s, ()>,
    templ: Local<FunctionTemplate>,
  ) -> Local<'s, Self> {
    unsafe {
      scope.cast_local(|sd| v8__Signature__New(sd.get_isolate_ptr(), &*templ))
    }
    .unwrap()
  }
}

impl FunctionTemplate {
  /// Create a FunctionBuilder to configure a FunctionTemplate.
  /// This is the same as FunctionBuilder::<FunctionTemplate>::new().
  pub fn builder<'s>(
    callback: impl MapFnTo<FunctionCallback>,
  ) -> FunctionBuilder<'s, Self> {
    FunctionBuilder::new(callback)
  }

  /// Creates a function template.
  pub fn new<'s>(
    scope: &mut HandleScope<'s, ()>,
    callback: impl MapFnTo<FunctionCallback>,
  ) -> Local<'s, FunctionTemplate> {
    Self::builder(callback).build(scope)
  }

  /// Returns the unique function instance in the current execution context.
  pub fn get_function<'s>(
    &self,
    scope: &mut HandleScope<'s>,
  ) -> Option<Local<'s, Function>> {
    unsafe {
      scope.cast_local(|sd| {
        v8__FunctionTemplate__GetFunction(self, sd.get_current_context())
      })
    }
  }

  /// Set the class name of the FunctionTemplate. This is used for
  /// printing objects created with the function created from the
  /// FunctionTemplate as its constructor.
  pub fn set_class_name(&self, name: Local<String>) {
    unsafe { v8__FunctionTemplate__SetClassName(self, &*name) };
  }

  /// Returns the ObjectTemplate that is used by this
  /// FunctionTemplate as a PrototypeTemplate
  pub fn prototype_template<'s>(
    &self,
    scope: &mut HandleScope<'s, ()>,
  ) -> Local<'s, ObjectTemplate> {
    unsafe {
      scope.cast_local(|_sd| v8__FunctionTemplate__PrototypeTemplate(self))
    }
    .unwrap()
  }

  /// Causes the function template to inherit from a parent function template.
  /// This means the function's prototype.__proto__ is set to the parent function's prototype.
  pub fn inherit(&self, parent: Local<FunctionTemplate>) {
    unsafe { v8__FunctionTemplate__Inherit(self, &*parent) };
  }

  /// Sets the ReadOnly flag in the attributes of the 'prototype' property
  /// of functions created from this FunctionTemplate to true.
  pub fn read_only_prototype(&self) {
    unsafe { v8__FunctionTemplate__ReadOnlyPrototype(self) };
  }

  /// Removes the prototype property from functions created from this FunctionTemplate.
  pub fn remove_prototype(&self) {
    unsafe { v8__FunctionTemplate__RemovePrototype(self) };
  }
}

impl ObjectTemplate {
  /// Creates an object template.
  pub fn new<'s>(scope: &mut HandleScope<'s, ()>) -> Local<'s, ObjectTemplate> {
    unsafe {
      scope.cast_local(|sd| {
        v8__ObjectTemplate__New(sd.get_isolate_ptr(), std::ptr::null())
      })
    }
    .unwrap()
  }

  /// Creates an object template from a function template.
  pub fn new_from_template<'s>(
    scope: &mut HandleScope<'s, ()>,
    templ: Local<FunctionTemplate>,
  ) -> Local<'s, ObjectTemplate> {
    unsafe {
      scope
        .cast_local(|sd| v8__ObjectTemplate__New(sd.get_isolate_ptr(), &*templ))
    }
    .unwrap()
  }

  /// Creates a new instance of this object template.
  pub fn new_instance<'s>(
    &self,
    scope: &mut HandleScope<'s>,
  ) -> Option<Local<'s, Object>> {
    unsafe {
      scope.cast_local(|sd| {
        v8__ObjectTemplate__NewInstance(self, sd.get_current_context())
      })
    }
  }

  /// Gets the number of internal fields for objects generated from
  /// this template.
  pub fn internal_field_count(&self) -> usize {
    let count = unsafe { v8__ObjectTemplate__InternalFieldCount(self) };
    usize::try_from(count).expect("bad internal field count") // Can't happen.
  }

  /// Sets the number of internal fields for objects generated from
  /// this template.
  pub fn set_internal_field_count(&self, value: usize) -> bool {
    // The C++ API takes an i32 but trying to set a value < 0
    // results in unpredictable behavior, hence we disallow it.
    match int::try_from(value) {
      Err(_) => false,
      Ok(value) => {
        unsafe { v8__ObjectTemplate__SetInternalFieldCount(self, value) };
        true
      }
    }
  }

  pub fn set_accessor(
    &self,
    key: Local<Name>,
    getter: impl for<'s> MapFnTo<AccessorNameGetterCallback<'s>>,
  ) {
    unsafe { v8__ObjectTemplate__SetAccessor(self, &*key, getter.map_fn_to()) }
  }

  pub fn set_accessor_with_setter(
    &self,
    key: Local<Name>,
    getter: impl for<'s> MapFnTo<AccessorNameGetterCallback<'s>>,
    setter: impl for<'s> MapFnTo<AccessorNameSetterCallback<'s>>,
  ) {
    unsafe {
      v8__ObjectTemplate__SetAccessorWithSetter(
        self,
        &*key,
        getter.map_fn_to(),
        setter.map_fn_to(),
      )
    }
  }
}
