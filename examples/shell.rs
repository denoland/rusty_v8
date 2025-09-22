fn main() {
  // Initialize V8.
  let platform = v8::new_default_platform(0, false).make_shared();
  v8::V8::initialize_platform(platform);
  v8::V8::initialize();

  // Pass command line arguments to V8.
  let args: Vec<String> = std::env::args().collect();
  let args = v8::V8::set_flags_from_command_line(args);

  let mut run_shell_flag = args.len() == 1;
  let isolate = &mut v8::Isolate::new(v8::CreateParams::default());
  v8::scope!(let handle_scope, isolate);

  let context = v8::Context::new(handle_scope, Default::default());

  let mut scope = v8::ContextScope::new(handle_scope, context);

  run_main(&mut scope, &args, &mut run_shell_flag);

  if run_shell_flag {
    run_shell(&mut scope);
  }
}

/// Process remaining command line arguments and execute files
fn run_shell(scope: &mut v8::PinScope) {
  use std::io::{self, Write};

  println!("V8 version {} [sample shell]", v8::V8::get_version());

  loop {
    print!("> ");
    io::stdout().flush().unwrap();

    let mut buf = String::new();
    match io::stdin().read_line(&mut buf) {
      Ok(n) => {
        if n == 0 {
          println!();
          return;
        }

        execute_string(scope, &buf, "(shell)", true, true);
      }
      Err(error) => println!("error: {error}"),
    }
  }
}

/// Process remaining command line arguments and execute files
fn run_main(scope: &mut v8::PinScope, args: &[String], run_shell: &mut bool) {
  let mut skip_next = false;

  // Parse command-line arguments.
  for (i, arg) in args.iter().enumerate().skip(1) {
    if skip_next {
      continue;
    }

    match &**arg {
      "--shell" => {
        // Enables the shell.
        *run_shell = true;
      }
      "-f" => {
        // Ignore any -f flags for compatibility with the other stand-
        // alone JavaScript engines.
      }
      "-e" => {
        // Execute script.
        let script: &str = &args[i + 1];
        skip_next = true;

        execute_string(scope, script, "unnamed", false, true);

        while v8::Platform::pump_message_loop(
          &v8::V8::get_current_platform(),
          scope,
          false,
        ) {
          // do nothing
        }
      }
      arg => {
        if arg.starts_with("--") {
          eprintln!("Warning: unknown flag {arg}.\nTry --help for options");
          continue;
        }

        // Use all other arguments as names of files to load and run.
        let script = std::fs::read_to_string(arg).expect("failed to read file");
        execute_string(scope, &script, arg, false, true);

        while v8::Platform::pump_message_loop(
          &v8::V8::get_current_platform(),
          scope,
          false,
        ) {
          // do nothing
        }
      }
    }
  }
}

fn execute_string(
  scope: &mut v8::PinScope,
  script: &str,
  filename: &str,
  print_result: bool,
  report_exceptions_flag: bool,
) {
  v8::tc_scope!(let tc, scope);

  let filename = v8::String::new(tc, filename).unwrap();
  let script = v8::String::new(tc, script).unwrap();
  let origin = v8::ScriptOrigin::new(
    tc,
    filename.into(),
    0,
    0,
    false,
    0,
    None,
    false,
    false,
    false,
    None,
  );

  let script =
    if let Some(script) = v8::Script::compile(tc, script, Some(&origin)) {
      script
    } else {
      assert!(tc.has_caught());

      if report_exceptions_flag {
        report_exceptions(tc);
      }
      return;
    };

  if let Some(result) = script.run(tc) {
    if print_result {
      println!("{}", result.to_string(tc).unwrap().to_rust_string_lossy(tc));
    }
  } else {
    assert!(tc.has_caught());
    if report_exceptions_flag {
      report_exceptions(tc);
    }
  }
}

fn report_exceptions(
  try_catch: &mut v8::PinnedRef<'_, v8::TryCatch<v8::HandleScope>>,
) {
  let exception = try_catch.exception().unwrap();
  let exception_string = exception
    .to_string(try_catch)
    .unwrap()
    .to_rust_string_lossy(try_catch);
  let message = if let Some(message) = try_catch.message() {
    message
  } else {
    eprintln!("{exception_string}");
    return;
  };

  // Print (filename):(line number): (message).
  let filename = message.get_script_resource_name(try_catch).map_or_else(
    || "(unknown)".into(),
    |s| {
      s.to_string(try_catch)
        .unwrap()
        .to_rust_string_lossy(try_catch)
    },
  );
  let line_number = message.get_line_number(try_catch).unwrap_or_default();

  eprintln!("{filename}:{line_number}: {exception_string}");

  // Print line of source code.
  let source_line = message
    .get_source_line(try_catch)
    .map(|s| {
      s.to_string(try_catch)
        .unwrap()
        .to_rust_string_lossy(try_catch)
    })
    .unwrap();
  eprintln!("{source_line}");

  // Print wavy underline (GetUnderline is deprecated).
  let start_column = message.get_start_column();
  let end_column = message.get_end_column();

  for _ in 0..start_column {
    eprint!(" ");
  }

  for _ in start_column..end_column {
    eprint!("^");
  }

  eprintln!();

  // Print stack trace
  let stack_trace = if let Some(stack_trace) = try_catch.stack_trace() {
    stack_trace
  } else {
    return;
  };
  let stack_trace =
    unsafe { v8::Local::<v8::String>::cast_unchecked(stack_trace) };
  let stack_trace = stack_trace
    .to_string(try_catch)
    .map(|s| s.to_rust_string_lossy(try_catch));

  if let Some(stack_trace) = stack_trace {
    eprintln!("{stack_trace}");
  }
}
