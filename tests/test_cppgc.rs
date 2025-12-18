// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.

use std::sync::atomic::{AtomicUsize, Ordering};
use v8::cppgc::{GarbageCollected, GcCell, Member, Traced, Visitor};

mod setup {
  use std::sync::Once;
  use std::sync::RwLock;
  // use std::sync::RwLockReadGuard;
  use std::sync::RwLockWriteGuard;

  static PROCESS_LOCK: RwLock<()> = RwLock::new(());

  /*
  /// Set up global state for a test that can run in parallel with other tests.
  pub(super) fn parallel_test() -> SetupGuard<RwLockReadGuard<'static, ()>> {
    initialize_once();
    SetupGuard::new(PROCESS_LOCK.read().unwrap())
  }
  */

  /// Set up global state for a test that must be the only test running.
  pub(super) fn sequential_test() -> SetupGuard<RwLockWriteGuard<'static, ()>> {
    initialize_once();
    SetupGuard::new(PROCESS_LOCK.write().unwrap())
  }

  fn initialize_once() {
    static START: Once = Once::new();
    START.call_once(|| {
      assert!(v8::icu::set_common_data_77(align_data::include_aligned!(
        align_data::Align16,
        "../third_party/icu/common/icudtl.dat"
      ))
      .is_ok());
      v8::V8::set_flags_from_string(
        "--no_freeze_flags_after_init --expose_gc --harmony-import-assertions --harmony-shadow-realm --allow_natives_syntax --turbo_fast_api_calls",
      );

      let platform = v8::new_unprotected_default_platform(0, false).make_shared();
      v8::V8::initialize_platform(platform.clone());
      v8::V8::initialize();
    });
  }

  #[must_use]
  pub(super) struct SetupGuard<G> {
    _inner: G,
  }

  impl<G> SetupGuard<G> {
    fn new(inner: G) -> Self {
      Self { _inner: inner }
    }
  }
}

const TAG: u16 = 1;

macro_rules! test {
  ( $( $decln:ident : $declt:ty )?, $( $initn:ident : $inite:expr )? ) => {{
      let _guard = setup::sequential_test();

      static TRACE_COUNT: AtomicUsize = AtomicUsize::new(0);
      static DROP_COUNT: AtomicUsize = AtomicUsize::new(0);

      struct Wrap {
        $( #[allow(unused)] $decln: $declt , )?
        value: v8::TracedReference<v8::Value>,
      }

      unsafe impl GarbageCollected for Wrap {
        fn trace(&self, visitor: &mut Visitor) {
          TRACE_COUNT.fetch_add(1, Ordering::SeqCst);
          visitor.trace(&self.value);
        }

        fn get_name(&self) -> &'static std::ffi::CStr {
          c"Eyecatcher"
        }
      }

      impl Drop for Wrap {
        fn drop(&mut self) {
          DROP_COUNT.fetch_add(1, Ordering::SeqCst);
        }
      }

      fn op_wrap(
        scope: &mut v8::PinScope<'_, '_>,
        args: v8::FunctionCallbackArguments,
        mut rv: v8::ReturnValue<v8::Value>,
      ) {
        fn empty(
          _scope: &mut v8::PinScope<'_, '_>,
          _args: v8::FunctionCallbackArguments,
          _rv: v8::ReturnValue<v8::Value>,
        ) {
        }
        let templ = v8::FunctionTemplate::new(scope, empty);
        let func = templ.get_function(scope).unwrap();
        let obj = func.new_instance(scope, &[]).unwrap();
        assert!(obj.is_api_wrapper());

        let wrap = Wrap {
          $( $initn: $inite , )?
          value: v8::TracedReference::new(scope, args.get(0)),
        };
        let member = unsafe {
          v8::cppgc::make_garbage_collected(scope.get_cpp_heap().unwrap(), wrap)
        };

        unsafe {
          v8::Object::wrap::<TAG, Wrap>(scope, obj, &member);
        }

        rv.set(obj.into());
      }

      fn op_unwrap(
        scope: &mut v8::PinScope<'_, '_>,
        args: v8::FunctionCallbackArguments,
        mut rv: v8::ReturnValue,
      ) {
        let obj = args.get(0).try_into().unwrap();
        let member = unsafe { v8::Object::unwrap::<TAG, Wrap>(scope, obj) };
        rv.set(unsafe { member.unwrap().as_ref().value.get(scope).unwrap() });
      }

      {
        let isolate = &mut v8::Isolate::new(v8::CreateParams::default());

        {
          let handle_scope = std::pin::pin!(v8::HandleScope::new(isolate));
          let handle_scope = &mut handle_scope.init();
          let context = v8::Context::new(handle_scope, Default::default());
          let scope = &mut v8::ContextScope::new(handle_scope, context);
          let global = context.global(scope);
          {
            let func = v8::Function::new(scope, op_wrap).unwrap();
            let name = v8::String::new(scope, "wrap").unwrap();
            global.set(scope, name.into(), func.into()).unwrap();
          }
          {
            let func = v8::Function::new(scope, op_unwrap).unwrap();
            let name = v8::String::new(scope, "unwrap").unwrap();
            global.set(scope, name.into(), func.into()).unwrap();
          }

          execute_script(
            scope,
            r#"
          {
            const x = {};
            const y = unwrap(wrap(x)); // collected
            if (x !== y) {
              throw new Error('mismatch');
            }
          }

          globalThis.wrapped = wrap(wrap({})); // not collected
        "#,
          );

          assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 0);

          {
            let mut vec = Vec::<u8>::new();
            scope.take_heap_snapshot(|chunk| {
              vec.extend_from_slice(chunk);
              true
            });
            let s = std::str::from_utf8(&vec).unwrap();
            assert!(s.contains("Eyecatcher"));
          }

          scope.request_garbage_collection_for_testing(
            v8::GarbageCollectionType::Full,
          );

          assert!(TRACE_COUNT.load(Ordering::SeqCst) > 0);
          assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 1);

          execute_script(
            scope,
            r#"
          globalThis.wrapped = undefined;
        "#,
          );

          scope.request_garbage_collection_for_testing(
            v8::GarbageCollectionType::Full,
          );

          assert_eq!(DROP_COUNT.load(Ordering::SeqCst), 3);
        }
      }
  }}
}

#[test]
fn cppgc_object_wrap8() {
  test!(,);
}

#[test]
fn cppgc_object_wrap16() {
  test!(big: u128, big: 0);
}

fn execute_script(
  context_scope: &mut v8::ContextScope<v8::HandleScope>,
  source: &str,
) {
  v8::scope!(let scope, context_scope);

  v8::tc_scope!(let scope, scope);

  let source = v8::String::new(scope, source).unwrap();

  let script =
    v8::Script::compile(scope, source, None).expect("failed to compile script");

  if script.run(scope).is_none() {
    let exception_string = scope
      .stack_trace()
      .or_else(|| scope.exception())
      .map_or_else(
        || "no stack trace".into(),
        |value| value.to_rust_string_lossy(scope),
      );

    panic!("{exception_string}");
  }
}

#[test]
fn cppgc_cell() {
  struct Wrap {
    int: GcCell<i32>,
    inner: GcCell<Inner>,
  }

  struct Inner {
    other: Member<Wrap>,
  }

  unsafe impl GarbageCollected for Wrap {
    fn trace(&self, visitor: &mut Visitor) {
      visitor.trace(&self.inner);
    }

    fn get_name(&self) -> &'static std::ffi::CStr {
      c"GcCellWrap"
    }
  }

  impl Traced for Inner {
    fn trace(&self, visitor: &mut Visitor) {
      visitor.trace(&self.other);
    }
  }

  fn op_wrap(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue<v8::Value>,
  ) {
    fn empty(
      _scope: &mut v8::PinScope<'_, '_>,
      _args: v8::FunctionCallbackArguments,
      _rv: v8::ReturnValue<v8::Value>,
    ) {
    }
    let templ = v8::FunctionTemplate::new(scope, empty);
    let func = templ.get_function(scope).unwrap();
    let obj = func.new_instance(scope, &[]).unwrap();
    assert!(obj.is_api_wrapper());

    let int = v8::Local::<v8::Integer>::try_from(args.get(0))
      .expect("expected integer");
    let other =
      v8::Local::<v8::Object>::try_from(args.get(1))
        .ok()
        .map(|obj| unsafe {
          v8::Object::unwrap::<TAG, _>(scope, obj)
            .expect("expected wrapped object")
        });

    let wrap = Wrap {
      int: GcCell::new(int.value() as i32),
      inner: GcCell::new(Inner {
        other: Member::empty(),
      }),
    };
    let wrapped = unsafe {
      v8::cppgc::make_garbage_collected(scope.get_cpp_heap().unwrap(), wrap)
    };
    if let Some(other) = other {
      // Initialize the member with the other object.
      unsafe { wrapped.as_ref() }
        .inner
        .get_mut(scope)
        .other
        .set(&other);
    }

    unsafe {
      v8::Object::wrap::<TAG, Wrap>(scope, obj, &wrapped);
    }

    rv.set(obj.into());
  }

  fn op_unwrap(
    scope: &mut v8::PinScope<'_, '_>,
    args: v8::FunctionCallbackArguments,
    mut rv: v8::ReturnValue,
  ) {
    let obj = args.get(0).try_into().unwrap();
    let wrapped =
      unsafe { v8::Object::unwrap::<TAG, Wrap>(scope, obj) }.unwrap();
    let inner = unsafe { wrapped.as_ref() }.inner.get(scope);
    let Some(other) = (unsafe {
      // SAFETY: Constructing on the stack.
      inner.other.get()
    }) else {
      return;
    };
    let int = *other.int.get(scope);
    rv.set(v8::Integer::new(scope, int).into());
  }

  let _guard = setup::sequential_test();

  {
    let isolate = &mut v8::Isolate::new(v8::CreateParams::default());

    {
      v8::scope!(handle_scope, isolate);
      let context = v8::Context::new(handle_scope, Default::default());
      let scope = &mut v8::ContextScope::new(handle_scope, context);
      let global = context.global(scope);
      {
        let func = v8::Function::new(scope, op_wrap).unwrap();
        let name = v8::String::new(scope, "wrap").unwrap();
        global.set(scope, name.into(), func.into()).unwrap();
      }
      {
        let func = v8::Function::new(scope, op_unwrap).unwrap();
        let name = v8::String::new(scope, "unwrap").unwrap();
        global.set(scope, name.into(), func.into()).unwrap();
      }

      execute_script(
        scope,
        r#"
          const a = wrap(123);
          const b = wrap(456, a);
          globalThis.testValue = unwrap(b); // 123
        "#,
      );

      {
        let mut vec = Vec::<u8>::new();
        scope.take_heap_snapshot(|chunk| {
          vec.extend_from_slice(chunk);
          true
        });
        let s = std::str::from_utf8(&vec).unwrap();
        assert!(s.contains("GcCellWrap"));
      }

      scope.request_garbage_collection_for_testing(
        v8::GarbageCollectionType::Full,
      );

      execute_script(
        scope,
        r#"
          if (globalThis.testValue !== 123) {
            throw new Error('testValue should be 123');
          }
        "#,
      );
    }
  }
}
