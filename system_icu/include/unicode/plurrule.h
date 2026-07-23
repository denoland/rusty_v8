// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_PLURAL_RULES_H_
#define V8_SYSTEM_ICU_PLURAL_RULES_H_

#include "unicode/locid.h"
#include "unicode/numberformatter.h"
#include "unicode/numberrangeformatter.h"
#include "unicode/strenum.h"
#include "unicode/uobject.h"
#include "unicode/upluralrules.h"

U_NAMESPACE_BEGIN

class PluralRules : public UObject {
 public:
  static PluralRules* forLocale(const Locale& locale, UPluralType type,
                                UErrorCode& status);
  static StringEnumeration* getAvailableLocales(UErrorCode& status);

  virtual ~PluralRules();

  UnicodeString select(const number::FormattedNumber& number,
                       UErrorCode& status) const;
  UnicodeString select(const number::FormattedNumberRange& range,
                       UErrorCode& status) const;
  StringEnumeration* getKeywords(UErrorCode& status) const;

 private:
  explicit PluralRules(UPluralRules* rules);

  UPluralRules* rules_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_PLURAL_RULES_H_
