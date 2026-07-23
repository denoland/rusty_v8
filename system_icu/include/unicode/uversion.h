// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_UVERSION_OVERLAY_H_
#define V8_SYSTEM_ICU_UVERSION_OVERLAY_H_

#include_next "unicode/uversion.h"

#if defined(__cplusplus) && defined(USING_SYSTEM_ICU)
// Keep all C++ façade symbols in a namespace that cannot bind to Apple's
// private ICU C++ ABI. The C entry points remain unsuffixed because
// U_DISABLE_RENAMING is set by the system ICU build config.
#undef U_ICU_NAMESPACE
#define U_ICU_NAMESPACE v8_icu_compat

#undef U_NAMESPACE_BEGIN
#undef U_NAMESPACE_END
#undef U_NAMESPACE_USE
#undef U_NAMESPACE_QUALIFIER
#define U_NAMESPACE_BEGIN namespace U_ICU_NAMESPACE {
#define U_NAMESPACE_END }
#define U_NAMESPACE_USE using namespace U_ICU_NAMESPACE;
#define U_NAMESPACE_QUALIFIER U_ICU_NAMESPACE::

namespace U_ICU_NAMESPACE {}

// V8 consistently spells the public namespace `icu`. A macro is necessary
// here because the upstream header has already declared an empty namespace
// with that name, which prevents creating a namespace alias.
#define icu U_ICU_NAMESPACE

#ifndef U_FORCE_HIDE_DRAFT_API
namespace U_HEADER_ONLY_NAMESPACE {}
#endif
#endif

#endif  // V8_SYSTEM_ICU_UVERSION_OVERLAY_H_
