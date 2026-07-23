// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_RELATIVE_DATE_TIME_FORMATTER_H_
#define V8_SYSTEM_ICU_RELATIVE_DATE_TIME_FORMATTER_H_

#include "unicode/formattedvalue.h"
#include "unicode/locid.h"
#include "unicode/numfmt.h"
#include "unicode/uobject.h"
#include "unicode/ureldatefmt.h"

U_NAMESPACE_BEGIN

class FormattedRelativeDateTime : public UMemory, public CFormattedValue {
 public:
  FormattedRelativeDateTime();
  explicit FormattedRelativeDateTime(UFormattedRelativeDateTime* result);
  FormattedRelativeDateTime(FormattedRelativeDateTime&& other) noexcept;
  FormattedRelativeDateTime& operator=(
      FormattedRelativeDateTime&& other) noexcept;
  virtual ~FormattedRelativeDateTime() override;

 protected:
  virtual const UFormattedValue* asUFormattedValue(
      UErrorCode& status) const override;

 private:
  UFormattedRelativeDateTime* result_;
};

class RelativeDateTimeFormatter : public UObject {
 public:
  RelativeDateTimeFormatter(const Locale& locale,
                            NumberFormat* number_format_to_adopt,
                            UDateRelativeDateTimeFormatterStyle style,
                            UDisplayContext capitalization_context,
                            UErrorCode& status);
  virtual ~RelativeDateTimeFormatter();

  FormattedRelativeDateTime formatNumericToValue(double offset,
                                                 URelativeDateTimeUnit unit,
                                                 UErrorCode& status) const;
  FormattedRelativeDateTime formatToValue(double offset,
                                          URelativeDateTimeUnit unit,
                                          UErrorCode& status) const;
  UDateRelativeDateTimeFormatterStyle getFormatStyle() const;

 private:
  URelativeDateTimeFormatter* formatter_;
  UDateRelativeDateTimeFormatterStyle style_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_RELATIVE_DATE_TIME_FORMATTER_H_
