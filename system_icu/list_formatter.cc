// Copyright 2018-2026 the Deno authors. MIT license.

#include <utility>
#include <vector>

#include "unicode/listformatter.h"

U_NAMESPACE_BEGIN

FormattedList::FormattedList() : result_(nullptr) {}

FormattedList::FormattedList(UFormattedList* result) : result_(result) {}

FormattedList::FormattedList(FormattedList&& other) noexcept
    : result_(std::exchange(other.result_, nullptr)) {}

FormattedList& FormattedList::operator=(FormattedList&& other) noexcept {
  if (this != &other) {
    ulistfmt_closeResult(result_);
    result_ = std::exchange(other.result_, nullptr);
  }
  return *this;
}

FormattedList::~FormattedList() { ulistfmt_closeResult(result_); }

const UFormattedValue* FormattedList::asUFormattedValue(
    UErrorCode& status) const {
  if (result_ == nullptr) {
    status = U_INVALID_STATE_ERROR;
    return nullptr;
  }
  return ulistfmt_resultAsValue(result_, &status);
}

ListFormatter::ListFormatter(UListFormatter* formatter)
    : formatter_(formatter) {}

ListFormatter::~ListFormatter() { ulistfmt_close(formatter_); }

ListFormatter* ListFormatter::createInstance(const Locale& locale,
                                             UListFormatterType type,
                                             UListFormatterWidth width,
                                             UErrorCode& status) {
  UListFormatter* formatter =
      ulistfmt_openForType(locale.getName(), type, width, &status);
  return formatter == nullptr ? nullptr : new ListFormatter(formatter);
}

FormattedList ListFormatter::formatStringsToValue(const UnicodeString items[],
                                                  int32_t item_count,
                                                  UErrorCode& status) const {
  UFormattedList* result = ulistfmt_openResult(&status);
  if (result == nullptr) {
    return {};
  }

  std::vector<const UChar*> strings(item_count);
  std::vector<int32_t> lengths(item_count);
  for (int32_t index = 0; index < item_count; ++index) {
    strings[index] = reinterpret_cast<const UChar*>(items[index].getBuffer());
    lengths[index] = items[index].length();
  }
  ulistfmt_formatStringsToResult(formatter_, strings.data(), lengths.data(),
                                 item_count, result, &status);
  return FormattedList(result);
}

U_NAMESPACE_END
