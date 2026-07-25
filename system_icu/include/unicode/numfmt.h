// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_NUMBER_FORMAT_H_
#define V8_SYSTEM_ICU_NUMBER_FORMAT_H_

#include "unicode/locid.h"
#include "unicode/unum.h"
#include "unicode/uobject.h"

U_NAMESPACE_BEGIN

class NumberFormat : public UObject {
 public:
  explicit NumberFormat(UNumberFormat* formatter);
  virtual ~NumberFormat();

  static NumberFormat* createInstance(const Locale& locale,
                                      UNumberFormatStyle style,
                                      UErrorCode& status);

  virtual UClassID getDynamicClassID() const override;
  static UClassID getStaticClassID();

  UNumberFormat* toUNumberFormat() const { return formatter_; }
  UNumberFormat* orphanUNumberFormat();

 protected:
  UNumberFormat* formatter_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_NUMBER_FORMAT_H_
