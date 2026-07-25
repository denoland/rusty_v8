// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_LOCALE_DISPLAY_NAMES_H_
#define V8_SYSTEM_ICU_LOCALE_DISPLAY_NAMES_H_

#include "unicode/locid.h"
#include "unicode/uldnames.h"
#include "unicode/unistr.h"
#include "unicode/uobject.h"

U_NAMESPACE_BEGIN

class LocaleDisplayNames : public UObject {
 public:
  static LocaleDisplayNames* createInstance(const Locale& locale,
                                            UDisplayContext* contexts,
                                            int32_t length);
  virtual ~LocaleDisplayNames();

  const Locale& getLocale() const { return locale_; }
  UnicodeString& localeDisplayName(const char* locale,
                                   UnicodeString& result) const;
  UnicodeString& scriptDisplayName(const char* script,
                                   UnicodeString& result) const;
  UnicodeString& regionDisplayName(const char* region,
                                   UnicodeString& result) const;
  UnicodeString& keyValueDisplayName(const char* key, const char* value,
                                     UnicodeString& result) const;

 private:
  LocaleDisplayNames(ULocaleDisplayNames* names, const Locale& locale);

  ULocaleDisplayNames* names_;
  Locale locale_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_LOCALE_DISPLAY_NAMES_H_
