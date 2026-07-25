// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_NUMBER_RANGE_FORMATTER_H_
#define V8_SYSTEM_ICU_NUMBER_RANGE_FORMATTER_H_

#include <string>

#include "unicode/fmtable.h"
#include "unicode/formattedvalue.h"
#include "unicode/numberformatter.h"
#include "unicode/unumberrangeformatter.h"

U_NAMESPACE_BEGIN
namespace number {

class FormattedNumberRange : public UMemory, public CFormattedValue {
 public:
  FormattedNumberRange();
  FormattedNumberRange(UFormattedNumberRange* result, bool use_fullwidth_yen);
  FormattedNumberRange(FormattedNumberRange&& other) noexcept;
  FormattedNumberRange& operator=(FormattedNumberRange&& other) noexcept;
  virtual ~FormattedNumberRange() override;

  const UFormattedNumberRange* toUFormattedNumberRange() const {
    return result_;
  }
  virtual UnicodeString toString(UErrorCode& status) const override;

 protected:
  virtual const UFormattedValue* asUFormattedValue(
      UErrorCode& status) const override;

 private:
  UFormattedNumberRange* result_;
  bool use_fullwidth_yen_;
};

class LocalizedNumberRangeFormatter {
 public:
  LocalizedNumberRangeFormatter();
  LocalizedNumberRangeFormatter(const LocalizedNumberRangeFormatter& other);
  LocalizedNumberRangeFormatter(LocalizedNumberRangeFormatter&& other) noexcept;
  LocalizedNumberRangeFormatter& operator=(
      const LocalizedNumberRangeFormatter& other);
  LocalizedNumberRangeFormatter& operator=(
      LocalizedNumberRangeFormatter&& other) noexcept;
  ~LocalizedNumberRangeFormatter();

  FormattedNumberRange formatFormattableRange(const Formattable& first,
                                              const Formattable& second,
                                              UErrorCode& status) const;

 private:
  LocalizedNumberRangeFormatter(std::u16string skeleton, std::string locale);

  std::u16string skeleton_;
  std::string locale_;

  friend class UnlocalizedNumberRangeFormatter;
};

class UnlocalizedNumberRangeFormatter {
 public:
  UnlocalizedNumberRangeFormatter() = default;

  UnlocalizedNumberRangeFormatter numberFormatterBoth(
      UnlocalizedNumberFormatter&& formatter) const;
  LocalizedNumberRangeFormatter locale(const Locale& locale) const;

 private:
  std::u16string skeleton_;
};

class NumberRangeFormatter {};

}  // namespace number
U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_NUMBER_RANGE_FORMATTER_H_
