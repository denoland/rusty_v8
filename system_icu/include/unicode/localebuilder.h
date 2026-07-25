// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_LOCALEBUILDER_H_
#define V8_SYSTEM_ICU_LOCALEBUILDER_H_

#include <string>

#include "unicode/locid.h"
#include "unicode/stringpiece.h"
#include "unicode/uobject.h"

U_NAMESPACE_BEGIN

class LocaleBuilder : public UObject {
 public:
  LocaleBuilder();
  virtual ~LocaleBuilder();

  LocaleBuilder& setLocale(const Locale& locale);
  LocaleBuilder& setLanguageTag(StringPiece tag);
  LocaleBuilder& setLanguage(StringPiece language);
  LocaleBuilder& setScript(StringPiece script);
  LocaleBuilder& setRegion(StringPiece region);
  LocaleBuilder& setVariant(StringPiece variant);
  LocaleBuilder& setUnicodeLocaleKeyword(StringPiece key, StringPiece value);
  LocaleBuilder& clearExtensions();

  Locale build(UErrorCode& status);

 private:
  void setComponent(StringPiece value, int component);

  std::string locale_id_;
  UErrorCode error_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_LOCALEBUILDER_H_
