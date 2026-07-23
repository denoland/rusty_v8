// Copyright 2018-2026 the Deno authors. MIT license.

#include "unicode/strenum.h"

U_NAMESPACE_BEGIN

StringEnumeration::StringEnumeration(UEnumeration* enumeration)
    : enumeration_(enumeration) {}

StringEnumeration::~StringEnumeration() { uenum_close(enumeration_); }

const char* StringEnumeration::next(int32_t* result_length,
                                    UErrorCode& status) {
  return uenum_next(enumeration_, result_length, &status);
}

int32_t StringEnumeration::count(UErrorCode& status) const {
  return uenum_count(enumeration_, &status);
}

void StringEnumeration::reset(UErrorCode& status) {
  uenum_reset(enumeration_, &status);
}

U_NAMESPACE_END
