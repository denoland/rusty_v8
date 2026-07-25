// Copyright 2018-2026 the Deno authors. MIT license.

#include <utility>

#include "unicode/reldatefmt.h"

U_NAMESPACE_BEGIN

FormattedRelativeDateTime::FormattedRelativeDateTime() : result_(nullptr) {}

FormattedRelativeDateTime::FormattedRelativeDateTime(
    UFormattedRelativeDateTime* result)
    : result_(result) {}

FormattedRelativeDateTime::FormattedRelativeDateTime(
    FormattedRelativeDateTime&& other) noexcept
    : result_(std::exchange(other.result_, nullptr)) {}

FormattedRelativeDateTime& FormattedRelativeDateTime::operator=(
    FormattedRelativeDateTime&& other) noexcept {
  if (this != &other) {
    ureldatefmt_closeResult(result_);
    result_ = std::exchange(other.result_, nullptr);
  }
  return *this;
}

FormattedRelativeDateTime::~FormattedRelativeDateTime() {
  ureldatefmt_closeResult(result_);
}

const UFormattedValue* FormattedRelativeDateTime::asUFormattedValue(
    UErrorCode& status) const {
  if (result_ == nullptr) {
    status = U_INVALID_STATE_ERROR;
    return nullptr;
  }
  return ureldatefmt_resultAsValue(result_, &status);
}

RelativeDateTimeFormatter::RelativeDateTimeFormatter(
    const Locale& locale, NumberFormat* number_format_to_adopt,
    UDateRelativeDateTimeFormatterStyle style,
    UDisplayContext capitalization_context, UErrorCode& status)
    : formatter_(nullptr), style_(style) {
  UNumberFormat* number_format =
      number_format_to_adopt == nullptr
          ? nullptr
          : number_format_to_adopt->orphanUNumberFormat();
  delete number_format_to_adopt;
  formatter_ = ureldatefmt_open(locale.getName(), number_format, style,
                                capitalization_context, &status);
}

RelativeDateTimeFormatter::~RelativeDateTimeFormatter() {
  ureldatefmt_close(formatter_);
}

FormattedRelativeDateTime RelativeDateTimeFormatter::formatNumericToValue(
    double offset, URelativeDateTimeUnit unit, UErrorCode& status) const {
  UFormattedRelativeDateTime* result = ureldatefmt_openResult(&status);
  if (result != nullptr) {
    ureldatefmt_formatNumericToResult(formatter_, offset, unit, result,
                                      &status);
  }
  return FormattedRelativeDateTime(result);
}

FormattedRelativeDateTime RelativeDateTimeFormatter::formatToValue(
    double offset, URelativeDateTimeUnit unit, UErrorCode& status) const {
  UFormattedRelativeDateTime* result = ureldatefmt_openResult(&status);
  if (result != nullptr) {
    ureldatefmt_formatToResult(formatter_, offset, unit, result, &status);
  }
  return FormattedRelativeDateTime(result);
}

UDateRelativeDateTimeFormatterStyle RelativeDateTimeFormatter::getFormatStyle()
    const {
  return style_;
}

U_NAMESPACE_END
