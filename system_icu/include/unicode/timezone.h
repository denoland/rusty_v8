// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_TIME_ZONE_H_
#define V8_SYSTEM_ICU_TIME_ZONE_H_

#include "unicode/strenum.h"
#include "unicode/ucal.h"
#include "unicode/unistr.h"
#include "unicode/uobject.h"

U_NAMESPACE_BEGIN

class TimeZone : public UObject {
 public:
  enum EDisplayType {
    SHORT = 1,
    LONG = 2,
    SHORT_GENERIC = 3,
    LONG_GENERIC = 4,
    SHORT_GMT = 5,
    LONG_GMT = 6,
    SHORT_COMMONLY_USED = 7,
    GENERIC_LOCATION = 8,
  };

  explicit TimeZone(const UnicodeString& id);
  TimeZone(const TimeZone& other);
  TimeZone& operator=(const TimeZone& other);
  virtual ~TimeZone();

  virtual TimeZone* clone() const;
  virtual bool operator==(const TimeZone& other) const;

  static TimeZone* createTimeZone(const UnicodeString& id);
  static TimeZone* createDefault();
  static TimeZone* detectHostTimeZone();
  static void adoptDefault(TimeZone* zone);
  static const TimeZone* getGMT();

  static StringEnumeration* createEnumeration();
  static StringEnumeration* createTimeZoneIDEnumeration(
      USystemTimeZoneType zone_type, const char* region,
      const int32_t* raw_offset, UErrorCode& status);
  static UnicodeString& getCanonicalID(const UnicodeString& id,
                                       UnicodeString& canonical,
                                       UErrorCode& status);

  UnicodeString& getID(UnicodeString& id) const;
  void getDisplayName(UBool daylight, EDisplayType style,
                      UnicodeString& result) const;
  virtual void getOffset(UDate date, UBool local, int32_t& raw_offset,
                         int32_t& dst_offset, UErrorCode& status) const;

 protected:
  UCalendar* openCalendar(UErrorCode& status) const;

  UnicodeString id_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_TIME_ZONE_H_
