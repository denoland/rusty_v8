// Copyright 2018-2026 the Deno authors. MIT license.

#include <cstring>
#include <vector>

#include "unicode/normalizer2.h"

U_NAMESPACE_BEGIN

Normalizer2::Normalizer2(const UNormalizer2* normalizer)
    : normalizer_(normalizer) {}

const Normalizer2* Normalizer2::getInstance(const char* package_name,
                                            const char* name,
                                            UNormalization2Mode mode,
                                            UErrorCode& status) {
  const UNormalizer2* normalizer =
      unorm2_getInstance(package_name, name, mode, &status);
  // ICU owns the C singleton. V8 obtains only four process-lifetime
  // normalizers, so keeping the tiny corresponding C++ views is intentional.
  return normalizer == nullptr ? nullptr : new Normalizer2(normalizer);
}

int32_t Normalizer2::spanQuickCheckYes(const UnicodeString& input,
                                       UErrorCode& status) const {
  return unorm2_spanQuickCheckYes(
      normalizer_, reinterpret_cast<const UChar*>(input.getBuffer()),
      input.length(), &status);
}

UnicodeString& Normalizer2::normalizeSecondAndAppend(
    UnicodeString& first, const UnicodeString& second,
    UErrorCode& status) const {
  if (U_FAILURE(status)) {
    return first;
  }

  const int32_t first_length = first.length();
  const int32_t second_length = second.length();
  int32_t capacity = first_length + second_length + 32;
  std::vector<UChar> buffer(capacity);
  std::memcpy(buffer.data(), first.getBuffer(),
              first_length * sizeof(char16_t));

  int32_t length = unorm2_normalizeSecondAndAppend(
      normalizer_, buffer.data(), first_length, capacity,
      reinterpret_cast<const UChar*>(second.getBuffer()), second_length,
      &status);
  if (status == U_BUFFER_OVERFLOW_ERROR) {
    status = U_ZERO_ERROR;
    capacity = length + 1;
    buffer.resize(capacity);
    std::memcpy(buffer.data(), first.getBuffer(),
                first_length * sizeof(char16_t));
    length = unorm2_normalizeSecondAndAppend(
        normalizer_, buffer.data(), first_length, capacity,
        reinterpret_cast<const UChar*>(second.getBuffer()), second_length,
        &status);
  }

  if (U_SUCCESS(status)) {
    first.remove();
    first.append(reinterpret_cast<const char16_t*>(buffer.data()), length);
  }
  return first;
}

U_NAMESPACE_END
