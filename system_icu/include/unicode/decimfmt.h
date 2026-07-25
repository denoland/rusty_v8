// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_DECIMAL_FORMAT_H_
#define V8_SYSTEM_ICU_DECIMAL_FORMAT_H_

#include "unicode/numfmt.h"

U_NAMESPACE_BEGIN

class DecimalFormat : public NumberFormat {
 public:
  explicit DecimalFormat(UNumberFormat* formatter);
  virtual ~DecimalFormat();

  virtual UClassID getDynamicClassID() const override;
  static UClassID getStaticClassID();

  void setMinimumGroupingDigits(int32_t value);
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_DECIMAL_FORMAT_H_
