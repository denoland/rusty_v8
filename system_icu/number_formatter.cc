// Copyright 2018-2026 the Deno authors. MIT license.

#include <charconv>
#include <cmath>
#include <cstdio>
#include <mutex>
#include <utility>

#include "unicode/numberformatter.h"
#include "unicode/numberrangeformatter.h"

U_NAMESPACE_BEGIN

Formattable::Formattable()
    : is_decimal_(false), double_value_(0), decimal_value_() {}

Formattable::Formattable(double value)
    : is_decimal_(false), double_value_(value), decimal_value_() {}

Formattable::Formattable(StringPiece decimal, UErrorCode& status)
    : is_decimal_(true),
      double_value_(0),
      decimal_value_(decimal.data(), decimal.length()) {
  if (decimal.empty()) {
    status = U_INVALID_FORMAT_ERROR;
  }
}

Formattable::Formattable(const Formattable& other) = default;

Formattable& Formattable::operator=(const Formattable& other) = default;

Formattable::~Formattable() = default;

namespace number {
namespace impl {

struct NumberFormatterState {
  explicit NumberFormatterState(const NumberFormatterOptions& options)
      : options(options) {}
  ~NumberFormatterState() { unumf_close(formatter); }

  NumberFormatterOptions options;
  std::once_flag once;
  UNumberFormatter* formatter = nullptr;
  UErrorCode status = U_ZERO_ERROR;
};

namespace {

void AppendToken(std::string& skeleton, const std::string& token) {
  if (token.empty()) {
    return;
  }
  if (!skeleton.empty()) {
    skeleton += ' ';
  }
  skeleton += token;
}

std::string ScaleStem(int32_t power) {
  if (power == 0) {
    return "scale/1";
  }
  if (power > 0) {
    return "scale/1" + std::string(power, '0');
  }
  return "scale/0." + std::string(-power - 1, '0') + "1";
}

bool IsJapaneseLocale(const std::string& locale) {
  return locale == "ja" ||
         (locale.size() > 2 &&
          (locale[2] == '_' || locale[2] == '-' || locale[2] == '@') &&
          locale.compare(0, 2, "ja") == 0);
}

bool ShouldUseFullwidthYen(const NumberFormatterOptions& options) {
  if (!IsJapaneseLocale(options.locale)) {
    return false;
  }
  if (options.unit_type == "currency" && options.unit == "JPY") {
    return true;
  }
  return options.raw_skeleton.find("currency/JPY") != std::string::npos;
}

UnicodeString UseFullwidthYen(UnicodeString result) {
  for (int32_t i = 0; i < result.length(); ++i) {
    if (result.charAt(i) == 0x00a5) {
      result.setCharAt(i, 0xffe5);
    }
  }
  return result;
}

}  // namespace

UnicodeString BuildSkeleton(const NumberFormatterOptions& options,
                            UErrorCode& status) {
  if (U_FAILURE(status)) {
    return {};
  }
  if (!options.raw_skeleton.empty()) {
    return UnicodeString::fromUTF8(options.raw_skeleton);
  }

  std::string skeleton;
  if (!options.numbering_system.empty()) {
    AppendToken(skeleton, "numbering-system/" + options.numbering_system);
  }
  if (!options.unit.empty()) {
    if (options.unit_type == "currency") {
      AppendToken(skeleton, "currency/" + options.unit);
    } else if (options.unit == "percent") {
      AppendToken(skeleton, "percent");
    } else {
      std::string unit = options.unit;
      if (!options.per_unit.empty()) {
        unit += "-per-" + options.per_unit;
      }
      AppendToken(skeleton, "unit/" + unit);
    }
  }
  AppendToken(skeleton, options.notation);
  AppendToken(skeleton, options.precision);
  AppendToken(skeleton, options.rounding_mode);
  if (options.integer_width > 0) {
    AppendToken(skeleton,
                "integer-width/*" + std::string(options.integer_width, '0'));
  }
  AppendToken(skeleton, options.grouping);
  AppendToken(skeleton, options.sign);
  if (options.has_unit_width) {
    AppendToken(
        skeleton,
        UnitWidthStem(static_cast<UNumberUnitWidth>(options.unit_width)));
  }
  if (options.has_scale) {
    AppendToken(skeleton, ScaleStem(options.scale));
  }
  return UnicodeString::fromUTF8(skeleton);
}

const char* RoundingModeStem(UNumberFormatRoundingMode mode) {
  switch (mode) {
    case UNUM_ROUND_CEILING:
      return "rounding-mode-ceiling";
    case UNUM_ROUND_FLOOR:
      return "rounding-mode-floor";
    case UNUM_ROUND_DOWN:
      return "rounding-mode-down";
    case UNUM_ROUND_UP:
      return "rounding-mode-up";
    case UNUM_ROUND_HALFEVEN:
      return "rounding-mode-half-even";
    case UNUM_ROUND_HALFDOWN:
      return "rounding-mode-half-down";
    case UNUM_ROUND_HALFUP:
      return "rounding-mode-half-up";
    case UNUM_ROUND_HALF_CEILING:
      return "rounding-mode-half-ceiling";
    case UNUM_ROUND_HALF_FLOOR:
      return "rounding-mode-half-floor";
    default:
      return "";
  }
}

const char* GroupingStem(UNumberGroupingStrategy grouping) {
  switch (grouping) {
    case UNUM_GROUPING_OFF:
      return "group-off";
    case UNUM_GROUPING_MIN2:
      return "group-min2";
    case UNUM_GROUPING_ON_ALIGNED:
      return "group-on-aligned";
    case UNUM_GROUPING_THOUSANDS:
      return "group-thousands";
    default:
      return "";
  }
}

const char* SignStem(UNumberSignDisplay sign) {
  switch (sign) {
    case UNUM_SIGN_NEVER:
      return "sign-never";
    case UNUM_SIGN_ALWAYS:
      return "sign-always";
    case UNUM_SIGN_EXCEPT_ZERO:
      return "sign-except-zero";
    case UNUM_SIGN_NEGATIVE:
      return "sign-negative";
    case UNUM_SIGN_ACCOUNTING:
      return "sign-accounting";
    case UNUM_SIGN_ACCOUNTING_ALWAYS:
      return "sign-accounting-always";
    case UNUM_SIGN_ACCOUNTING_EXCEPT_ZERO:
      return "sign-accounting-except-zero";
    case UNUM_SIGN_ACCOUNTING_NEGATIVE:
      return "sign-accounting-negative";
    default:
      return "";
  }
}

const char* UnitWidthStem(UNumberUnitWidth width) {
  switch (width) {
    case UNUM_UNIT_WIDTH_NARROW:
      return "unit-width-narrow";
    case UNUM_UNIT_WIDTH_FULL_NAME:
      return "unit-width-full-name";
    case UNUM_UNIT_WIDTH_ISO_CODE:
      return "unit-width-iso-code";
    case UNUM_UNIT_WIDTH_HIDDEN:
      return "unit-width-hidden";
    default:
      return "";
  }
}

}  // namespace impl

Notation Notation::simple() { return Notation(""); }

Notation Notation::scientific() { return Notation("scientific"); }

Notation Notation::engineering() { return Notation("engineering"); }

Notation Notation::compactShort() { return Notation("compact-short"); }

Notation Notation::compactLong() { return Notation("compact-long"); }

Precision Precision::unlimited() { return Precision(""); }

FractionPrecision Precision::minMaxFraction(int32_t minimum, int32_t maximum) {
  if (maximum == 0) {
    return FractionPrecision("precision-integer");
  }
  return FractionPrecision("." + std::string(minimum, '0') +
                           std::string(maximum - minimum, '#'));
}

Precision Precision::minMaxSignificantDigits(int32_t minimum, int32_t maximum) {
  return Precision(std::string(minimum, '@') +
                   std::string(maximum - minimum, '#'));
}

IncrementPrecision Precision::incrementExact(uint64_t mantissa,
                                             int16_t magnitude) {
  std::string digits = std::to_string(mantissa);
  std::string value;
  if (magnitude >= 0) {
    value = digits + std::string(magnitude, '0');
  } else {
    const int32_t decimal_places = -magnitude;
    if (decimal_places >= static_cast<int32_t>(digits.size())) {
      value = "0." + std::string(decimal_places - digits.size(), '0') + digits;
    } else {
      value = digits;
      value.insert(value.size() - decimal_places, 1, '.');
    }
  }
  return IncrementPrecision("precision-increment/" + value);
}

Precision Precision::trailingZeroDisplay(
    UNumberTrailingZeroDisplay display) const {
  return display == UNUM_TRAILING_ZERO_HIDE_IF_WHOLE ? Precision(stem_ + "/w")
                                                     : *this;
}

Precision FractionPrecision::withSignificantDigits(
    int32_t minimum, int32_t maximum, UNumberRoundingPriority priority) const {
  const char priority_char =
      priority == UNUM_ROUNDING_PRIORITY_RELAXED ? 'r' : 's';
  return Precision(stem_ + "/" + std::string(minimum, '@') +
                   std::string(maximum - minimum, '#') + priority_char);
}

Precision IncrementPrecision::withMinFraction(int32_t minimum) const {
  const size_t dot = stem_.find('.');
  const size_t existing = dot == std::string::npos ? 0 : stem_.size() - dot - 1;
  if (existing >= static_cast<size_t>(minimum)) {
    return *this;
  }
  return Precision(stem_ + std::string(minimum - existing, '0'));
}

IntegerWidth IntegerWidth::zeroFillTo(int32_t minimum) {
  return IntegerWidth(minimum);
}

Scale Scale::powerOfTen(int32_t power) { return Scale(power); }

FormattedNumber::FormattedNumber()
    : result_(nullptr), use_fullwidth_yen_(false) {}

FormattedNumber::FormattedNumber(UFormattedNumber* result,
                                 bool use_fullwidth_yen)
    : result_(result), use_fullwidth_yen_(use_fullwidth_yen) {}

FormattedNumber::FormattedNumber(FormattedNumber&& other) noexcept
    : result_(std::exchange(other.result_, nullptr)),
      use_fullwidth_yen_(other.use_fullwidth_yen_) {}

FormattedNumber& FormattedNumber::operator=(FormattedNumber&& other) noexcept {
  if (this != &other) {
    unumf_closeResult(result_);
    result_ = std::exchange(other.result_, nullptr);
    use_fullwidth_yen_ = other.use_fullwidth_yen_;
  }
  return *this;
}

FormattedNumber::~FormattedNumber() { unumf_closeResult(result_); }

UnicodeString FormattedNumber::toString(UErrorCode& status) const {
  UnicodeString result = CFormattedValue::toString(status);
  return use_fullwidth_yen_ ? impl::UseFullwidthYen(std::move(result)) : result;
}

const UFormattedValue* FormattedNumber::asUFormattedValue(
    UErrorCode& status) const {
  if (result_ == nullptr) {
    status = U_INVALID_STATE_ERROR;
    return nullptr;
  }
  return unumf_resultAsValue(result_, &status);
}

LocalizedNumberFormatter::LocalizedNumberFormatter() = default;

LocalizedNumberFormatter::LocalizedNumberFormatter(
    const impl::NumberFormatterOptions& options)
    : NumberFormatterSettings(options),
      state_(std::make_shared<impl::NumberFormatterState>(options)) {}

LocalizedNumberFormatter::LocalizedNumberFormatter(
    const LocalizedNumberFormatter& other) = default;

LocalizedNumberFormatter::LocalizedNumberFormatter(
    LocalizedNumberFormatter&& other) noexcept = default;

LocalizedNumberFormatter& LocalizedNumberFormatter::operator=(
    const LocalizedNumberFormatter& other) = default;

LocalizedNumberFormatter& LocalizedNumberFormatter::operator=(
    LocalizedNumberFormatter&& other) noexcept = default;

LocalizedNumberFormatter::~LocalizedNumberFormatter() = default;

void LocalizedNumberFormatter::clearHandle() {
  state_ = std::make_shared<impl::NumberFormatterState>(options_);
}

UNumberFormatter* LocalizedNumberFormatter::getFormatter(
    UErrorCode& status) const {
  if (state_ == nullptr) {
    state_ = std::make_shared<impl::NumberFormatterState>(options_);
  }
  std::call_once(state_->once, [this] {
    UErrorCode build_status = U_ZERO_ERROR;
    UnicodeString skeleton = impl::BuildSkeleton(options_, build_status);
    if (U_SUCCESS(build_status)) {
      state_->formatter = unumf_openForSkeletonAndLocale(
          reinterpret_cast<const UChar*>(skeleton.getBuffer()),
          skeleton.length(), options_.locale.c_str(), &build_status);
    }
    state_->status = build_status;
  });
  if (U_SUCCESS(status) && U_FAILURE(state_->status)) {
    status = state_->status;
  }
  return state_->formatter;
}

FormattedNumber LocalizedNumberFormatter::formatInt(int64_t value,
                                                    UErrorCode& status) const {
  UFormattedNumber* result = unumf_openResult(&status);
  UNumberFormatter* formatter = getFormatter(status);
  if (result != nullptr && formatter != nullptr) {
    unumf_formatInt(formatter, value, result, &status);
  }
  return FormattedNumber(result, impl::ShouldUseFullwidthYen(options_));
}

FormattedNumber LocalizedNumberFormatter::formatDouble(
    double value, UErrorCode& status) const {
  UFormattedNumber* result = unumf_openResult(&status);
  UNumberFormatter* formatter = getFormatter(status);
  if (result != nullptr && formatter != nullptr) {
    unumf_formatDouble(formatter, value, result, &status);
  }
  return FormattedNumber(result, impl::ShouldUseFullwidthYen(options_));
}

FormattedNumber LocalizedNumberFormatter::formatDecimal(
    StringPiece value, UErrorCode& status) const {
  UFormattedNumber* result = unumf_openResult(&status);
  UNumberFormatter* formatter = getFormatter(status);
  if (result != nullptr && formatter != nullptr) {
    unumf_formatDecimal(formatter, value.data(), value.length(), result,
                        &status);
  }
  return FormattedNumber(result, impl::ShouldUseFullwidthYen(options_));
}

LocalizedNumberFormatter UnlocalizedNumberFormatter::locale(
    const Locale& locale) const {
  impl::NumberFormatterOptions options = options_;
  options.locale = locale.getName();
  return LocalizedNumberFormatter(options);
}

UnlocalizedNumberFormatter NumberFormatter::forSkeleton(
    const UnicodeString& skeleton, UParseError& parse_error,
    UErrorCode& status) {
  impl::NumberFormatterOptions options;
  skeleton.toUTF8String(options.raw_skeleton);
  parse_error.offset = -1;
  if (U_FAILURE(status)) {
    return UnlocalizedNumberFormatter();
  }
  return UnlocalizedNumberFormatter(options);
}

FormattedNumberRange::FormattedNumberRange()
    : result_(nullptr), use_fullwidth_yen_(false) {}

FormattedNumberRange::FormattedNumberRange(UFormattedNumberRange* result,
                                           bool use_fullwidth_yen)
    : result_(result), use_fullwidth_yen_(use_fullwidth_yen) {}

FormattedNumberRange::FormattedNumberRange(
    FormattedNumberRange&& other) noexcept
    : result_(std::exchange(other.result_, nullptr)),
      use_fullwidth_yen_(other.use_fullwidth_yen_) {}

FormattedNumberRange& FormattedNumberRange::operator=(
    FormattedNumberRange&& other) noexcept {
  if (this != &other) {
    unumrf_closeResult(result_);
    result_ = std::exchange(other.result_, nullptr);
    use_fullwidth_yen_ = other.use_fullwidth_yen_;
  }
  return *this;
}

FormattedNumberRange::~FormattedNumberRange() { unumrf_closeResult(result_); }

UnicodeString FormattedNumberRange::toString(UErrorCode& status) const {
  UnicodeString result = CFormattedValue::toString(status);
  return use_fullwidth_yen_ ? impl::UseFullwidthYen(std::move(result)) : result;
}

const UFormattedValue* FormattedNumberRange::asUFormattedValue(
    UErrorCode& status) const {
  if (result_ == nullptr) {
    status = U_INVALID_STATE_ERROR;
    return nullptr;
  }
  return unumrf_resultAsValue(result_, &status);
}

LocalizedNumberRangeFormatter::LocalizedNumberRangeFormatter() = default;

LocalizedNumberRangeFormatter::LocalizedNumberRangeFormatter(
    std::u16string skeleton, std::string locale)
    : skeleton_(std::move(skeleton)), locale_(std::move(locale)) {}

LocalizedNumberRangeFormatter::LocalizedNumberRangeFormatter(
    const LocalizedNumberRangeFormatter& other) = default;

LocalizedNumberRangeFormatter::LocalizedNumberRangeFormatter(
    LocalizedNumberRangeFormatter&& other) noexcept = default;

LocalizedNumberRangeFormatter& LocalizedNumberRangeFormatter::operator=(
    const LocalizedNumberRangeFormatter& other) = default;

LocalizedNumberRangeFormatter& LocalizedNumberRangeFormatter::operator=(
    LocalizedNumberRangeFormatter&& other) noexcept = default;

LocalizedNumberRangeFormatter::~LocalizedNumberRangeFormatter() = default;

FormattedNumberRange LocalizedNumberRangeFormatter::formatFormattableRange(
    const Formattable& first, const Formattable& second,
    UErrorCode& status) const {
  UNumberRangeFormatter* formatter =
      unumrf_openForSkeletonWithCollapseAndIdentityFallback(
          reinterpret_cast<const UChar*>(skeleton_.data()), skeleton_.size(),
          UNUM_RANGE_COLLAPSE_AUTO, UNUM_IDENTITY_FALLBACK_APPROXIMATELY,
          locale_.c_str(), nullptr, &status);
  UFormattedNumberRange* result = unumrf_openResult(&status);
  if (formatter != nullptr && result != nullptr) {
    if (!first.isDecimal() && !second.isDecimal()) {
      unumrf_formatDoubleRange(formatter, first.getDouble(), second.getDouble(),
                               result, &status);
    } else {
      const std::string first_value = first.isDecimal()
                                          ? first.getDecimal()
                                          : std::to_string(first.getDouble());
      const std::string second_value = second.isDecimal()
                                           ? second.getDecimal()
                                           : std::to_string(second.getDouble());
      unumrf_formatDecimalRange(formatter, first_value.data(),
                                first_value.size(), second_value.data(),
                                second_value.size(), result, &status);
    }
  }
  unumrf_close(formatter);
  const bool use_fullwidth_yen =
      impl::IsJapaneseLocale(locale_) &&
      skeleton_.find(u"currency/JPY") != std::u16string::npos;
  return FormattedNumberRange(result, use_fullwidth_yen);
}

UnlocalizedNumberRangeFormatter
UnlocalizedNumberRangeFormatter::numberFormatterBoth(
    UnlocalizedNumberFormatter&& formatter) const {
  UnlocalizedNumberRangeFormatter result(*this);
  UErrorCode status = U_ZERO_ERROR;
  UnicodeString skeleton = formatter.toSkeleton(status);
  result.skeleton_.assign(skeleton.getBuffer(),
                          skeleton.getBuffer() + skeleton.length());
  return result;
}

LocalizedNumberRangeFormatter UnlocalizedNumberRangeFormatter::locale(
    const Locale& locale) const {
  return LocalizedNumberRangeFormatter(skeleton_, locale.getName());
}

}  // namespace number
U_NAMESPACE_END
