// Copyright 2018-2026 the Deno authors. MIT license.

#include <vector>

#include "unicode/locdspnm.h"

U_NAMESPACE_BEGIN
namespace {

template <typename Get>
UnicodeString& GetDisplayName(Get get, UnicodeString& result) {
  UErrorCode status = U_ZERO_ERROR;
  int32_t length = get(nullptr, 0, &status);
  if (status != U_BUFFER_OVERFLOW_ERROR && U_FAILURE(status)) {
    result.setToBogus();
    return result;
  }
  status = U_ZERO_ERROR;
  std::vector<UChar> buffer(length + 1);
  length = get(buffer.data(), buffer.size(), &status);
  if (U_SUCCESS(status)) {
    result =
        UnicodeString(reinterpret_cast<const char16_t*>(buffer.data()), length);
  } else {
    result.setToBogus();
  }
  return result;
}

}  // namespace

LocaleDisplayNames::LocaleDisplayNames(ULocaleDisplayNames* names,
                                       const Locale& locale)
    : names_(names), locale_(locale) {}

LocaleDisplayNames::~LocaleDisplayNames() { uldn_close(names_); }

LocaleDisplayNames* LocaleDisplayNames::createInstance(
    const Locale& locale, UDisplayContext* contexts, int32_t length) {
  UErrorCode status = U_ZERO_ERROR;
  ULocaleDisplayNames* names =
      uldn_openForContext(locale.getName(), contexts, length, &status);
  if (U_FAILURE(status) || names == nullptr) {
    return nullptr;
  }
  return new LocaleDisplayNames(names, Locale(uldn_getLocale(names)));
}

UnicodeString& LocaleDisplayNames::localeDisplayName(
    const char* locale, UnicodeString& result) const {
  return GetDisplayName(
      [&](UChar* buffer, int32_t capacity, UErrorCode* status) {
        return uldn_localeDisplayName(names_, locale, buffer, capacity, status);
      },
      result);
}

UnicodeString& LocaleDisplayNames::scriptDisplayName(
    const char* script, UnicodeString& result) const {
  return GetDisplayName(
      [&](UChar* buffer, int32_t capacity, UErrorCode* status) {
        return uldn_scriptDisplayName(names_, script, buffer, capacity, status);
      },
      result);
}

UnicodeString& LocaleDisplayNames::regionDisplayName(
    const char* region, UnicodeString& result) const {
  return GetDisplayName(
      [&](UChar* buffer, int32_t capacity, UErrorCode* status) {
        return uldn_regionDisplayName(names_, region, buffer, capacity, status);
      },
      result);
}

UnicodeString& LocaleDisplayNames::keyValueDisplayName(
    const char* key, const char* value, UnicodeString& result) const {
  return GetDisplayName(
      [&](UChar* buffer, int32_t capacity, UErrorCode* status) {
        return uldn_keyValueDisplayName(names_, key, value, buffer, capacity,
                                        status);
      },
      result);
}

U_NAMESPACE_END
