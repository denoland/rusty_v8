// Copyright 2018-2026 the Deno authors. MIT license.

#include <vector>

#include "unicode/coll.h"

U_NAMESPACE_BEGIN

Collator::Collator(UCollator* collator) : collator_(collator) {}

Collator::~Collator() { ucol_close(collator_); }

Collator* Collator::createInstance(const Locale& locale, UErrorCode& status) {
  UCollator* collator = ucol_open(locale.getName(), &status);
  return collator == nullptr ? nullptr : new Collator(collator);
}

const Locale* Collator::getAvailableLocales(int32_t& count) {
  static const std::vector<Locale> locales = [] {
    std::vector<Locale> result;
    const int32_t available = ucol_countAvailable();
    result.reserve(available);
    for (int32_t index = 0; index < available; ++index) {
      result.emplace_back(ucol_getAvailable(index), "", "", "");
    }
    return result;
  }();
  count = locales.size();
  return locales.data();
}

StringEnumeration* Collator::getKeywordValues(const char* keyword,
                                              UErrorCode& status) {
  UEnumeration* values = ucol_getKeywordValues(keyword, &status);
  return values == nullptr ? nullptr : new StringEnumeration(values);
}

StringEnumeration* Collator::getKeywordValuesForLocale(const char* keyword,
                                                       const Locale& locale,
                                                       UBool commonly_used,
                                                       UErrorCode& status) {
  UEnumeration* values = ucol_getKeywordValuesForLocale(
      keyword, locale.getName(), commonly_used, &status);
  return values == nullptr ? nullptr : new StringEnumeration(values);
}

UColAttributeValue Collator::getAttribute(UColAttribute attribute,
                                          UErrorCode& status) const {
  return ucol_getAttribute(collator_, attribute, &status);
}

void Collator::setAttribute(UColAttribute attribute, UColAttributeValue value,
                            UErrorCode& status) {
  ucol_setAttribute(collator_, attribute, value, &status);
}

void Collator::setStrength(ECollationStrength strength) {
  ucol_setStrength(collator_, static_cast<UCollationStrength>(strength));
}

Collator::ECollationStrength Collator::getStrength() const {
  return static_cast<ECollationStrength>(ucol_getStrength(collator_));
}

const char* Collator::getLocale(ULocDataLocaleType type,
                                UErrorCode& status) const {
  return ucol_getLocaleByType(collator_, type, &status);
}

UCollationResult Collator::compare(const UnicodeString& left,
                                   const UnicodeString& right,
                                   UErrorCode& status) const {
  if (U_FAILURE(status)) {
    return UCOL_EQUAL;
  }
  return ucol_strcoll(
      collator_, reinterpret_cast<const UChar*>(left.getBuffer()),
      left.length(), reinterpret_cast<const UChar*>(right.getBuffer()),
      right.length());
}

UCollationResult Collator::compareUTF8(StringPiece left, StringPiece right,
                                       UErrorCode& status) const {
  return ucol_strcollUTF8(collator_, left.data(), left.length(), right.data(),
                          right.length(), &status);
}

U_NAMESPACE_END
