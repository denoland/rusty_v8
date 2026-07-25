// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_CURRENCY_UNIT_H_
#define V8_SYSTEM_ICU_CURRENCY_UNIT_H_

#include "unicode/measunit.h"
#include "unicode/unistr.h"

U_NAMESPACE_BEGIN

class CurrencyUnit : public MeasureUnit {
 public:
  CurrencyUnit();
  CurrencyUnit(ConstChar16Ptr iso_code, UErrorCode& status);
  CurrencyUnit(const CurrencyUnit& other);
  CurrencyUnit& operator=(const CurrencyUnit& other);
  virtual ~CurrencyUnit();

  const char16_t* getISOCurrency() const { return iso_code_; }

 private:
  char16_t iso_code_[4];
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_CURRENCY_UNIT_H_
