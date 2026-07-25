// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_MEASURE_UNIT_H_
#define V8_SYSTEM_ICU_MEASURE_UNIT_H_

#include <string>

#include "unicode/uobject.h"
#include "unicode/utypes.h"

U_NAMESPACE_BEGIN

class MeasureUnit : public UObject {
 public:
  MeasureUnit();
  MeasureUnit(const MeasureUnit& other);
  MeasureUnit(MeasureUnit&& other) noexcept;
  MeasureUnit& operator=(const MeasureUnit& other);
  MeasureUnit& operator=(MeasureUnit&& other) noexcept;
  virtual ~MeasureUnit();

  MeasureUnit(const char* type, const char* subtype);

  virtual bool operator==(const UObject& other) const;
  bool operator!=(const UObject& other) const { return !(*this == other); }

  const char* getType() const;
  const char* getSubtype() const;
  const char* getIdentifier() const;

  static int32_t getAvailable(MeasureUnit* destination, int32_t capacity,
                              UErrorCode& status);

  static MeasureUnit getPercent();
  static MeasureUnit getPermille();
  static MeasureUnit getYear();
  static MeasureUnit getMonth();
  static MeasureUnit getWeek();
  static MeasureUnit getDay();
  static MeasureUnit getHour();
  static MeasureUnit getMinute();
  static MeasureUnit getSecond();
  static MeasureUnit getMillisecond();
  static MeasureUnit getMicrosecond();
  static MeasureUnit getNanosecond();

 private:
  std::string type_;
  std::string subtype_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_MEASURE_UNIT_H_
