// Copyright 2018-2026 the Deno authors. MIT license.

#include <vector>

#include "unicode/basictz.h"
#include "unicode/uloc.h"

U_NAMESPACE_BEGIN

namespace {

UnicodeString ReadDefaultTimeZone(UErrorCode& status) {
  int32_t length = ucal_getDefaultTimeZone(nullptr, 0, &status);
  if (status != U_BUFFER_OVERFLOW_ERROR && U_FAILURE(status)) {
    return {};
  }
  status = U_ZERO_ERROR;
  std::vector<UChar> buffer(length + 1);
  length = ucal_getDefaultTimeZone(buffer.data(), buffer.size(), &status);
  return U_SUCCESS(status)
             ? UnicodeString(reinterpret_cast<const char16_t*>(buffer.data()),
                             length)
             : UnicodeString();
}

UCalendarDisplayNameType DisplayNameType(UBool daylight,
                                         TimeZone::EDisplayType style) {
  if (style == TimeZone::SHORT) {
    return daylight ? UCAL_SHORT_DST : UCAL_SHORT_STANDARD;
  }
  return daylight ? UCAL_DST : UCAL_STANDARD;
}

}  // namespace

TimeZone::TimeZone(const UnicodeString& id) : id_(id) {}

TimeZone::TimeZone(const TimeZone& other) = default;

TimeZone& TimeZone::operator=(const TimeZone& other) = default;

TimeZone::~TimeZone() = default;

TimeZone* TimeZone::clone() const { return new BasicTimeZone(id_); }

bool TimeZone::operator==(const TimeZone& other) const {
  return id_ == other.id_;
}

TimeZone* TimeZone::createTimeZone(const UnicodeString& id) {
  return new BasicTimeZone(id);
}

TimeZone* TimeZone::createDefault() {
  UErrorCode status = U_ZERO_ERROR;
  UnicodeString id = ReadDefaultTimeZone(status);
  if (U_FAILURE(status)) {
    id = UnicodeString("Etc/UTC", -1, US_INV);
  }
  return new BasicTimeZone(id);
}

TimeZone* TimeZone::detectHostTimeZone() { return createDefault(); }

void TimeZone::adoptDefault(TimeZone* zone) {
  if (zone == nullptr) {
    return;
  }
  UErrorCode status = U_ZERO_ERROR;
  ucal_setDefaultTimeZone(
      reinterpret_cast<const UChar*>(zone->id_.getTerminatedBuffer()), &status);
  delete zone;
}

const TimeZone* TimeZone::getGMT() {
  static const BasicTimeZone gmt(UnicodeString("Etc/GMT", -1, US_INV));
  return &gmt;
}

StringEnumeration* TimeZone::createEnumeration() {
  UErrorCode status = U_ZERO_ERROR;
  UEnumeration* enumeration = ucal_openTimeZones(&status);
  return enumeration == nullptr ? nullptr : new StringEnumeration(enumeration);
}

StringEnumeration* TimeZone::createTimeZoneIDEnumeration(
    USystemTimeZoneType zone_type, const char* region,
    const int32_t* raw_offset, UErrorCode& status) {
  UEnumeration* enumeration =
      ucal_openTimeZoneIDEnumeration(zone_type, region, raw_offset, &status);
  return enumeration == nullptr ? nullptr : new StringEnumeration(enumeration);
}

UnicodeString& TimeZone::getCanonicalID(const UnicodeString& id,
                                        UnicodeString& canonical,
                                        UErrorCode& status) {
  UBool is_system_id = false;
  std::vector<UChar> buffer(128);
  int32_t length = ucal_getCanonicalTimeZoneID(
      reinterpret_cast<const UChar*>(id.getBuffer()), id.length(),
      buffer.data(), buffer.size(), &is_system_id, &status);
  if (status == U_BUFFER_OVERFLOW_ERROR) {
    status = U_ZERO_ERROR;
    buffer.resize(length + 1);
    length = ucal_getCanonicalTimeZoneID(
        reinterpret_cast<const UChar*>(id.getBuffer()), id.length(),
        buffer.data(), buffer.size(), &is_system_id, &status);
  }
  if (U_SUCCESS(status)) {
    canonical.remove();
    canonical.append(reinterpret_cast<const char16_t*>(buffer.data()), length);
  }
  return canonical;
}

UnicodeString& TimeZone::getID(UnicodeString& id) const {
  id = id_;
  return id;
}

UCalendar* TimeZone::openCalendar(UErrorCode& status) const {
  return ucal_open(reinterpret_cast<const UChar*>(id_.getBuffer()),
                   id_.length(), uloc_getDefault(), UCAL_DEFAULT, &status);
}

void TimeZone::getDisplayName(UBool daylight, EDisplayType style,
                              UnicodeString& result) const {
  UErrorCode status = U_ZERO_ERROR;
  UCalendar* calendar = openCalendar(status);
  if (calendar == nullptr) {
    return;
  }
  int32_t length =
      ucal_getTimeZoneDisplayName(calendar, DisplayNameType(daylight, style),
                                  uloc_getDefault(), nullptr, 0, &status);
  if (status == U_BUFFER_OVERFLOW_ERROR) {
    status = U_ZERO_ERROR;
    std::vector<UChar> buffer(length + 1);
    length = ucal_getTimeZoneDisplayName(
        calendar, DisplayNameType(daylight, style), uloc_getDefault(),
        buffer.data(), buffer.size(), &status);
    if (U_SUCCESS(status)) {
      result.remove();
      result.append(reinterpret_cast<const char16_t*>(buffer.data()), length);
    }
  }
  ucal_close(calendar);
}

void TimeZone::getOffset(UDate date, UBool local, int32_t& raw_offset,
                         int32_t& dst_offset, UErrorCode& status) const {
  UCalendar* calendar = openCalendar(status);
  if (calendar == nullptr) {
    return;
  }
  ucal_setMillis(calendar, date, &status);
  if (local && U_SUCCESS(status)) {
    ucal_getTimeZoneOffsetFromLocal(calendar, UCAL_TZ_LOCAL_FORMER,
                                    UCAL_TZ_LOCAL_FORMER, &raw_offset,
                                    &dst_offset, &status);
  } else if (U_SUCCESS(status)) {
    raw_offset = ucal_get(calendar, UCAL_ZONE_OFFSET, &status);
    dst_offset = ucal_get(calendar, UCAL_DST_OFFSET, &status);
  }
  ucal_close(calendar);
}

BasicTimeZone::BasicTimeZone(const UnicodeString& id) : TimeZone(id) {}

BasicTimeZone::BasicTimeZone(const BasicTimeZone& other) = default;

BasicTimeZone::~BasicTimeZone() = default;

BasicTimeZone* BasicTimeZone::clone() const { return new BasicTimeZone(*this); }

void BasicTimeZone::getOffsetFromLocal(
    UDate date, UTimeZoneLocalOption non_existing_time_opt,
    UTimeZoneLocalOption duplicated_time_opt, int32_t& raw_offset,
    int32_t& dst_offset, UErrorCode& status) const {
  UCalendar* calendar = openCalendar(status);
  if (calendar == nullptr) {
    return;
  }
  ucal_setMillis(calendar, date, &status);
  if (U_SUCCESS(status)) {
    ucal_getTimeZoneOffsetFromLocal(calendar, non_existing_time_opt,
                                    duplicated_time_opt, &raw_offset,
                                    &dst_offset, &status);
  }
  ucal_close(calendar);
}

UBool BasicTimeZone::getNextTransition(UDate base, UBool inclusive,
                                       TimeZoneTransition& result) const {
  UErrorCode status = U_ZERO_ERROR;
  UCalendar* calendar = openCalendar(status);
  if (calendar == nullptr) {
    return false;
  }
  ucal_setMillis(calendar, base, &status);
  UDate transition = 0;
  UBool found =
      U_SUCCESS(status) && ucal_getTimeZoneTransitionDate(
                               calendar,
                               inclusive ? UCAL_TZ_TRANSITION_NEXT_INCLUSIVE
                                         : UCAL_TZ_TRANSITION_NEXT,
                               &transition, &status);
  ucal_close(calendar);
  if (found && U_SUCCESS(status)) {
    result.setTime(transition);
    return true;
  }
  return false;
}

UBool BasicTimeZone::getPreviousTransition(UDate base, UBool inclusive,
                                           TimeZoneTransition& result) const {
  UErrorCode status = U_ZERO_ERROR;
  UCalendar* calendar = openCalendar(status);
  if (calendar == nullptr) {
    return false;
  }
  ucal_setMillis(calendar, base, &status);
  UDate transition = 0;
  UBool found =
      U_SUCCESS(status) && ucal_getTimeZoneTransitionDate(
                               calendar,
                               inclusive ? UCAL_TZ_TRANSITION_PREVIOUS_INCLUSIVE
                                         : UCAL_TZ_TRANSITION_PREVIOUS,
                               &transition, &status);
  ucal_close(calendar);
  if (found && U_SUCCESS(status)) {
    result.setTime(transition);
    return true;
  }
  return false;
}

U_NAMESPACE_END
