// Copyright 2018-2026 the Deno authors. MIT license.

#include <utility>

#include "unicode/decimfmt.h"
#include "unicode/numfmt.h"

U_NAMESPACE_BEGIN

NumberFormat::NumberFormat(UNumberFormat* formatter) : formatter_(formatter) {}

NumberFormat::~NumberFormat() { unum_close(formatter_); }

NumberFormat* NumberFormat::createInstance(const Locale& locale,
                                           UNumberFormatStyle style,
                                           UErrorCode& status) {
  UNumberFormat* formatter =
      unum_open(style, nullptr, 0, locale.getName(), nullptr, &status);
  return formatter == nullptr ? nullptr : new DecimalFormat(formatter);
}

UClassID NumberFormat::getStaticClassID() {
  static char class_id = 0;
  return &class_id;
}

UClassID NumberFormat::getDynamicClassID() const { return getStaticClassID(); }

UNumberFormat* NumberFormat::orphanUNumberFormat() {
  return std::exchange(formatter_, nullptr);
}

DecimalFormat::DecimalFormat(UNumberFormat* formatter)
    : NumberFormat(formatter) {}

DecimalFormat::~DecimalFormat() = default;

UClassID DecimalFormat::getStaticClassID() {
  static char class_id = 0;
  return &class_id;
}

UClassID DecimalFormat::getDynamicClassID() const { return getStaticClassID(); }

void DecimalFormat::setMinimumGroupingDigits(int32_t value) {
  unum_setAttribute(formatter_, UNUM_MINIMUM_GROUPING_DIGITS, value);
}

U_NAMESPACE_END
