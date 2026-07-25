// Copyright 2018-2026 the Deno authors. MIT license.

#include "unicode/calendar.h"

#include <cstring>
#include <vector>

#include "unicode/gregocal.h"

U_NAMESPACE_BEGIN

namespace {

UnicodeString CalendarTimeZoneID(const UCalendar* calendar,
                                 UErrorCode& status) {
  int32_t length = ucal_getTimeZoneID(calendar, nullptr, 0, &status);
  if (status != U_BUFFER_OVERFLOW_ERROR && U_FAILURE(status)) {
    return {};
  }
  status = U_ZERO_ERROR;
  std::vector<UChar> buffer(length + 1);
  length = ucal_getTimeZoneID(calendar, buffer.data(), buffer.size(), &status);
  return U_SUCCESS(status)
             ? UnicodeString(reinterpret_cast<const char16_t*>(buffer.data()),
                             length)
             : UnicodeString();
}

Calendar* WrapCalendar(UCalendar* calendar, const UnicodeString& time_zone_id) {
  if (calendar == nullptr) {
    return nullptr;
  }
  UErrorCode status = U_ZERO_ERROR;
  const char* type = ucal_getType(calendar, &status);
  if (type != nullptr && (std::strcmp(type, "gregorian") == 0 ||
                          std::strcmp(type, "iso8601") == 0)) {
    return new GregorianCalendar(calendar, time_zone_id);
  }
  return new Calendar(calendar, time_zone_id);
}

}  // namespace

Calendar::Calendar(UCalendar* calendar, const UnicodeString& time_zone_id)
    : calendar_(calendar), time_zone_(time_zone_id) {}

Calendar::~Calendar() { ucal_close(calendar_); }

Calendar* Calendar::createInstance(const Locale& locale, UErrorCode& status) {
  UCalendar* calendar =
      ucal_open(nullptr, 0, locale.getName(), UCAL_DEFAULT, &status);
  if (calendar == nullptr) {
    return nullptr;
  }
  UnicodeString id = CalendarTimeZoneID(calendar, status);
  if (U_FAILURE(status)) {
    ucal_close(calendar);
    return nullptr;
  }
  return WrapCalendar(calendar, id);
}

Calendar* Calendar::createInstance(TimeZone* zone_to_adopt,
                                   const Locale& locale, UErrorCode& status) {
  UnicodeString id;
  if (zone_to_adopt != nullptr) {
    zone_to_adopt->getID(id);
  }
  UCalendar* calendar = ucal_open(
      id.isEmpty() ? nullptr : reinterpret_cast<const UChar*>(id.getBuffer()),
      id.length(), locale.getName(), UCAL_DEFAULT, &status);
  delete zone_to_adopt;
  return calendar == nullptr ? nullptr : WrapCalendar(calendar, id);
}

StringEnumeration* Calendar::getKeywordValuesForLocale(const char* keyword,
                                                       const Locale& locale,
                                                       UBool commonly_used,
                                                       UErrorCode& status) {
  UEnumeration* values = ucal_getKeywordValuesForLocale(
      keyword, locale.getName(), commonly_used, &status);
  return values == nullptr ? nullptr : new StringEnumeration(values);
}

Calendar* Calendar::clone() const {
  UErrorCode status = U_ZERO_ERROR;
  UCalendar* copy = ucal_clone(calendar_, &status);
  UnicodeString id;
  time_zone_.getID(id);
  return U_FAILURE(status) ? nullptr : WrapCalendar(copy, id);
}

UClassID Calendar::getStaticClassID() {
  static char class_id = 0;
  return &class_id;
}

UClassID Calendar::getDynamicClassID() const { return getStaticClassID(); }

const char* Calendar::getType() const {
  UErrorCode status = U_ZERO_ERROR;
  return ucal_getType(calendar_, &status);
}

Calendar::EDaysOfWeek Calendar::getFirstDayOfWeek() const {
  return static_cast<EDaysOfWeek>(
      ucal_getAttribute(calendar_, UCAL_FIRST_DAY_OF_WEEK));
}

UCalendarWeekdayType Calendar::getDayOfWeekType(UCalendarDaysOfWeek day,
                                                UErrorCode& status) const {
  return ucal_getDayOfWeekType(calendar_, day, &status);
}

void Calendar::setTime(UDate date, UErrorCode& status) {
  ucal_setMillis(calendar_, date, &status);
}

void Calendar::setTimeInMillis(UDate date, UErrorCode& status) {
  setTime(date, status);
}

void Calendar::setTimeZone(const TimeZone& zone) {
  UnicodeString id;
  zone.getID(id);
  UErrorCode status = U_ZERO_ERROR;
  ucal_setTimeZone(calendar_, reinterpret_cast<const UChar*>(id.getBuffer()),
                   id.length(), &status);
  if (U_SUCCESS(status)) {
    time_zone_ = BasicTimeZone(id);
  }
}

const TimeZone& Calendar::getTimeZone() const { return time_zone_; }

GregorianCalendar::GregorianCalendar(UCalendar* calendar,
                                     const UnicodeString& time_zone_id)
    : Calendar(calendar, time_zone_id) {}

GregorianCalendar::~GregorianCalendar() = default;

GregorianCalendar* GregorianCalendar::clone() const {
  UErrorCode status = U_ZERO_ERROR;
  UCalendar* copy = ucal_clone(calendar_, &status);
  if (U_FAILURE(status)) {
    return nullptr;
  }
  UnicodeString id;
  time_zone_.getID(id);
  return new GregorianCalendar(copy, id);
}

UClassID GregorianCalendar::getStaticClassID() {
  static char class_id = 0;
  return &class_id;
}

UClassID GregorianCalendar::getDynamicClassID() const {
  return getStaticClassID();
}

void GregorianCalendar::setGregorianChange(UDate date, UErrorCode& status) {
  ucal_setGregorianChange(calendar_, date, &status);
}

U_NAMESPACE_END
