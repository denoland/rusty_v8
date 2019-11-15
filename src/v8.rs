// Copyright 2018-2019 the Deno authors. All rights reserved. MIT license.

use libc::c_char;
use libc::c_int;
use std::ffi::CStr;
use std::ffi::CString;
use std::vec::Vec;

extern "C" {
  pub fn v8__V8__SetFlagsFromCommandLine(
    argc: *mut c_int,
    argv: *mut *mut c_char,
  );

  pub fn v8__V8__GetVersion() -> *const c_char;
}

/// Pass the command line arguments to v8.
/// The first element of args (which usually corresponds to the binary name) is
/// ignored.
/// Returns a vector of command line arguments that V8 did not understand.
pub fn set_flags_from_command_line(args: Vec<String>) -> Vec<String> {
  // deno_set_v8_flags(int* argc, char** argv) mutates argc and argv to remove
  // flags that v8 understands.

  // Make a new array, that can be modified by V8::SetFlagsFromCommandLine(),
  // containing mutable raw pointers to the individual command line args.
  let mut raw_argv = args
    .iter()
    .map(|arg| CString::new(arg.as_str()).unwrap().into_bytes_with_nul())
    .collect::<Vec<_>>();
  let mut c_argv = raw_argv
    .iter_mut()
    .map(|arg| arg.as_mut_ptr() as *mut c_char)
    .collect::<Vec<_>>();

  // Store the length of the c_argv array in a local variable. We'll pass
  // a pointer to this local variable to deno_set_v8_flags(), which then
  // updates its value.
  let mut c_argv_len = c_argv.len() as c_int;
  // Let v8 parse the arguments it recognizes and remove them from c_argv.
  unsafe {
    v8__V8__SetFlagsFromCommandLine(&mut c_argv_len, c_argv.as_mut_ptr())
  };
  // If c_argv_len was updated we have to change the length of c_argv to match.
  c_argv.truncate(c_argv_len as usize);
  // Copy the modified arguments list into a proper rust vec and return it.
  c_argv
    .iter()
    .map(|ptr| unsafe {
      let cstr = CStr::from_ptr(*ptr as *const c_char);
      let slice = cstr.to_str().unwrap();
      slice.to_string()
    })
    .collect()
}

#[test]
fn test_set_flags_from_command_line() {
  let r = set_flags_from_command_line(vec![
    "binaryname".to_string(),
    "--log".to_string(),
    "--should-be-ignored".to_string(),
  ]);
  assert_eq!(
    r,
    vec!["binaryname".to_string(), "--should-be-ignored".to_string()]
  );
}

pub fn get_version() -> &'static str {
  let version = unsafe { v8__V8__GetVersion() };
  let c_str = unsafe { CStr::from_ptr(version) };
  c_str.to_str().unwrap()
}

#[test]
fn test_get_version() {
  assert!(get_version().len() > 3);
}
