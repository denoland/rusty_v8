// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_DATE_FORMAT_SYMBOLS_H_
#define V8_SYSTEM_ICU_DATE_FORMAT_SYMBOLS_H_

#include "unicode/locid.h"
#include "unicode/unistr.h"
#include "unicode/uobject.h"

U_NAMESPACE_BEGIN

class DateFormatSymbols final : public UObject {
 public:
  DateFormatSymbols(const Locale& locale, UErrorCode& status);
  virtual ~DateFormatSymbols();

  UnicodeString& getTimeSeparatorString(UnicodeString& result) const;

 private:
  Locale locale_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_DATE_FORMAT_SYMBOLS_H_
