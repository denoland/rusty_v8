extern "C" {
  fn udata_setCommonData_69(this: *const u8, error_code: *mut i32);
}

/// This function bypasses the normal ICU data loading process and allows you to force ICU's system
/// data to come out of a user-specified area in memory.
///
/// ICU data must be at least 8-aligned, and should be 16-aligned. See
/// https://unicode-org.github.io/icu/userguide/icudata
///
/// The format of this data is that of the icu common data file, as is generated by the pkgdata
/// tool with mode=common or mode=dll. You can read in a whole common mode file and pass the
/// address to the start of the data, or (with the appropriate link options) pass in the pointer to
/// the data that has been loaded from a dll by the operating system, as shown in this code:
///
/// ```c++
///       extern const char U_IMPORT U_ICUDATA_ENTRY_POINT [];
///        // U_ICUDATA_ENTRY_POINT is same as entry point specified to pkgdata tool
///       UErrorCode  status = U_ZERO_ERROR;
///
///       udata_setCommonData(&U_ICUDATA_ENTRY_POINT, &status);
/// ```
///
/// It is important that the declaration be as above. The entry point must not be declared as an
/// extern void*.
///
/// Starting with ICU 4.4, it is possible to set several data packages, one per call to this
/// function. udata_open() will look for data in the multiple data packages in the order in which
/// they were set. The position of the linked-in or default-name ICU .data package in the search
/// list depends on when the first data item is loaded that is not contained in the already
/// explicitly set packages. If data was loaded implicitly before the first call to this function
/// (for example, via opening a converter, constructing a UnicodeString from default-codepage data,
/// using formatting or collation APIs, etc.), then the default data will be first in the list.
///
/// This function has no effect on application (non ICU) data. See udata_setAppData() for similar
/// functionality for application data.
// TODO(ry) Map error code to something useful.
pub fn set_common_data(data: &'static [u8]) -> Result<(), i32> {
  let mut error_code = 0i32;
  unsafe {
    udata_setCommonData_69(data.as_ptr(), &mut error_code);
  }
  if error_code == 0 {
    Ok(())
  } else {
    Err(error_code)
  }
}
