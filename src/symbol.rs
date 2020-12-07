use crate::HandleScope;
use crate::Isolate;
use crate::Local;
use crate::String;
use crate::Symbol;
use crate::Value;

extern "C" {
  fn v8__Symbol__New(
    isolate: *mut Isolate,
    description: *const String,
  ) -> *const Symbol;
  fn v8__Symbol__ForApi(
    isolate: *mut Isolate,
    description: *const String,
  ) -> *const Symbol;
  fn v8__Symbol__Description(this: *const Symbol) -> *const Value;
}

macro_rules! well_known {
  ($name:ident, $binding:ident) => {
    pub fn $name<'s>(scope: &mut HandleScope<'s, ()>) -> Local<'s, Symbol> {
      extern "C" {
        fn $binding(isolate: *mut Isolate) -> *const Symbol;
      }
      unsafe { scope.cast_local(|sd| $binding(sd.get_isolate_ptr())) }.unwrap()
    }
  };
}

impl Symbol {
  /// Create a symbol. If description is not empty, it will be used as the
  /// description.
  pub fn new<'s>(
    scope: &mut HandleScope<'s, ()>,
    description: Option<Local<String>>,
  ) -> Local<'s, Symbol> {
    unsafe {
      scope.cast_local(|sd| {
        v8__Symbol__New(
          sd.get_isolate_ptr(),
          description.map_or_else(std::ptr::null, |v| &*v),
        )
      })
    }
    .unwrap()
  }

  /// Access global symbol registry.
  /// Note that symbols created this way are never collected, so
  /// they should only be used for statically fixed properties.
  /// Also, there is only one global description space for the descriptions used as
  /// keys.
  /// To minimize the potential for clashes, use qualified descriptions as keys.
  /// Corresponds to v8::Symbol::For() in C++.
  pub fn for_global<'s>(
    scope: &mut HandleScope<'s, ()>,
    description: Local<String>,
  ) -> Local<'s, Symbol> {
    unsafe {
      scope.cast_local(|sd| {
        v8__Symbol__ForApi(sd.get_isolate_ptr(), &*description)
      })
    }
    .unwrap()
  }

  /// Returns the description string of the symbol, or undefined if none.
  pub fn description<'s>(
    &self,
    scope: &mut HandleScope<'s, ()>,
  ) -> Local<'s, Value> {
    unsafe { scope.cast_local(|_| v8__Symbol__Description(&*self)) }.unwrap()
  }

  well_known!(get_async_iterator, v8__Symbol__GetAsyncIterator);
  well_known!(get_has_instance, v8__Symbol__GetHasInstance);
  well_known!(get_is_concat_spreadable, v8__Symbol__GetIsConcatSpreadable);
  well_known!(get_iterator, v8__Symbol__GetIterator);
  well_known!(get_match, v8__Symbol__GetMatch);
  well_known!(get_replace, v8__Symbol__GetReplace);
  well_known!(get_search, v8__Symbol__GetSearch);
  well_known!(get_split, v8__Symbol__GetSplit);
  well_known!(get_to_primitive, v8__Symbol__GetToPrimitive);
  well_known!(get_to_string_tag, v8__Symbol__GetToStringTag);
  well_known!(get_unscopables, v8__Symbol__GetUnscopables);
}
