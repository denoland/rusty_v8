// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_DATE_FORMAT_H_
#define V8_SYSTEM_ICU_DATE_FORMAT_H_

#include "unicode/calendar.h"
#include "unicode/fpositer.h"
#include "unicode/locid.h"
#include "unicode/udat.h"
#include "unicode/unistr.h"
#include "unicode/uobject.h"

U_NAMESPACE_BEGIN

class DateFormat : public UObject {
 public:
  enum EStyle {
    kFull = 0,
    kLong = 1,
    kMedium = 2,
    kShort = 3,
    kNone = -1,
    kDefault = kMedium,
  };

  virtual ~DateFormat();
  virtual DateFormat* clone() const;

  static DateFormat* createTimeInstance(
      EStyle style = kDefault, const Locale& locale = Locale::getDefault());
  static DateFormat* createDateInstance(
      EStyle style = kDefault, const Locale& locale = Locale::getDefault());
  static DateFormat* createDateTimeInstance(
      EStyle date_style = kDefault, EStyle time_style = kDefault,
      const Locale& locale = Locale::getDefault());
  static DateFormat* createInstanceForSkeleton(const UnicodeString& skeleton,
                                               const Locale& locale,
                                               UErrorCode& status);

  UnicodeString& format(UDate date, UnicodeString& result) const;
  UnicodeString& format(UDate date, UnicodeString& result,
                        FieldPositionIterator* positions,
                        UErrorCode& status) const;
  UnicodeString& format(Calendar& calendar, UnicodeString& result,
                        FieldPositionIterator* positions,
                        UErrorCode& status) const;

  const Calendar* getCalendar() const { return calendar_; }
  const TimeZone& getTimeZone() const { return calendar_->getTimeZone(); }
  void adoptCalendar(Calendar* calendar);
  void setTimeZone(const TimeZone& zone);

 protected:
  DateFormat(UDateFormat* format, const Locale& locale);

  UDateFormat* format_;
  Calendar* calendar_;
  Locale locale_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_DATE_FORMAT_H_
