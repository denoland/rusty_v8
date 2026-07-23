// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_UNICODESET_H_
#define V8_SYSTEM_ICU_UNICODESET_H_

#include "unicode/uset.h"
#include "unicode/utypes.h"

U_NAMESPACE_BEGIN

class UnicodeString;

// Source-compatible subset of ICU's C++ UnicodeSet used by V8. The object is
// entirely ours; only its opaque USet handle crosses into libicucore.
class UnicodeSet final {
 public:
  UnicodeSet();
  UnicodeSet(UChar32 start, UChar32 end);
  UnicodeSet(const UnicodeString& pattern, UErrorCode& status);
  UnicodeSet(const UnicodeSet& other);
  ~UnicodeSet();

  UnicodeSet& operator=(const UnicodeSet& other);

  UnicodeSet& set(UChar32 start, UChar32 end);
  UnicodeSet& add(UChar32 code_point);
  UnicodeSet& add(UChar32 start, UChar32 end);
  UnicodeSet& removeAll(const UnicodeSet& other);
  UnicodeSet& closeOver(int32_t attributes);
  UnicodeSet& applyIntPropertyValue(UProperty property, int32_t value,
                                    UErrorCode& status);
  UnicodeSet& complement();
  UnicodeSet& removeAllStrings();
  UnicodeSet& freeze();

  bool contains(UChar32 code_point) const;
  bool isEmpty() const;
  bool hasStrings() const;
  int32_t size() const;
  int32_t getRangeCount() const;
  UChar32 getRangeStart(int32_t index) const;
  UChar32 getRangeEnd(int32_t index) const;

 private:
  friend class UnicodeSetIterator;
  USet* set_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_UNICODESET_H_
