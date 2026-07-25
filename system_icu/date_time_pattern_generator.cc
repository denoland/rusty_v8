// Copyright 2018-2026 the Deno authors. MIT license.

#include <vector>

#include "unicode/dtptngen.h"

U_NAMESPACE_BEGIN
namespace {

template <typename Get>
UnicodeString GetString(Get get, UErrorCode& status) {
  if (U_FAILURE(status)) {
    return {};
  }
  int32_t length = get(nullptr, 0, &status);
  if (status != U_BUFFER_OVERFLOW_ERROR && U_FAILURE(status)) {
    return {};
  }
  status = U_ZERO_ERROR;
  std::vector<UChar> result(length + 1);
  length = get(result.data(), result.size(), &status);
  return U_SUCCESS(status)
             ? UnicodeString(reinterpret_cast<const char16_t*>(result.data()),
                             length)
             : UnicodeString();
}

}  // namespace

DateTimePatternGenerator::DateTimePatternGenerator(
    UDateTimePatternGenerator* generator)
    : generator_(generator) {}

DateTimePatternGenerator::~DateTimePatternGenerator() {
  udatpg_close(generator_);
}

DateTimePatternGenerator* DateTimePatternGenerator::createInstance(
    const Locale& locale, UErrorCode& status) {
  UDateTimePatternGenerator* generator = udatpg_open(locale.getName(), &status);
  return U_SUCCESS(status) && generator != nullptr
             ? new DateTimePatternGenerator(generator)
             : nullptr;
}

DateTimePatternGenerator* DateTimePatternGenerator::clone() const {
  UErrorCode status = U_ZERO_ERROR;
  UDateTimePatternGenerator* copy = udatpg_clone(generator_, &status);
  return U_SUCCESS(status) && copy != nullptr
             ? new DateTimePatternGenerator(copy)
             : nullptr;
}

UnicodeString DateTimePatternGenerator::getBestPattern(
    const UnicodeString& skeleton, UDateTimePatternMatchOptions options,
    UErrorCode& status) {
  return GetString(
      [&](UChar* result, int32_t capacity, UErrorCode* error) {
        return udatpg_getBestPatternWithOptions(
            generator_, reinterpret_cast<const UChar*>(skeleton.getBuffer()),
            skeleton.length(), options, result, capacity, error);
      },
      status);
}

UnicodeString DateTimePatternGenerator::staticGetSkeleton(
    const UnicodeString& pattern, UErrorCode& status) {
  return GetString(
      [&](UChar* result, int32_t capacity, UErrorCode* error) {
        return udatpg_getSkeleton(
            nullptr, reinterpret_cast<const UChar*>(pattern.getBuffer()),
            pattern.length(), result, capacity, error);
      },
      status);
}

UDateFormatHourCycle DateTimePatternGenerator::getDefaultHourCycle(
    UErrorCode& status) const {
  return udatpg_getDefaultHourCycle(generator_, &status);
}

UnicodeString DateTimePatternGenerator::getFieldDisplayName(
    UDateTimePatternField field, UDateTimePGDisplayWidth width) const {
  UErrorCode status = U_ZERO_ERROR;
  return GetString(
      [&](UChar* result, int32_t capacity, UErrorCode* error) {
        return udatpg_getFieldDisplayName(generator_, field, width, result,
                                          capacity, error);
      },
      status);
}

U_NAMESPACE_END
