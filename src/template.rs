use crate::data::Data;
use crate::data::FunctionTemplate;
use crate::data::Name;
use crate::data::ObjectTemplate;
use crate::data::Template;
use crate::isolate::Isolate;
use crate::support::MapFnTo;
use crate::Context;
use crate::Function;
use crate::FunctionCallback;
use crate::Local;
use crate::Object;
use crate::PropertyAttribute;
use crate::Scope;
use crate::String;
use crate::NONE;

extern "C" {
  fn v8__Template__Set(
    self_: &Template,
    key: *const Name,
    value: *const Data,
    attr: PropertyAttribute,
  );

  fn v8__FunctionTemplate__New(
    isolate: &Isolate,
    callback: FunctionCallback,
  ) -> *mut FunctionTemplate;
  fn v8__FunctionTemplate__GetFunction(
    fn_template: *mut FunctionTemplate,
    context: *mut Context,
  ) -> *mut Function;
  fn v8__FunctionTemplate__SetClassName(
    fn_template: *mut FunctionTemplate,
    name: Local<String>,
  ) -> *mut Function;

  fn v8__ObjectTemplate__New(
    isolate: *mut Isolate,
    templ: *const FunctionTemplate,
  ) -> *mut ObjectTemplate;
  fn v8__ObjectTemplate__NewInstance(
    self_: &ObjectTemplate,
    context: *mut Context,
  ) -> *mut Object;
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

impl FunctionTemplate {
  /// Creates a function template.
  pub fn new<'sc>(
    scope: &'sc mut Scope,
    callback: impl MapFnTo<FunctionCallback>,
  ) -> Local<FunctionTemplate> {
    let ptr = unsafe {
      v8__FunctionTemplate__New(scope.isolate(), callback.map_fn_to())
    };
    unsafe { scope.to_local(ptr) }.unwrap()
  }

  /// Returns the unique function instance in the current execution context.
  pub fn get_function<'sc>(
    &mut self,
    scope: &'sc mut Scope,
    mut context: Local<Context>,
  ) -> Option<Local<Function>> {
    unsafe {
      scope
        .to_local(v8__FunctionTemplate__GetFunction(&mut *self, &mut *context))
    }
  }

  /// Set the class name of the FunctionTemplate. This is used for
  /// printing objects created with the function created from the
  /// FunctionTemplate as its constructor.
  pub fn set_class_name(&mut self, name: Local<String>) {
    unsafe { v8__FunctionTemplate__SetClassName(&mut *self, name) };
  }
}

impl ObjectTemplate {
  /// Creates an object template.
  pub fn new<'sc>(scope: &'sc mut Scope) -> Local<ObjectTemplate> {
    let ptr =
      unsafe { v8__ObjectTemplate__New(scope.isolate(), std::ptr::null()) };
    unsafe { scope.to_local(ptr) }.unwrap()
  }

  /// Creates an object template from a function template.
  pub fn new_from_template<'sc>(
    scope: &'sc mut Scope,
    templ: Local<FunctionTemplate>,
  ) -> Local<ObjectTemplate> {
    let ptr = unsafe { v8__ObjectTemplate__New(scope.isolate(), &*templ) };
    unsafe { scope.to_local(ptr) }.unwrap()
  }

  /// Creates a new instance of this object template.
  pub fn new_instance<'a>(
    &self,
    scope: &'a mut Scope,
    mut context: Local<Context>,
  ) -> Option<Local<Object>> {
    let ptr = unsafe { v8__ObjectTemplate__NewInstance(self, &mut *context) };
    unsafe { scope.to_local(ptr) }
  }
}
