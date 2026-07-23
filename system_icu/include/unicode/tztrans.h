// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_TIME_ZONE_TRANSITION_H_
#define V8_SYSTEM_ICU_TIME_ZONE_TRANSITION_H_

#include "unicode/utypes.h"

U_NAMESPACE_BEGIN

class TimeZoneTransition {
 public:
  TimeZoneTransition() : time_(0) {}

  UDate getTime() const { return time_; }
  void setTime(UDate time) { time_ = time; }

 private:
  UDate time_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_TIME_ZONE_TRANSITION_H_
