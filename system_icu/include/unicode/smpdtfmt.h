// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_SIMPLE_DATE_FORMAT_H_
#define V8_SYSTEM_ICU_SIMPLE_DATE_FORMAT_H_

#include "unicode/datefmt.h"

U_NAMESPACE_BEGIN

class SimpleDateFormat : public DateFormat {
 public:
  SimpleDateFormat(const UnicodeString& pattern, const Locale& locale,
                   UErrorCode& status);
  virtual ~SimpleDateFormat() override;

  virtual SimpleDateFormat* clone() const override;
  UnicodeString& toPattern(UnicodeString& result) const;
  const Locale& getSmpFmtLocale() const;

 private:
  SimpleDateFormat(UDateFormat* format, const Locale& locale);

  friend class DateFormat;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_SIMPLE_DATE_FORMAT_H_
