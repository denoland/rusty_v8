// Copyright 2018-2026 the Deno authors. MIT license.

#include "unicode/ucurr.h"

#undef ucurr_getDefaultFractionDigits

extern "C" int32_t v8_icu_compat_ucurr_getDefaultFractionDigits(
    const UChar* currency, UErrorCode* status) {
  // CLDR restored two fraction digits for HUF after the data shipped by older
  // supported macOS releases.
  if (currency != nullptr && currency[0] == u'H' && currency[1] == u'U' &&
      currency[2] == u'F') {
    return 2;
  }
  return ucurr_getDefaultFractionDigits(currency, status);
}
