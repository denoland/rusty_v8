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
  let handle_scope = &mut v8::HandleScope::new(isolate);

  let context = v8::Context::new(handle_scope);

  let context_scope = &mut v8::ContextScope::new(handle_scope, context);
  let scope = &mut v8::HandleScope::new(context_scope);

  run_main(scope, &*args, &mut run_shell_flag);

  if run_shell_flag {
    run_shell(scope);
  }
}

/// Process remaining command line arguments and execute files
fn run_shell(scope: &mut v8::HandleScope) {
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
      Err(error) => println!("error: {}", error),
    }
  }
}

/// Process remaining command line arguments and execute files
fn run_main(
  scope: &mut v8::HandleScope,
  args: &[String],
  run_shell: &mut bool,
) {
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
          eprintln!("Warning: unknown flag {}.\nTry --help for options", arg);
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
  scope: &mut v8::HandleScope,
  script: &str,
  filename: &str,
  print_result: bool,
  report_exceptions_flag: bool,
) {
  let mut scope = v8::TryCatch::new(scope);

  let filename = v8::String::new(&mut scope, filename).unwrap();
  let undefined = v8::undefined(&mut scope);
  let script = v8::String::new(&mut scope, script).unwrap();
  let origin = v8::ScriptOrigin::new(
    &mut scope,
    filename.into(),
    0,
    0,
    false,
    0,
    undefined.into(),
    false,
    false,
    false,
  );

  let script = if let Some(script) =
    v8::Script::compile(&mut scope, script, Some(&origin))
  {
    script
  } else {
    assert!(scope.has_caught());

    if report_exceptions_flag {
      report_exceptions(scope);
    }
    return;
  };

  if let Some(result) = script.run(&mut scope) {
    if print_result {
      println!(
        "{}",
        result
          .to_string(&mut scope)
          .unwrap()
          .to_rust_string_lossy(&mut scope)
      );
    }
  } else {
    assert!(scope.has_caught());
    if report_exceptions_flag {
      report_exceptions(scope);
    }
  }
}

fn report_exceptions(mut try_catch: v8::TryCatch<v8::HandleScope>) {
  let exception = try_catch.exception().unwrap();
  let exception_string = exception
    .to_string(&mut try_catch)
    .unwrap()
    .to_rust_string_lossy(&mut try_catch);
  let message = if let Some(message) = try_catch.message() {
    message
  } else {
    eprintln!("{}", exception_string);
    return;
  };

  // Print (filename):(line number): (message).
  let filename = message
    .get_script_resource_name(&mut try_catch)
    .map_or_else(
      || "(unknown)".into(),
      |s| {
        s.to_string(&mut try_catch)
          .unwrap()
          .to_rust_string_lossy(&mut try_catch)
      },
    );
  let line_number = message.get_line_number(&mut try_catch).unwrap_or_default();

  eprintln!("{}:{}: {}", filename, line_number, exception_string);

  // Print line of source code.
  let source_line = message
    .get_source_line(&mut try_catch)
    .map(|s| {
      s.to_string(&mut try_catch)
        .unwrap()
        .to_rust_string_lossy(&mut try_catch)
    })
    .unwrap();
  eprintln!("{}", source_line);

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
  let stack_trace = unsafe { v8::Local::<v8::String>::cast(stack_trace) };
  let stack_trace = stack_trace
    .to_string(&mut try_catch)
    .map(|s| s.to_rust_string_lossy(&mut try_catch));

  if let Some(stack_trace) = stack_trace {
    eprintln!("{}", stack_trace);
  }
}
