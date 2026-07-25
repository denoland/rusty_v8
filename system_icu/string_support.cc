// Copyright 2018-2026 the Deno authors. MIT license.

#include "unicode/ustring.h"
#include "unicode/utf16.h"
#include "ustr_imp.h"

namespace {

bool IsSurrogatePairAt(const char16_t* string, int32_t length, int32_t index) {
  const char16_t value = string[index];
  return (U16_IS_LEAD(value) && index + 1 < length &&
          U16_IS_TRAIL(string[index + 1])) ||
         (U16_IS_TRAIL(value) && index > 0 && U16_IS_LEAD(string[index - 1]));
}

}  // namespace

U_CFUNC int32_t U_EXPORT2 uprv_strCompare(
    const char16_t* left, int32_t left_length, const char16_t* right,
    int32_t right_length, UBool strncmp_style, UBool code_point_order) {
  if (left == right && left_length == right_length) {
    return 0;
  }
  if (left_length < 0) {
    left_length = u_strlen(left);
  }
  if (right_length < 0) {
    right_length = u_strlen(right);
  }

  const int32_t common_length =
      left_length < right_length ? left_length : right_length;
  int32_t index = 0;
  char16_t left_value = 0;
  char16_t right_value = 0;
  while (index < common_length) {
    left_value = left[index];
    right_value = right[index];
    if (left_value != right_value) {
      break;
    }
    if (strncmp_style && left_value == 0) {
      return 0;
    }
    ++index;
  }

  if (index == common_length) {
    return left_length < right_length ? -1 : left_length > right_length ? 1 : 0;
  }

  if (code_point_order && left_value >= 0xd800 && right_value >= 0xd800) {
    if (!IsSurrogatePairAt(left, left_length, index)) {
      left_value -= 0x2800;
    }
    if (!IsSurrogatePairAt(right, right_length, index)) {
      right_value -= 0x2800;
    }
  }
  return static_cast<int32_t>(left_value) - static_cast<int32_t>(right_value);
}
