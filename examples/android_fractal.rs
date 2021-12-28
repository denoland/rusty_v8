// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.

// Don't run on non-Android targets.
#![cfg(target_os = "android")]
// Don't run this as a test in `--all-targets` mode.
#![cfg(not(test))]

use pixels::Pixels;
use pixels::SurfaceTexture;
use std::cell::Cell;
use winit::platform::run_return::EventLoopExtRunReturn;

#[ndk_glue::main(
  backtrace = "on",
  logger(level = "debug", tag = "android_fractal")
)]
fn main() {
  let mut event_loop = winit::event_loop::EventLoop::new();
  let window = winit::window::WindowBuilder::new()
    .with_title("rusty_v8 android_fractal")
    .build(&event_loop)
    .unwrap();

  // Initialize V8.
  let platform = v8::new_default_platform(0, false).make_shared();
  v8::V8::initialize_platform(platform);
  v8::V8::initialize();

  let mut isolate = v8::Isolate::new(v8::CreateParams::default());
  let mut scope = v8::HandleScope::new(&mut isolate);
  let source =
    v8::String::new(&mut scope, include_str!("android_fractal.js")).unwrap();

  let context = v8::Context::new(&mut scope);
  let mut context_scope = v8::ContextScope::new(&mut scope, context);

  execute_script(&mut context_scope, source);

  let draw_str = v8::String::new(&mut context_scope, "DrawFrame").unwrap();
  let draw_fn = context
    .global(&mut context_scope)
    .get(&mut context_scope, draw_str.into())
    .expect("missing function DrawFrame");

  let draw_fn =
    v8::Local::<v8::Function>::try_from(draw_fn).expect("function expected");

  let mut allowed = false;

  loop {
    event_loop.run_return(|event, _, control_flow| {
      *control_flow = winit::event_loop::ControlFlow::Wait;
      match event {
        winit::event::Event::WindowEvent {
          event: winit::event::WindowEvent::CloseRequested,
          ..
        } => *control_flow = winit::event_loop::ControlFlow::Exit,
        // Drawing on android must only happen before Event::Suspended and
        // after Event::Resumed.
        //
        // https://github.com/rust-windowing/winit/issues/1588
        winit::event::Event::Resumed => {
          allowed = true;
        }
        winit::event::Event::Suspended => {
          allowed = false;
        }
        winit::event::Event::RedrawRequested(_) => {
          if !allowed {
            return;
          }
          let surface_texture = SurfaceTexture::new(800, 800, &window);
          let mut pixels = Pixels::new(800, 800, surface_texture).unwrap();

          draw(&mut context_scope, draw_fn, pixels.get_frame());

          if pixels.render().is_err() {
            *control_flow = winit::event_loop::ControlFlow::Exit;
            return;
          }
        }
        _ => {}
      }

      window.request_redraw();
    });
  }
}

fn execute_script(
  context_scope: &mut v8::ContextScope<v8::HandleScope>,
  script: v8::Local<v8::String>,
) {
  let scope = &mut v8::HandleScope::new(context_scope);
  let try_catch = &mut v8::TryCatch::new(scope);

  let script = v8::Script::compile(try_catch, script, None)
    .expect("failed to compile script");

  if script.run(try_catch).is_none() {
    let exception_string = try_catch
      .stack_trace()
      .or_else(|| try_catch.exception())
      .map(|value| value.to_rust_string_lossy(try_catch))
      .unwrap_or_else(|| "no stack trace".into());

    panic!("{}", exception_string);
  }
}

fn draw(
  context_scope: &mut v8::ContextScope<v8::HandleScope>,
  draw_fn: v8::Local<v8::Function>,
  frame: &mut [u8],
) {
  let scope = &mut v8::HandleScope::new(context_scope);
  let recv = v8::undefined(scope);
  let try_catch = &mut v8::TryCatch::new(scope);

  let len = frame.len();
  let frame_len = v8::Integer::new(try_catch, len as i32);

  let ab = match draw_fn.call(try_catch, recv.into(), &[frame_len.into()]) {
    Some(ab) => ab,
    None => {
      let exception_string = try_catch
        .stack_trace()
        .or_else(|| try_catch.exception())
        .map(|value| value.to_rust_string_lossy(try_catch))
        .unwrap_or_else(|| "no stack trace".into());

      panic!("{}", exception_string);
    }
  };

  let ab =
    v8::Local::<v8::ArrayBuffer>::try_from(ab).expect("array buffer expected");
  let bs = ab.get_backing_store();

  let js_frame = unsafe { get_backing_store_slice(&bs, 0, len) };
  frame.copy_from_slice(js_frame.as_ref());
}

unsafe fn get_backing_store_slice(
  backing_store: &v8::SharedRef<v8::BackingStore>,
  byte_offset: usize,
  byte_length: usize,
) -> &[u8] {
  let cells: *const [Cell<u8>] =
    &backing_store[byte_offset..byte_offset + byte_length];
  let bytes = cells as *const [u8];
  &*bytes
}
