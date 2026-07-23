// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_CALENDAR_H_
#define V8_SYSTEM_ICU_CALENDAR_H_

#include "unicode/basictz.h"
#include "unicode/locid.h"
#include "unicode/strenum.h"
#include "unicode/ucal.h"
#include "unicode/uobject.h"

U_NAMESPACE_BEGIN

class Calendar : public UObject {
 public:
  enum EDaysOfWeek {
    SUNDAY = UCAL_SUNDAY,
    MONDAY = UCAL_MONDAY,
    TUESDAY = UCAL_TUESDAY,
    WEDNESDAY = UCAL_WEDNESDAY,
    THURSDAY = UCAL_THURSDAY,
    FRIDAY = UCAL_FRIDAY,
    SATURDAY = UCAL_SATURDAY,
  };

  Calendar(UCalendar* calendar, const UnicodeString& time_zone_id);
  virtual ~Calendar();

  static Calendar* createInstance(const Locale& locale, UErrorCode& status);
  static Calendar* createInstance(TimeZone* zone_to_adopt, const Locale& locale,
                                  UErrorCode& status);
  static StringEnumeration* getKeywordValuesForLocale(const char* keyword,
                                                      const Locale& locale,
                                                      UBool commonly_used,
                                                      UErrorCode& status);

  virtual Calendar* clone() const;
  virtual UClassID getDynamicClassID() const override;
  static UClassID getStaticClassID();

  const char* getType() const;
  EDaysOfWeek getFirstDayOfWeek() const;
  UCalendarWeekdayType getDayOfWeekType(UCalendarDaysOfWeek day,
                                        UErrorCode& status) const;
  void setTime(UDate date, UErrorCode& status);
  void setTimeInMillis(UDate date, UErrorCode& status);
  void setTimeZone(const TimeZone& zone);
  const TimeZone& getTimeZone() const;

  UCalendar* toUCalendar() const { return calendar_; }

 protected:
  UCalendar* calendar_;
  BasicTimeZone time_zone_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_CALENDAR_H_
