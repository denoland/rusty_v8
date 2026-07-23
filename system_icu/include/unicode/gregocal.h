// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_GREGORIAN_CALENDAR_H_
#define V8_SYSTEM_ICU_GREGORIAN_CALENDAR_H_

#include "unicode/calendar.h"

U_NAMESPACE_BEGIN

class GregorianCalendar : public Calendar {
 public:
  GregorianCalendar(UCalendar* calendar, const UnicodeString& time_zone_id);
  virtual ~GregorianCalendar();

  GregorianCalendar* clone() const override;
  virtual UClassID getDynamicClassID() const override;
  static UClassID getStaticClassID();

  void setGregorianChange(UDate date, UErrorCode& status);
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_GREGORIAN_CALENDAR_H_
