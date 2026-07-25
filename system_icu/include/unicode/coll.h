// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_COLLATOR_H_
#define V8_SYSTEM_ICU_COLLATOR_H_

#include "unicode/locid.h"
#include "unicode/strenum.h"
#include "unicode/stringpiece.h"
#include "unicode/ucol.h"
#include "unicode/unistr.h"
#include "unicode/uobject.h"

U_NAMESPACE_BEGIN

class Collator : public UMemory {
 public:
  enum EComparisonResult {
    LESS = UCOL_LESS,
    EQUAL = UCOL_EQUAL,
    GREATER = UCOL_GREATER,
  };

  enum ECollationStrength {
    PRIMARY = UCOL_PRIMARY,
    SECONDARY = UCOL_SECONDARY,
    TERTIARY = UCOL_TERTIARY,
    QUATERNARY = UCOL_QUATERNARY,
    IDENTICAL = UCOL_IDENTICAL,
  };

  explicit Collator(UCollator* collator);
  virtual ~Collator();

  static Collator* createInstance(const Locale& locale, UErrorCode& status);
  static const Locale* getAvailableLocales(int32_t& count);
  static StringEnumeration* getKeywordValues(const char* keyword,
                                             UErrorCode& status);
  static StringEnumeration* getKeywordValuesForLocale(const char* keyword,
                                                      const Locale& locale,
                                                      UBool commonly_used,
                                                      UErrorCode& status);

  UColAttributeValue getAttribute(UColAttribute attribute,
                                  UErrorCode& status) const;
  void setAttribute(UColAttribute attribute, UColAttributeValue value,
                    UErrorCode& status);
  void setStrength(ECollationStrength strength);
  ECollationStrength getStrength() const;
  const char* getLocale(ULocDataLocaleType type, UErrorCode& status) const;

  UCollationResult compare(const UnicodeString& left,
                           const UnicodeString& right,
                           UErrorCode& status) const;
  UCollationResult compareUTF8(StringPiece left, StringPiece right,
                               UErrorCode& status) const;

  UCollator* toUCollator() const { return collator_; }

 private:
  UCollator* collator_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_COLLATOR_H_
