// Copyright 2018-2026 the Deno authors. MIT license.

#include "unicode/formattedvalue.h"

U_NAMESPACE_BEGIN

ConstrainedFieldPosition::ConstrainedFieldPosition()
    : field_position_(nullptr) {
  UErrorCode status = U_ZERO_ERROR;
  field_position_ = ucfpos_open(&status);
}

ConstrainedFieldPosition::~ConstrainedFieldPosition() {
  ucfpos_close(field_position_);
}

void ConstrainedFieldPosition::reset() {
  UErrorCode status = U_ZERO_ERROR;
  ucfpos_reset(field_position_, &status);
}

void ConstrainedFieldPosition::constrainCategory(int32_t category) {
  UErrorCode status = U_ZERO_ERROR;
  ucfpos_constrainCategory(field_position_, category, &status);
}

void ConstrainedFieldPosition::constrainField(int32_t category, int32_t field) {
  UErrorCode status = U_ZERO_ERROR;
  ucfpos_constrainField(field_position_, category, field, &status);
}

int32_t ConstrainedFieldPosition::getCategory() const {
  UErrorCode status = U_ZERO_ERROR;
  return ucfpos_getCategory(field_position_, &status);
}

int32_t ConstrainedFieldPosition::getField() const {
  UErrorCode status = U_ZERO_ERROR;
  return ucfpos_getField(field_position_, &status);
}

int32_t ConstrainedFieldPosition::getStart() const {
  UErrorCode status = U_ZERO_ERROR;
  int32_t start = 0;
  int32_t limit = 0;
  ucfpos_getIndexes(field_position_, &start, &limit, &status);
  return start;
}

int32_t ConstrainedFieldPosition::getLimit() const {
  UErrorCode status = U_ZERO_ERROR;
  int32_t start = 0;
  int32_t limit = 0;
  ucfpos_getIndexes(field_position_, &start, &limit, &status);
  return limit;
}

FormattedValue::~FormattedValue() = default;

UnicodeString CFormattedValue::toString(UErrorCode& status) const {
  const UFormattedValue* value = asUFormattedValue(status);
  if (value == nullptr || U_FAILURE(status)) {
    return {};
  }
  int32_t length = 0;
  const UChar* string = ufmtval_getString(value, &length, &status);
  return U_SUCCESS(status)
             ? UnicodeString(reinterpret_cast<const char16_t*>(string), length)
             : UnicodeString();
}

UnicodeString CFormattedValue::toTempString(UErrorCode& status) const {
  return toString(status);
}

Appendable& CFormattedValue::appendTo(Appendable& appendable,
                                      UErrorCode& status) const {
  UnicodeString string = toString(status);
  if (U_SUCCESS(status) &&
      !appendable.appendString(string.getBuffer(), string.length())) {
    status = U_MEMORY_ALLOCATION_ERROR;
  }
  return appendable;
}

UBool CFormattedValue::nextPosition(ConstrainedFieldPosition& field_position,
                                    UErrorCode& status) const {
  const UFormattedValue* value = asUFormattedValue(status);
  return value != nullptr &&
         ufmtval_nextPosition(
             value, field_position.toUConstrainedFieldPosition(), &status);
}

U_NAMESPACE_END
