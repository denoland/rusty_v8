// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_BREAKITERATOR_H_
#define V8_SYSTEM_ICU_BREAKITERATOR_H_

#include <string>

#include "unicode/locid.h"
#include "unicode/ubrk.h"
#include "unicode/unistr.h"
#include "unicode/uobject.h"

U_NAMESPACE_BEGIN

class BreakIterator : public UMemory {
 public:
  static constexpr int32_t DONE = UBRK_DONE;

  virtual ~BreakIterator();

  BreakIterator(UBreakIteratorType type, const Locale& locale,
                UBreakIterator* iterator);

  static BreakIterator* createCharacterInstance(const Locale& locale,
                                                UErrorCode& status);
  static BreakIterator* createWordInstance(const Locale& locale,
                                           UErrorCode& status);
  static BreakIterator* createLineInstance(const Locale& locale,
                                           UErrorCode& status);
  static BreakIterator* createSentenceInstance(const Locale& locale,
                                               UErrorCode& status);

  BreakIterator* clone() const;
  void setText(const UnicodeString& text);

  int32_t first();
  int32_t next();
  int32_t current() const;
  int32_t preceding(int32_t offset);
  int32_t following(int32_t offset);
  UBool isBoundary(int32_t offset);
  int32_t getRuleStatus() const;

  class TextView {
   public:
    explicit TextView(int32_t length) : length_(length) {}
    int32_t getLength() const { return length_; }

   private:
    int32_t length_;
  };

  TextView getText() const { return TextView(text_.length()); }

 private:
  UBreakIteratorType type_;
  std::string locale_;
  UBreakIterator* iterator_;
  UnicodeString text_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_BREAKITERATOR_H_
