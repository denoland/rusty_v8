// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_BASIC_TIME_ZONE_H_
#define V8_SYSTEM_ICU_BASIC_TIME_ZONE_H_

#include "unicode/timezone.h"
#include "unicode/tztrans.h"

U_NAMESPACE_BEGIN

class BasicTimeZone : public TimeZone {
 public:
  explicit BasicTimeZone(const UnicodeString& id);
  BasicTimeZone(const BasicTimeZone& other);
  virtual ~BasicTimeZone();

  virtual BasicTimeZone* clone() const;

  void getOffsetFromLocal(UDate date,
                          UTimeZoneLocalOption non_existing_time_opt,
                          UTimeZoneLocalOption duplicated_time_opt,
                          int32_t& raw_offset, int32_t& dst_offset,
                          UErrorCode& status) const;
  UBool getNextTransition(UDate base, UBool inclusive,
                          TimeZoneTransition& result) const;
  UBool getPreviousTransition(UDate base, UBool inclusive,
                              TimeZoneTransition& result) const;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_BASIC_TIME_ZONE_H_
