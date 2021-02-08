extern "C" {
  fn udata_setCommonData_68(this: *const u8, error_code: *mut i32);
}

/// Set ICU data file http://userguide.icu-project.org/icudata
/// Returns error code.
// TODO(ry) Map error code to something useful.
pub fn set_common_data(data: &'static [u8]) -> Result<(), i32> {
  let mut error_code = 0i32;
  unsafe {
    udata_setCommonData_68(data.as_ptr(), &mut error_code as &mut i32);
  }
  if error_code == 0 {
    Ok(())
  } else {
    Err(error_code)
  }
}
