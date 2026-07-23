// Copyright 2018-2026 the Deno authors. MIT license.

#include <array>
#include <memory>
#include <vector>

#include "unicode/plurrule.h"
#include "unicode/uloc.h"

U_NAMESPACE_BEGIN
namespace {

template <typename Select>
UnicodeString SelectKeyword(Select select, UErrorCode& status) {
  if (U_FAILURE(status)) {
    return {};
  }

  std::array<UChar, 32> stack_buffer;
  int32_t length = select(stack_buffer.data(), stack_buffer.size(), &status);
  if (status != U_BUFFER_OVERFLOW_ERROR) {
    return U_SUCCESS(status)
               ? UnicodeString(
                     reinterpret_cast<const char16_t*>(stack_buffer.data()),
                     length)
               : UnicodeString();
  }

  status = U_ZERO_ERROR;
  std::vector<UChar> buffer(length + 1);
  length = select(buffer.data(), buffer.size(), &status);
  return U_SUCCESS(status)
             ? UnicodeString(reinterpret_cast<const char16_t*>(buffer.data()),
                             length)
             : UnicodeString();
}

}  // namespace

PluralRules::PluralRules(UPluralRules* rules) : rules_(rules) {}

PluralRules::~PluralRules() { uplrules_close(rules_); }

PluralRules* PluralRules::forLocale(const Locale& locale, UPluralType type,
                                    UErrorCode& status) {
  UPluralRules* rules = uplrules_openForType(locale.getName(), type, &status);
  return U_SUCCESS(status) && rules != nullptr ? new PluralRules(rules)
                                               : nullptr;
}

UnicodeString PluralRules::select(const number::FormattedNumber& number,
                                  UErrorCode& status) const {
  return SelectKeyword(
      [&](UChar* result, int32_t capacity, UErrorCode* error) {
        return uplrules_selectFormatted(rules_, number.toUFormattedNumber(),
                                        result, capacity, error);
      },
      status);
}

UnicodeString PluralRules::select(const number::FormattedNumberRange& range,
                                  UErrorCode& status) const {
  return SelectKeyword(
      [&](UChar* result, int32_t capacity, UErrorCode* error) {
        return uplrules_selectForRange(rules_, range.toUFormattedNumberRange(),
                                       result, capacity, error);
      },
      status);
}

StringEnumeration* PluralRules::getKeywords(UErrorCode& status) const {
  UEnumeration* enumeration = uplrules_getKeywords(rules_, &status);
  return U_SUCCESS(status) && enumeration != nullptr
             ? new StringEnumeration(enumeration)
             : nullptr;
}

StringEnumeration* PluralRules::getAvailableLocales(UErrorCode& status) {
  // The C++ API exposes the formatting locale registry. The public C API's
  // locale registry is the same superset and is stable for the process
  // lifetime, which makes it safe to wrap in a UEnumeration without copying.
  static const std::vector<const char*> locales = [] {
    std::vector<const char*> result;
    const int32_t count = uloc_countAvailable();
    result.reserve(count);
    for (int32_t i = 0; i < count; ++i) {
      result.push_back(uloc_getAvailable(i));
    }
    return result;
  }();

  UEnumeration* enumeration = uenum_openCharStringsEnumeration(
      locales.data(), static_cast<int32_t>(locales.size()), &status);
  return U_SUCCESS(status) && enumeration != nullptr
             ? new StringEnumeration(enumeration)
             : nullptr;
}

U_NAMESPACE_END
