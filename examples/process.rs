use std::collections::HashMap;
use std::convert::TryFrom;

#[allow(clippy::needless_pass_by_value)] // this function should follow the callback type
fn log_callback(
  scope: &mut v8::PinScope,
  args: v8::FunctionCallbackArguments,
  mut _retval: v8::ReturnValue,
) {
  let message = args
    .get(0)
    .to_string(scope)
    .unwrap()
    .to_rust_string_lossy(scope);

  println!("Logged: {message}");
}

fn main() {
  // Initialize V8.
  let platform = v8::new_default_platform(0, false).make_shared();
  v8::V8::initialize_platform(platform);
  v8::V8::initialize();

  // Parse options.
  let (options, file) = parse_args();
  if file.is_empty() {
    panic!("no script was specified");
  }

  let mut isolate = v8::Isolate::new(v8::CreateParams::default());
  let scope = std::pin::pin!(v8::HandleScope::new(&mut isolate));
  let scope = &mut scope.init();

  let source = std::fs::read_to_string(&file)
    .unwrap_or_else(|err| panic!("failed to open {file}: {err}"));
  let source = v8::String::new(scope, &source).unwrap();

  let mut processor = JsHttpRequestProcessor::new(scope, source, options);

  let requests = vec![
    StringHttpRequest::new("/process.cc", "localhost", "google.com", "firefox"),
    StringHttpRequest::new("/", "localhost", "google.net", "firefox"),
    StringHttpRequest::new("/", "localhost", "google.org", "safari"),
    StringHttpRequest::new("/", "localhost", "yahoo.com", "ie"),
    StringHttpRequest::new("/", "localhost", "yahoo.com", "safari"),
    StringHttpRequest::new("/", "localhost", "yahoo.com", "firefox"),
  ];

  for req in requests {
    processor.process(req);
  }

  processor.print_output();
}

fn parse_args() -> (HashMap<String, String>, String) {
  use std::env;
  let args: Vec<String> = env::args().collect();
  let mut options = HashMap::new();
  let mut file = String::new();

  for arg in &args {
    if let Some(pos) = arg.find('=') {
      let (key, value) = arg.split_at(pos);
      let value = &value[1..];
      options.insert(key.into(), value.into());
    } else {
      file = arg.into();
    }
  }

  (options, file)
}

/// A simplified HTTP request.
trait HttpRequest {
  fn path(&self) -> &str;
  fn referrer(&self) -> &str;
  fn host(&self) -> &str;
  fn user_agent(&self) -> &str;
}

/// A simplified HTTP request.
struct StringHttpRequest {
  pub path: String,
  pub referrer: String,
  pub host: String,
  pub user_agent: String,
}

impl StringHttpRequest {
  /// Creates a `StringHttpRequest`.
  pub fn new(
    path: impl Into<String>,
    referrer: impl Into<String>,
    host: impl Into<String>,
    user_agent: impl Into<String>,
  ) -> Self {
    Self {
      path: path.into(),
      referrer: referrer.into(),
      host: host.into(),
      user_agent: user_agent.into(),
    }
  }
}

impl HttpRequest for StringHttpRequest {
  fn path(&self) -> &str {
    &self.path
  }

  fn referrer(&self) -> &str {
    &self.referrer
  }

  fn host(&self) -> &str {
    &self.host
  }

  fn user_agent(&self) -> &str {
    &self.user_agent
  }
}

/// An http request processor that is scriptable using JavaScript.
struct JsHttpRequestProcessor<'scope, 'obj, 'isolate> {
  context: v8::Local<'obj, v8::Context>,
  context_scope: v8::ContextScope<'scope, 'obj, v8::HandleScope<'isolate>>,
  process_fn: Option<v8::Local<'obj, v8::Function>>,
  request_template: v8::Global<v8::ObjectTemplate>,
  _map_template: Option<v8::Global<v8::ObjectTemplate>>,
}

impl<'scope, 'obj, 'isolate> JsHttpRequestProcessor<'scope, 'obj, 'isolate> {
  /// Creates a scriptable HTTP request processor.
  pub fn new(
    isolate_scope: &'scope mut v8::PinScope<'obj, 'isolate, ()>,
    source: v8::Local<'obj, v8::String>,
    options: HashMap<String, String>,
  ) -> Self {
    let global = v8::ObjectTemplate::new(isolate_scope);
    global.set(
      v8::String::new(isolate_scope, "log").unwrap().into(),
      v8::FunctionTemplate::new(isolate_scope, log_callback).into(),
    );

    let context = v8::Context::new(
      isolate_scope,
      v8::ContextOptions {
        global_template: Some(global),
        ..Default::default()
      },
    );
    let context_scope = v8::ContextScope::new(isolate_scope, context);

    let request_template = v8::ObjectTemplate::new(&context_scope);
    request_template.set_internal_field_count(1);

    // make it global
    let request_template = v8::Global::new(&context_scope, request_template);

    let mut self_ = JsHttpRequestProcessor {
      context,
      context_scope,
      process_fn: None,
      request_template,
      _map_template: None,
    };

    // loads options and output
    let options = self_.wrap_map(options);
    let options_str = v8::String::new(&self_.context_scope, "options").unwrap();
    self_.context.global(&self_.context_scope).set(
      &self_.context_scope,
      options_str.into(),
      options.into(),
    );

    let output = v8::Object::new(&self_.context_scope);
    let output_str = v8::String::new(&self_.context_scope, "output").unwrap();
    self_.context.global(&self_.context_scope).set(
      &self_.context_scope,
      output_str.into(),
      output.into(),
    );

    // execute script
    self_.execute_script(source);

    let process_str =
      v8::String::new(&mut self_.context_scope, "Process").unwrap();
    let process_fn = self_
      .context
      .global(&self_.context_scope)
      .get(&self_.context_scope, process_str.into())
      .expect("missing function Process");

    let process_fn = v8::Local::<v8::Function>::try_from(process_fn)
      .expect("function expected");
    self_.process_fn = Some(process_fn);

    self_
  }

  fn execute_script(&mut self, script: v8::Local<'scope, v8::String>) {
    let scope = std::pin::pin!(v8::HandleScope::new(&mut *self.context_scope));
    let scope = &mut scope.init();
    let try_catch = std::pin::pin!(v8::TryCatch::new(scope));
    let try_catch = &mut try_catch.init();

    let script = v8::Script::compile(try_catch, script, None)
      .expect("failed to compile script");

    if script.run(try_catch).is_none() {
      let exception = try_catch.exception().unwrap();
      let exception_string = exception
        .to_string(try_catch)
        .unwrap()
        .to_rust_string_lossy(try_catch);

      panic!("{exception_string}");
    }
  }

  /// Processes the given HTTP request.
  pub fn process<R>(&mut self, request: R)
  where
    R: HttpRequest + 'static,
  {
    let request: Box<dyn HttpRequest> = Box::new(request);
    let request = self.wrap_request(request);

    let scope = std::pin::pin!(v8::HandleScope::new(&mut *self.context_scope));
    let scope = &mut scope.init();
    let try_catch = std::pin::pin!(v8::TryCatch::new(scope));
    let try_catch = &mut try_catch.init();

    let process_fn = self.process_fn.as_mut().unwrap();
    let global = self.context.global(try_catch).into();

    if process_fn
      .call(try_catch, global, &[request.into()])
      .is_none()
    {
      let exception = try_catch.exception().unwrap();
      let exception_string = exception
        .to_string(try_catch)
        .unwrap()
        .to_rust_string_lossy(try_catch);

      panic!("{exception_string}");
    }
  }

  /// Utility function that wraps a http request object in a JavaScript object.
  fn wrap_request(
    &mut self,
    request: Box<dyn HttpRequest>,
  ) -> v8::Local<'scope, v8::Object> {
    // TODO: fix memory leak

    use std::ffi::c_void;

    // Double-box to get C-sized reference of Box<dyn HttpRequest>
    let request = Box::new(request);

    // Local scope for temporary handles.
    let scope = &mut self.context_scope;

    let request_template = v8::Local::new(scope, &self.request_template);
    let result = request_template.new_instance(scope).unwrap();

    let external = v8::External::new(
      scope,
      Box::leak(request) as *mut Box<dyn HttpRequest> as *mut c_void,
    );

    result.set_internal_field(0, external.into());

    let name = v8::String::new(scope, "path").unwrap().into();
    result.set_accessor(scope, name, Self::request_prop_handler);
    let name = v8::String::new(scope, "userAgent").unwrap().into();
    result.set_accessor(scope, name, Self::request_prop_handler);
    let name = v8::String::new(scope, "referrer").unwrap().into();
    result.set_accessor(scope, name, Self::request_prop_handler);
    let name = v8::String::new(scope, "host").unwrap().into();
    result.set_accessor(scope, name, Self::request_prop_handler);

    result
  }

  /// This handles the properties of `HttpRequest`
  #[allow(clippy::needless_pass_by_value)] // this function should follow the callback type
  fn request_prop_handler(
    scope: &mut v8::PinScope,
    key: v8::Local<v8::Name>,
    args: v8::PropertyCallbackArguments,
    mut rv: v8::ReturnValue,
  ) {
    let this = args.this();
    let external = Self::unwrap_request(scope, this);

    assert!(
      !external.is_null(),
      "the pointer to Box<dyn HttpRequest> should not be null"
    );

    let request = unsafe { &mut *external };

    let key = key.to_string(scope).unwrap().to_rust_string_lossy(scope);

    let value = match &*key {
      "path" => request.path(),
      "userAgent" => request.user_agent(),
      "referrer" => request.referrer(),
      "host" => request.host(),
      _ => {
        return;
      }
    };

    rv.set(v8::String::new(scope, value).unwrap().into());
  }

  /// Utility function that extracts the http request object from a wrapper object.
  fn unwrap_request(
    scope: &v8::PinScope,
    request: v8::Local<v8::Object>,
  ) -> *mut Box<dyn HttpRequest> {
    let external = request
      .get_internal_field(scope, 0)
      .unwrap()
      .cast::<v8::External>();
    external.value() as *mut Box<dyn HttpRequest>
  }

  fn wrap_map(
    &mut self,
    options: HashMap<String, String>,
  ) -> v8::Local<'scope, v8::Object> {
    // TODO: wrap map, not convert into Object
    let scope = &self.context_scope;
    let result = v8::Object::new(scope);

    for (key, value) in options {
      let key = v8::String::new(scope, &key).unwrap().into();
      let value = v8::String::new(scope, &value).unwrap().into();
      result.set(scope, key, value);
    }

    result
  }

  /// Prints the output.
  pub fn print_output(&mut self) {
    let scope: std::pin::Pin<&mut v8::ScopeStorage<v8::HandleScope<'_>>> =
      std::pin::pin!(v8::HandleScope::new(&mut self.context_scope));
    let scope = &scope.init();
    let key = v8::String::new(scope, "output").unwrap();
    let output = self
      .context
      .global(scope)
      .get(scope, key.into())
      .unwrap()
      .to_object(scope)
      .unwrap();

    let props = output
      .get_property_names(scope, v8::GetPropertyNamesArgsBuilder::new().build())
      .unwrap();
    for i in 0..props.length() {
      let key = props.get_index(scope, i).unwrap();
      let value = output.get(scope, key).unwrap();

      let key = key.to_string(scope).unwrap().to_rust_string_lossy(scope);
      let value = value.to_string(scope).unwrap().to_rust_string_lossy(scope);

      println!("{key}: {value}");
    }
  }
}
