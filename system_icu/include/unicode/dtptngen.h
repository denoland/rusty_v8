// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_DATE_TIME_PATTERN_GENERATOR_H_
#define V8_SYSTEM_ICU_DATE_TIME_PATTERN_GENERATOR_H_

#include "unicode/locid.h"
#include "unicode/udatpg.h"
#include "unicode/unistr.h"
#include "unicode/uobject.h"

U_NAMESPACE_BEGIN

class DateTimePatternGenerator : public UObject {
 public:
  static DateTimePatternGenerator* createInstance(const Locale& locale,
                                                  UErrorCode& status);
  static UnicodeString staticGetSkeleton(const UnicodeString& pattern,
                                         UErrorCode& status);

  virtual ~DateTimePatternGenerator();
  DateTimePatternGenerator* clone() const;

  UnicodeString getBestPattern(const UnicodeString& skeleton,
                               UDateTimePatternMatchOptions options,
                               UErrorCode& status);
  UDateFormatHourCycle getDefaultHourCycle(UErrorCode& status) const;
  UnicodeString getFieldDisplayName(UDateTimePatternField field,
                                    UDateTimePGDisplayWidth width) const;

 private:
  explicit DateTimePatternGenerator(UDateTimePatternGenerator* generator);

  UDateTimePatternGenerator* generator_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_DATE_TIME_PATTERN_GENERATOR_H_
