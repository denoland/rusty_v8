// Copyright 2018-2026 the Deno authors. MIT license.

#include <cstring>

#include "unicode/ures.h"

#undef ures_open

extern "C" UResourceBundle* v8_icu_compat_ures_open(const char* package_name,
                                                    const char* locale,
                                                    UErrorCode* status) {
  if (package_name != nullptr &&
      std::strstr(package_name, "-coll") != nullptr) {
    package_name = nullptr;
  }
  return ures_open(package_name, locale, status);
}
