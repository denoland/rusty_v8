// Copyright 2018-2026 the Deno authors. MIT license.

#include <array>
#include <cstring>
#include <mutex>
#include <string>
#include <vector>

#include "unicode/uenum.h"
#include "unicode/uloc.h"

namespace {

struct AvailableLocales {
  std::once_flag initialized;
  std::vector<std::string> storage;
  std::vector<const char*> pointers;
  UErrorCode status = U_ZERO_ERROR;
};

std::array<AvailableLocales, ULOC_AVAILABLE_COUNT> available_locales;

}  // namespace

extern "C" UEnumeration* v8_icu_compat_uloc_openAvailableByType(
    ULocAvailableType type, UErrorCode* status) {
  if (U_FAILURE(*status) || type < ULOC_AVAILABLE_DEFAULT ||
      type >= ULOC_AVAILABLE_COUNT) {
    if (U_SUCCESS(*status)) {
      *status = U_ILLEGAL_ARGUMENT_ERROR;
    }
    return nullptr;
  }

  AvailableLocales& cache = available_locales[type];
  std::call_once(cache.initialized, [&]() {
    UEnumeration* system_locales =
        uloc_openAvailableByType(type, &cache.status);
    if (U_FAILURE(cache.status) || system_locales == nullptr) {
      return;
    }
    int32_t count = uenum_count(system_locales, &cache.status);
    if (U_SUCCESS(cache.status) && count > 0) {
      cache.storage.reserve(count);
    }
    const char* locale = nullptr;
    while (U_SUCCESS(cache.status) &&
           (locale = uenum_next(system_locales, nullptr, &cache.status)) !=
               nullptr) {
      // Chromium's ICU data filters out Lojban as a service locale. Apple's
      // system data includes it, even though most V8 Intl services have no
      // matching locale data.
      if (std::strcmp(locale, "jbo") != 0) {
        cache.storage.emplace_back(locale);
      }
    }
    uenum_close(system_locales);
    if (U_SUCCESS(cache.status)) {
      cache.pointers.reserve(cache.storage.size());
      for (const std::string& value : cache.storage) {
        cache.pointers.push_back(value.c_str());
      }
    }
  });

  if (U_FAILURE(cache.status)) {
    *status = cache.status;
    return nullptr;
  }
  return uenum_openCharStringsEnumeration(
      cache.pointers.data(), static_cast<int32_t>(cache.pointers.size()),
      status);
}
