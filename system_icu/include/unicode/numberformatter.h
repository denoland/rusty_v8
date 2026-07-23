// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_NUMBER_FORMATTER_H_
#define V8_SYSTEM_ICU_NUMBER_FORMATTER_H_

#include <memory>
#include <string>
#include <utility>

#include "unicode/formattedvalue.h"
#include "unicode/locid.h"
#include "unicode/measunit.h"
#include "unicode/numsys.h"
#include "unicode/stringpiece.h"
#include "unicode/uformattednumber.h"
#include "unicode/unum.h"
#include "unicode/unumberformatter.h"

U_NAMESPACE_BEGIN
namespace number {

namespace impl {

struct NumberFormatterOptions {
  std::string raw_skeleton;
  std::string locale;
  std::string numbering_system;
  std::string unit_type;
  std::string unit;
  std::string per_unit;
  std::string precision;
  std::string notation;
  std::string rounding_mode;
  std::string grouping;
  std::string sign;
  int32_t integer_width = 0;
  int32_t unit_width = UNUM_UNIT_WIDTH_SHORT;
  int32_t scale = 0;
  bool has_unit_width = false;
  bool has_scale = false;
};

struct NumberFormatterState;

UnicodeString BuildSkeleton(const NumberFormatterOptions& options,
                            UErrorCode& status);
const char* RoundingModeStem(UNumberFormatRoundingMode mode);
const char* GroupingStem(UNumberGroupingStrategy grouping);
const char* SignStem(UNumberSignDisplay sign);
const char* UnitWidthStem(UNumberUnitWidth width);

}  // namespace impl

class Notation {
 public:
  static Notation simple();
  static Notation scientific();
  static Notation engineering();
  static Notation compactShort();
  static Notation compactLong();

  const std::string& stem() const { return stem_; }

 private:
  explicit Notation(const char* stem) : stem_(stem) {}
  std::string stem_;
};

class Precision {
 public:
  static Precision unlimited();
  static class FractionPrecision minMaxFraction(int32_t minimum,
                                                int32_t maximum);
  static Precision minMaxSignificantDigits(int32_t minimum, int32_t maximum);
  static class IncrementPrecision incrementExact(uint64_t mantissa,
                                                 int16_t magnitude);

  Precision trailingZeroDisplay(UNumberTrailingZeroDisplay display) const;

  const std::string& stem() const { return stem_; }

  explicit Precision(std::string stem) : stem_(std::move(stem)) {}

 protected:
  std::string stem_;
};

class FractionPrecision : public Precision {
 public:
  explicit FractionPrecision(std::string stem) : Precision(std::move(stem)) {}

  Precision withSignificantDigits(int32_t minimum, int32_t maximum,
                                  UNumberRoundingPriority priority) const;
};

class IncrementPrecision : public Precision {
 public:
  explicit IncrementPrecision(std::string stem) : Precision(std::move(stem)) {}

  Precision withMinFraction(int32_t minimum) const;
};

class IntegerWidth {
 public:
  static IntegerWidth zeroFillTo(int32_t minimum);

  int32_t minimum() const { return minimum_; }

 private:
  explicit IntegerWidth(int32_t minimum) : minimum_(minimum) {}
  int32_t minimum_;
};

class Scale {
 public:
  static Scale powerOfTen(int32_t power);

  int32_t power() const { return power_; }

 private:
  explicit Scale(int32_t power) : power_(power) {}
  int32_t power_;
};

class UnlocalizedNumberFormatter;
class LocalizedNumberFormatter;

template <typename Derived>
class NumberFormatterSettings {
 public:
  Derived adoptSymbols(NumberingSystem* numbering_system) const {
    Derived result(static_cast<const Derived&>(*this));
    if (numbering_system != nullptr) {
      result.options_.numbering_system = numbering_system->getName();
    }
    delete numbering_system;
    result.clearHandle();
    return result;
  }

  Derived unit(const MeasureUnit& unit) const {
    Derived result(static_cast<const Derived&>(*this));
    result.options_.unit_type = unit.getType();
    result.options_.unit = unit.getIdentifier();
    result.clearHandle();
    return result;
  }

  Derived perUnit(const MeasureUnit& unit) const {
    Derived result(static_cast<const Derived&>(*this));
    result.options_.per_unit = unit.getIdentifier();
    result.clearHandle();
    return result;
  }

  Derived unitWidth(UNumberUnitWidth width) const {
    Derived result(static_cast<const Derived&>(*this));
    result.options_.unit_width = width;
    result.options_.has_unit_width = true;
    result.clearHandle();
    return result;
  }

  Derived notation(const Notation& notation) const {
    Derived result(static_cast<const Derived&>(*this));
    result.options_.notation = notation.stem();
    result.clearHandle();
    return result;
  }

  Derived precision(const Precision& precision) const {
    Derived result(static_cast<const Derived&>(*this));
    result.options_.precision = precision.stem();
    result.clearHandle();
    return result;
  }

  Derived integerWidth(const IntegerWidth& width) const {
    Derived result(static_cast<const Derived&>(*this));
    result.options_.integer_width = width.minimum();
    result.clearHandle();
    return result;
  }

  Derived roundingMode(UNumberFormatRoundingMode mode) const {
    Derived result(static_cast<const Derived&>(*this));
    result.options_.rounding_mode = impl::RoundingModeStem(mode);
    result.clearHandle();
    return result;
  }

  Derived grouping(UNumberGroupingStrategy grouping) const {
    Derived result(static_cast<const Derived&>(*this));
    result.options_.grouping = impl::GroupingStem(grouping);
    result.clearHandle();
    return result;
  }

  Derived sign(UNumberSignDisplay sign) const {
    Derived result(static_cast<const Derived&>(*this));
    result.options_.sign = impl::SignStem(sign);
    result.clearHandle();
    return result;
  }

  Derived scale(const Scale& scale) const {
    Derived result(static_cast<const Derived&>(*this));
    result.options_.scale = scale.power();
    result.options_.has_scale = true;
    result.clearHandle();
    return result;
  }

  UnicodeString toSkeleton(UErrorCode& status) const {
    return impl::BuildSkeleton(options_, status);
  }

 protected:
  NumberFormatterSettings() = default;
  explicit NumberFormatterSettings(const impl::NumberFormatterOptions& options)
      : options_(options) {}

  impl::NumberFormatterOptions options_;

  friend class UnlocalizedNumberFormatter;
  friend class LocalizedNumberFormatter;
};

class FormattedNumber : public UMemory, public CFormattedValue {
 public:
  FormattedNumber();
  FormattedNumber(UFormattedNumber* result, bool use_fullwidth_yen);
  FormattedNumber(FormattedNumber&& other) noexcept;
  FormattedNumber& operator=(FormattedNumber&& other) noexcept;
  virtual ~FormattedNumber() override;

  const UFormattedNumber* toUFormattedNumber() const { return result_; }
  virtual UnicodeString toString(UErrorCode& status) const override;

 protected:
  virtual const UFormattedValue* asUFormattedValue(
      UErrorCode& status) const override;

 private:
  UFormattedNumber* result_;
  bool use_fullwidth_yen_;
};

class LocalizedNumberFormatter
    : public NumberFormatterSettings<LocalizedNumberFormatter>,
      public UMemory {
 public:
  LocalizedNumberFormatter();
  LocalizedNumberFormatter(const LocalizedNumberFormatter& other);
  LocalizedNumberFormatter(LocalizedNumberFormatter&& other) noexcept;
  LocalizedNumberFormatter& operator=(const LocalizedNumberFormatter& other);
  LocalizedNumberFormatter& operator=(
      LocalizedNumberFormatter&& other) noexcept;
  ~LocalizedNumberFormatter();

  FormattedNumber formatInt(int64_t value, UErrorCode& status) const;
  FormattedNumber formatDouble(double value, UErrorCode& status) const;
  FormattedNumber formatDecimal(StringPiece value, UErrorCode& status) const;

  void clearHandle();

 private:
  explicit LocalizedNumberFormatter(
      const impl::NumberFormatterOptions& options);
  UNumberFormatter* getFormatter(UErrorCode& status) const;

  mutable std::shared_ptr<impl::NumberFormatterState> state_;

  friend class UnlocalizedNumberFormatter;
};

class UnlocalizedNumberFormatter
    : public NumberFormatterSettings<UnlocalizedNumberFormatter> {
 public:
  UnlocalizedNumberFormatter() = default;

  LocalizedNumberFormatter locale(const Locale& locale) const;
  void clearHandle() {}

 private:
  explicit UnlocalizedNumberFormatter(
      const impl::NumberFormatterOptions& options)
      : NumberFormatterSettings(options) {}

  friend class NumberFormatter;
};

class NumberFormatter {
 public:
  static UnlocalizedNumberFormatter forSkeleton(const UnicodeString& skeleton,
                                                UParseError& parse_error,
                                                UErrorCode& status);
};

}  // namespace number
U_NAMESPACE_END

#include "unicode/ucurr_compat.h"

#endif  // V8_SYSTEM_ICU_NUMBER_FORMATTER_H_
