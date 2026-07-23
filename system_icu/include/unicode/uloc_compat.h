// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_ULOC_COMPAT_H_
#define V8_SYSTEM_ICU_ULOC_COMPAT_H_

#if defined(USING_SYSTEM_ICU)
U_CAPI UEnumeration* U_EXPORT2 v8_icu_compat_uloc_openAvailableByType(
    ULocAvailableType type, UErrorCode* status);

#define uloc_openAvailableByType v8_icu_compat_uloc_openAvailableByType
#endif

#endif  // V8_SYSTEM_ICU_ULOC_COMPAT_H_
