// Copyright 2018-2026 the Deno authors. MIT license.

#include <string>
#include <utility>

#include "unicode/uniset.h"
#include "unicode/unistr.h"
#include "unicode/usetiter.h"

U_NAMESPACE_BEGIN

namespace {

UChar32 GetRangeEndpoint(const USet* set, int32_t index, bool get_end) {
  UErrorCode status = U_ZERO_ERROR;
  UChar32 start = 0;
  UChar32 end = 0;
  uset_getItem(set, index, &start, &end, nullptr, 0, &status);
  return U_SUCCESS(status) ? (get_end ? end : start) : -1;
}

int32_t GetRangeCount(const USet* set) {
  const int32_t item_count = uset_getItemCount(set);
  for (int32_t index = 0; index < item_count; ++index) {
    UErrorCode status = U_ZERO_ERROR;
    UChar32 start = 0;
    UChar32 end = 0;
    const int32_t length =
        uset_getItem(set, index, &start, &end, nullptr, 0, &status);
    if (length != 0 || status == U_BUFFER_OVERFLOW_ERROR) {
      return index;
    }
  }
  return item_count;
}

}  // namespace

UnicodeSet::UnicodeSet() : set_(uset_openEmpty()) {}

UnicodeSet::UnicodeSet(UChar32 start, UChar32 end)
    : set_(uset_open(start, end)) {}

UnicodeSet::UnicodeSet(const UnicodeString& pattern, UErrorCode& status)
    : set_(nullptr) {
  if (U_SUCCESS(status)) {
    set_ = uset_openPattern(reinterpret_cast<const UChar*>(pattern.getBuffer()),
                            pattern.length(), &status);
  }
  if (set_ == nullptr) {
    set_ = uset_openEmpty();
  }
}

UnicodeSet::UnicodeSet(const UnicodeSet& other)
    : set_(uset_clone(other.set_)) {}

UnicodeSet::~UnicodeSet() { uset_close(set_); }

UnicodeSet& UnicodeSet::operator=(const UnicodeSet& other) {
  if (this != &other) {
    USet* replacement = uset_clone(other.set_);
    uset_close(set_);
    set_ = replacement;
  }
  return *this;
}

UnicodeSet& UnicodeSet::set(UChar32 start, UChar32 end) {
  uset_set(set_, start, end);
  return *this;
}

UnicodeSet& UnicodeSet::add(UChar32 code_point) {
  uset_add(set_, code_point);
  return *this;
}

UnicodeSet& UnicodeSet::add(UChar32 start, UChar32 end) {
  uset_addRange(set_, start, end);
  return *this;
}

UnicodeSet& UnicodeSet::removeAll(const UnicodeSet& other) {
  uset_removeAll(set_, other.set_);
  return *this;
}

UnicodeSet& UnicodeSet::closeOver(int32_t attributes) {
  uset_closeOver(set_, attributes);
  return *this;
}

UnicodeSet& UnicodeSet::applyIntPropertyValue(UProperty property, int32_t value,
                                              UErrorCode& status) {
  uset_applyIntPropertyValue(set_, property, value, &status);
  return *this;
}

UnicodeSet& UnicodeSet::complement() {
  uset_complement(set_);
  return *this;
}

UnicodeSet& UnicodeSet::removeAllStrings() {
  uset_removeAllStrings(set_);
  return *this;
}

UnicodeSet& UnicodeSet::freeze() {
  uset_freeze(set_);
  return *this;
}

bool UnicodeSet::contains(UChar32 code_point) const {
  return uset_contains(set_, code_point);
}

bool UnicodeSet::isEmpty() const { return uset_isEmpty(set_); }

bool UnicodeSet::hasStrings() const {
  return uset_getItemCount(set_) > GetRangeCount(set_);
}

int32_t UnicodeSet::size() const { return uset_size(set_); }

int32_t UnicodeSet::getRangeCount() const { return GetRangeCount(set_); }

UChar32 UnicodeSet::getRangeStart(int32_t index) const {
  return GetRangeEndpoint(set_, index, false);
}

UChar32 UnicodeSet::getRangeEnd(int32_t index) const {
  return GetRangeEndpoint(set_, index, true);
}

UnicodeSetIterator::UnicodeSetIterator(const UnicodeSet& set)
    : set_(&set), string_index_(0), strings_only_(false), current_() {}

UnicodeSetIterator::~UnicodeSetIterator() = default;

UBool UnicodeSetIterator::next() {
  if (!strings_only_) {
    // V8 only uses code point ranges through UnicodeSet itself and switches
    // this iterator to strings before advancing it.
    strings_only_ = true;
  }
  const int32_t item_index = GetRangeCount(set_->set_) + string_index_++;
  if (item_index >= uset_getItemCount(set_->set_)) {
    return false;
  }
  UErrorCode status = U_ZERO_ERROR;
  UChar32 start = 0;
  UChar32 end = 0;
  int32_t length =
      uset_getItem(set_->set_, item_index, &start, &end, nullptr, 0, &status);
  if (status != U_BUFFER_OVERFLOW_ERROR || length <= 0) {
    return false;
  }
  status = U_ZERO_ERROR;
  std::u16string string(length, u'\0');
  uset_getItem(set_->set_, item_index, &start, &end,
               reinterpret_cast<UChar*>(string.data()), string.size(), &status);
  if (U_FAILURE(status)) {
    return false;
  }
  current_ = UnicodeString(string.data(), string.size());
  return true;
}

const UnicodeString& UnicodeSetIterator::getString() { return current_; }

U_NAMESPACE_END
