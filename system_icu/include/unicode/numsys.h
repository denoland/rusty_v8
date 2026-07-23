// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_NUMBERING_SYSTEM_H_
#define V8_SYSTEM_ICU_NUMBERING_SYSTEM_H_

#include "unicode/locid.h"
#include "unicode/strenum.h"
#include "unicode/unumsys.h"
#include "unicode/uobject.h"

U_NAMESPACE_BEGIN

constexpr const size_t kInternalNumSysNameCapacity = 8;

class NumberingSystem : public UMemory {
 public:
  explicit NumberingSystem(UNumberingSystem* numbering_system);
  virtual ~NumberingSystem();

  static NumberingSystem* createInstance(const Locale& locale,
                                         UErrorCode& status);
  static NumberingSystem* createInstanceByName(const char* name,
                                               UErrorCode& status);
  static StringEnumeration* getAvailableNames(UErrorCode& status);

  const char* getName() const;
  UBool isAlgorithmic() const;

  UNumberingSystem* toUNumberingSystem() const { return numbering_system_; }

 private:
  UNumberingSystem* numbering_system_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_NUMBERING_SYSTEM_H_
