// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_STRINGENUMERATION_H_
#define V8_SYSTEM_ICU_STRINGENUMERATION_H_

#include "unicode/uenum.h"
#include "unicode/uobject.h"

U_NAMESPACE_BEGIN

class StringEnumeration : public UMemory {
 public:
  explicit StringEnumeration(UEnumeration* enumeration);
  virtual ~StringEnumeration();

  const char* next(int32_t* result_length, UErrorCode& status);
  int32_t count(UErrorCode& status) const;
  void reset(UErrorCode& status);

  UEnumeration* toUEnumeration() const { return enumeration_; }

 private:
  UEnumeration* enumeration_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_STRINGENUMERATION_H_
