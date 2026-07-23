// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_UNICODESETITERATOR_H_
#define V8_SYSTEM_ICU_UNICODESETITERATOR_H_

#include "unicode/uniset.h"
#include "unicode/unistr.h"

U_NAMESPACE_BEGIN

class UnicodeSetIterator final {
 public:
  explicit UnicodeSetIterator(const UnicodeSet& set);
  ~UnicodeSetIterator();

  UnicodeSetIterator& skipToStrings();
  UBool next();
  const UnicodeString& getString();

 private:
  const UnicodeSet* set_;
  int32_t string_index_;
  bool strings_only_;
  UnicodeString current_;
};

inline UnicodeSetIterator& UnicodeSetIterator::skipToStrings() {
  strings_only_ = true;
  string_index_ = 0;
  return *this;
}

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_UNICODESETITERATOR_H_
