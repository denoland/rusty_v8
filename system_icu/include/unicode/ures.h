// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_URES_OVERLAY_H_
#define V8_SYSTEM_ICU_URES_OVERLAY_H_

#include_next "unicode/ures.h"

#if defined(USING_SYSTEM_ICU)
U_CAPI UResourceBundle* U_EXPORT2 v8_icu_compat_ures_open(
    const char* package_name, const char* locale, UErrorCode* status);

// Apple exposes ICU's locale resources through the default package but does
// not expose ICU's versioned "icudtNNl-coll" package name. Keep V8's resource
// validation source-compatible and translate that package in the façade.
#define ures_open v8_icu_compat_ures_open
#endif

#endif  // V8_SYSTEM_ICU_URES_OVERLAY_H_
