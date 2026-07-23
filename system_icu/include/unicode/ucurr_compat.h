// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_UCURR_COMPAT_H_
#define V8_SYSTEM_ICU_UCURR_COMPAT_H_

#include "unicode/utypes.h"

#if defined(USING_SYSTEM_ICU)
U_CAPI int32_t U_EXPORT2 v8_icu_compat_ucurr_getDefaultFractionDigits(
    const UChar* currency, UErrorCode* status);

#define ucurr_getDefaultFractionDigits \
  v8_icu_compat_ucurr_getDefaultFractionDigits
#endif

#endif  // V8_SYSTEM_ICU_UCURR_COMPAT_H_
