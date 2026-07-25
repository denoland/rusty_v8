// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_DATE_INTERVAL_FORMAT_H_
#define V8_SYSTEM_ICU_DATE_INTERVAL_FORMAT_H_

#include <string>

#include "unicode/calendar.h"
#include "unicode/formattedvalue.h"
#include "unicode/locid.h"
#include "unicode/udateintervalformat.h"
#include "unicode/uobject.h"

U_NAMESPACE_BEGIN

class FormattedDateInterval : public UMemory, public CFormattedValue {
 public:
  FormattedDateInterval();
  explicit FormattedDateInterval(UFormattedDateInterval* result);
  FormattedDateInterval(FormattedDateInterval&& other) noexcept;
  FormattedDateInterval& operator=(FormattedDateInterval&& other) noexcept;
  virtual ~FormattedDateInterval() override;

 protected:
  virtual const UFormattedValue* asUFormattedValue(
      UErrorCode& status) const override;

 private:
  UFormattedDateInterval* result_;
};

class DateIntervalFormat : public UObject {
 public:
  static DateIntervalFormat* createInstance(const UnicodeString& skeleton,
                                            const Locale& locale,
                                            UErrorCode& status);
  virtual ~DateIntervalFormat();

  DateIntervalFormat* clone() const;
  void setTimeZone(const TimeZone& zone);
  FormattedDateInterval formatToValue(Calendar& from, Calendar& to,
                                      UErrorCode& status) const;

 private:
  DateIntervalFormat(std::u16string skeleton, std::string locale,
                     std::u16string time_zone, UErrorCode& status);
  void reopen(UErrorCode& status);

  UDateIntervalFormat* format_;
  std::u16string skeleton_;
  std::string locale_;
  std::u16string time_zone_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_DATE_INTERVAL_FORMAT_H_
