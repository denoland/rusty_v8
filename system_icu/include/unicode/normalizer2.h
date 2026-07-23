// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_NORMALIZER2_H_
#define V8_SYSTEM_ICU_NORMALIZER2_H_

#include "unicode/unistr.h"
#include "unicode/unorm2.h"
#include "unicode/uobject.h"

U_NAMESPACE_BEGIN

class Normalizer2 : public UMemory {
 public:
  static const Normalizer2* getInstance(const char* package_name,
                                        const char* name,
                                        UNormalization2Mode mode,
                                        UErrorCode& status);

  int32_t spanQuickCheckYes(const UnicodeString& input,
                            UErrorCode& status) const;
  UnicodeString& normalizeSecondAndAppend(UnicodeString& first,
                                          const UnicodeString& second,
                                          UErrorCode& status) const;

 private:
  explicit Normalizer2(const UNormalizer2* normalizer);

  const UNormalizer2* normalizer_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_NORMALIZER2_H_
