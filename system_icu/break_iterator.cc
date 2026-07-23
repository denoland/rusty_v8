// Copyright 2018-2026 the Deno authors. MIT license.

#include "unicode/brkiter.h"

U_NAMESPACE_BEGIN

namespace {

BreakIterator* OpenIterator(UBreakIteratorType type, const Locale& locale,
                            UErrorCode& status) {
  UBreakIterator* iterator =
      ubrk_open(type, locale.getName(), nullptr, 0, &status);
  if (iterator == nullptr) {
    return nullptr;
  }
  return new BreakIterator(type, locale, iterator);
}

}  // namespace

BreakIterator::BreakIterator(UBreakIteratorType type, const Locale& locale,
                             UBreakIterator* iterator)
    : type_(type), locale_(locale.getName()), iterator_(iterator), text_() {}

BreakIterator::~BreakIterator() { ubrk_close(iterator_); }

BreakIterator* BreakIterator::createCharacterInstance(const Locale& locale,
                                                      UErrorCode& status) {
  return OpenIterator(UBRK_CHARACTER, locale, status);
}

BreakIterator* BreakIterator::createWordInstance(const Locale& locale,
                                                 UErrorCode& status) {
  return OpenIterator(UBRK_WORD, locale, status);
}

BreakIterator* BreakIterator::createLineInstance(const Locale& locale,
                                                 UErrorCode& status) {
  return OpenIterator(UBRK_LINE, locale, status);
}

BreakIterator* BreakIterator::createSentenceInstance(const Locale& locale,
                                                     UErrorCode& status) {
  return OpenIterator(UBRK_SENTENCE, locale, status);
}

BreakIterator* BreakIterator::clone() const {
  UErrorCode status = U_ZERO_ERROR;
  Locale locale(locale_.c_str(), "", "", "");
  BreakIterator* result = OpenIterator(type_, locale, status);
  if (result != nullptr && !text_.isEmpty()) {
    result->setText(text_);
  }
  return result;
}

void BreakIterator::setText(const UnicodeString& text) {
  text_ = text;
  UErrorCode status = U_ZERO_ERROR;
  ubrk_setText(iterator_, reinterpret_cast<const UChar*>(text_.getBuffer()),
               text_.length(), &status);
}

int32_t BreakIterator::first() { return ubrk_first(iterator_); }

int32_t BreakIterator::next() { return ubrk_next(iterator_); }

int32_t BreakIterator::current() const { return ubrk_current(iterator_); }

int32_t BreakIterator::preceding(int32_t offset) {
  return ubrk_preceding(iterator_, offset);
}

int32_t BreakIterator::following(int32_t offset) {
  return ubrk_following(iterator_, offset);
}

UBool BreakIterator::isBoundary(int32_t offset) {
  return ubrk_isBoundary(iterator_, offset);
}

int32_t BreakIterator::getRuleStatus() const {
  return ubrk_getRuleStatus(iterator_);
}

U_NAMESPACE_END
