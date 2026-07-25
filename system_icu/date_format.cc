// Copyright 2018-2026 the Deno authors. MIT license.

#include <cstring>
#include <memory>
#include <string>
#include <string_view>
#include <utility>
#include <vector>

#include "unicode/datefmt.h"
#include "unicode/dtfmtsym.h"
#include "unicode/dtitvfmt.h"
#include "unicode/dtptngen.h"
#include "unicode/smpdtfmt.h"

U_NAMESPACE_BEGIN
namespace {

UnicodeString CalendarTimeZoneId(const UCalendar* calendar) {
  UErrorCode status = U_ZERO_ERROR;
  int32_t length = ucal_getTimeZoneID(calendar, nullptr, 0, &status);
  if (status != U_BUFFER_OVERFLOW_ERROR && U_FAILURE(status)) {
    return {};
  }
  status = U_ZERO_ERROR;
  std::vector<UChar> result(length + 1);
  length = ucal_getTimeZoneID(calendar, result.data(), result.size(), &status);
  return U_SUCCESS(status)
             ? UnicodeString(reinterpret_cast<const char16_t*>(result.data()),
                             length)
             : UnicodeString();
}

Calendar* CloneCalendar(const UDateFormat* format) {
  const UCalendar* source = udat_getCalendar(format);
  if (source == nullptr) {
    return nullptr;
  }
  UErrorCode status = U_ZERO_ERROR;
  UCalendar* clone = ucal_clone(source, &status);
  return U_SUCCESS(status) && clone != nullptr
             ? new Calendar(clone, CalendarTimeZoneId(source))
             : nullptr;
}

template <typename Format>
UnicodeString& AppendFormatted(Format format, UnicodeString& result,
                               UErrorCode& status) {
  if (U_FAILURE(status)) {
    return result;
  }
  int32_t length = format(nullptr, 0, &status);
  if (status != U_BUFFER_OVERFLOW_ERROR && U_FAILURE(status)) {
    return result;
  }
  status = U_ZERO_ERROR;
  std::vector<UChar> buffer(length + 1);
  length = format(buffer.data(), buffer.size(), &status);
  if (U_SUCCESS(status)) {
    result.append(reinterpret_cast<const char16_t*>(buffer.data()), length);
  }
  return result;
}

UDateFormatStyle ToStyle(DateFormat::EStyle style) {
  return static_cast<UDateFormatStyle>(style);
}

void NormalizeGmtZero(UnicodeString& value,
                      FieldPositionIterator* positions = nullptr) {
  std::u16string text(value.getBuffer(), value.length());
  constexpr std::u16string_view kLongZero = u"GMT+00:00";
  constexpr std::u16string_view kShortZero = u"GMT+0";
  constexpr std::u16string_view kReplacement = u"GMT";

  size_t start = 0;
  while (start < text.size()) {
    size_t found = text.find(kLongZero, start);
    size_t length = kLongZero.size();
    if (found == std::u16string::npos) {
      found = text.find(kShortZero, start);
      length = kShortZero.size();
      if (found != std::u16string::npos && found + length < text.size() &&
          (text[found + length] == u':' ||
           (text[found + length] >= u'0' && text[found + length] <= u'9'))) {
        start = found + length;
        continue;
      }
    }
    if (found == std::u16string::npos) {
      break;
    }
    text.replace(found, length, kReplacement);
    if (positions != nullptr) {
      positions->adjustForReplacement(found, found + length,
                                      kReplacement.size());
    }
    start = found + kReplacement.size();
  }
  value = UnicodeString(text.data(), text.size());
}

template <typename Format>
UnicodeString& AppendNormalizedDate(Format format, UnicodeString& result,
                                    FieldPositionIterator* positions,
                                    UErrorCode& status) {
  UnicodeString formatted;
  AppendFormatted(format, formatted, status);
  if (U_SUCCESS(status)) {
    NormalizeGmtZero(formatted, positions);
    result.append(formatted);
  }
  return result;
}

std::string CalendarType(const Locale& locale) {
  UErrorCode status = U_ZERO_ERROR;
  char calendar[ULOC_KEYWORD_AND_VALUES_CAPACITY] = {};
  const int32_t length =
      locale.getKeywordValue("calendar", calendar, sizeof(calendar), status);
  return U_SUCCESS(status) && length > 0 ? std::string(calendar, length)
                                         : std::string();
}

bool HasPatternField(const std::u16string& pattern, char16_t field) {
  bool quoted = false;
  for (size_t index = 0; index < pattern.size(); ++index) {
    if (pattern[index] == u'\'') {
      if (index + 1 < pattern.size() && pattern[index + 1] == u'\'') {
        ++index;
      } else {
        quoted = !quoted;
      }
    } else if (!quoted && pattern[index] == field) {
      return true;
    }
  }
  return false;
}

UnicodeString AdjustDatePattern(const UnicodeString& input,
                                const Locale& locale) {
  std::u16string pattern(input.getBuffer(), input.length());
  const std::string calendar = CalendarType(locale);
  const bool japanese =
      std::strcmp(locale.getLanguage(), "ja") == 0 && calendar == "japanese";
  const bool chinese =
      std::strcmp(locale.getLanguage(), "zh") == 0 && calendar == "chinese";
  const bool arabic_islamic = std::strcmp(locale.getLanguage(), "ar") == 0 &&
                              calendar.compare(0, 7, "islamic") == 0;
  if (!japanese && !chinese && !arabic_islamic) {
    return input;
  }

  const bool chinese_needs_related_year = chinese &&
                                          HasPatternField(pattern, u'U') &&
                                          !HasPatternField(pattern, u'r');
  const bool chinese_numeric_date = chinese && HasPatternField(pattern, u'M') &&
                                    HasPatternField(pattern, u'd');

  std::u16string result;
  result.reserve(pattern.size() + (chinese_needs_related_year ? 1 : 0));
  bool quoted = false;
  for (size_t index = 0; index < pattern.size();) {
    const char16_t ch = pattern[index];
    if (ch == u'\'') {
      result += ch;
      if (index + 1 < pattern.size() && pattern[index + 1] == u'\'') {
        result += pattern[index + 1];
        index += 2;
      } else {
        quoted = !quoted;
        ++index;
      }
      continue;
    }

    if (!quoted && arabic_islamic && ch == u'\u060c') {
      size_t next = index + 1;
      while (next < pattern.size() && pattern[next] == u' ') {
        ++next;
      }
      if (next < pattern.size() && pattern[next] == u'y') {
        result += u' ';
        index = next;
        continue;
      }
    }

    if (!quoted && japanese && ch == u' ' && !result.empty() &&
        result.back() == u'\u65e5') {
      size_t next = index + 1;
      if (next < pattern.size() && pattern[next] == u'E') {
        ++index;
        continue;
      }
    }

    if (!quoted && japanese && (ch == u'M' || ch == u'd')) {
      size_t end = index + 1;
      while (end < pattern.size() && pattern[end] == ch) {
        ++end;
      }
      result.append(end - index == 2 ? 1 : end - index, ch);
      index = end;
      continue;
    }

    if (!quoted && chinese_needs_related_year && ch == u'U') {
      result += u'r';
    }
    if (!quoted && chinese_numeric_date && ch == u'-') {
      result += u'/';
    } else {
      result += ch;
    }
    ++index;
  }
  return UnicodeString(result.data(), result.size());
}

UDateFormat* AdjustStylePattern(UDateFormat* format, const Locale& locale,
                                UErrorCode& status) {
  if (format == nullptr || U_FAILURE(status)) {
    return format;
  }
  UnicodeString pattern;
  AppendFormatted(
      [&](UChar* buffer, int32_t capacity, UErrorCode* error) {
        return udat_toPattern(format, false, buffer, capacity, error);
      },
      pattern, status);
  if (U_FAILURE(status)) {
    return format;
  }
  UnicodeString adjusted = AdjustDatePattern(pattern, locale);
  if (adjusted == pattern) {
    return format;
  }

  UErrorCode replacement_status = U_ZERO_ERROR;
  UDateFormat* replacement =
      udat_open(UDAT_PATTERN, UDAT_PATTERN, locale.getName(), nullptr, 0,
                reinterpret_cast<const UChar*>(adjusted.getBuffer()),
                adjusted.length(), &replacement_status);
  if (U_SUCCESS(replacement_status) && replacement != nullptr) {
    udat_close(format);
    return replacement;
  }
  return format;
}

}  // namespace

DateFormat::DateFormat(UDateFormat* format, const Locale& locale)
    : format_(format), calendar_(CloneCalendar(format)), locale_(locale) {}

DateFormat::~DateFormat() {
  delete calendar_;
  udat_close(format_);
}

DateFormat* DateFormat::clone() const {
  UErrorCode status = U_ZERO_ERROR;
  UDateFormat* copy = udat_clone(format_, &status);
  return U_SUCCESS(status) && copy != nullptr
             ? new SimpleDateFormat(copy, locale_)
             : nullptr;
}

DateFormat* DateFormat::createTimeInstance(EStyle style, const Locale& locale) {
  UErrorCode status = U_ZERO_ERROR;
  UDateFormat* format = udat_open(ToStyle(style), UDAT_NONE, locale.getName(),
                                  nullptr, 0, nullptr, 0, &status);
  return U_SUCCESS(status) && format != nullptr
             ? new SimpleDateFormat(format, locale)
             : nullptr;
}

DateFormat* DateFormat::createDateInstance(EStyle style, const Locale& locale) {
  UErrorCode status = U_ZERO_ERROR;
  UDateFormat* format = nullptr;
  if (std::strcmp(locale.getLanguage(), "ja") == 0 &&
      CalendarType(locale) == "japanese") {
    const char* pattern = nullptr;
    switch (style) {
      case kFull:
        pattern = "Gy年M月d日EEEE";
        break;
      case kLong:
      case kMedium:
        pattern = "Gy年M月d日";
        break;
      case kShort:
        pattern = "GGGGGy/M/d";
        break;
      default:
        break;
    }
    if (pattern != nullptr) {
      UnicodeString unicode_pattern = UnicodeString::fromUTF8(pattern);
      format =
          udat_open(UDAT_PATTERN, UDAT_PATTERN, locale.getName(), nullptr, 0,
                    reinterpret_cast<const UChar*>(unicode_pattern.getBuffer()),
                    unicode_pattern.length(), &status);
    }
  }
  if (format == nullptr && U_SUCCESS(status)) {
    format = udat_open(UDAT_NONE, ToStyle(style), locale.getName(), nullptr, 0,
                       nullptr, 0, &status);
  }
  format = AdjustStylePattern(format, locale, status);
  return U_SUCCESS(status) && format != nullptr
             ? new SimpleDateFormat(format, locale)
             : nullptr;
}

DateFormat* DateFormat::createDateTimeInstance(EStyle date_style,
                                               EStyle time_style,
                                               const Locale& locale) {
  UErrorCode status = U_ZERO_ERROR;
  UDateFormat* format =
      udat_open(ToStyle(time_style), ToStyle(date_style), locale.getName(),
                nullptr, 0, nullptr, 0, &status);
  format = AdjustStylePattern(format, locale, status);
  return U_SUCCESS(status) && format != nullptr
             ? new SimpleDateFormat(format, locale)
             : nullptr;
}

DateFormat* DateFormat::createInstanceForSkeleton(const UnicodeString& skeleton,
                                                  const Locale& locale,
                                                  UErrorCode& status) {
  std::unique_ptr<DateTimePatternGenerator> generator(
      DateTimePatternGenerator::createInstance(locale, status));
  if (generator == nullptr || U_FAILURE(status)) {
    return nullptr;
  }
  UnicodeString pattern =
      generator->getBestPattern(skeleton, UDATPG_MATCH_NO_OPTIONS, status);
  return U_SUCCESS(status) ? new SimpleDateFormat(pattern, locale, status)
                           : nullptr;
}

UnicodeString& DateFormat::format(UDate date, UnicodeString& result) const {
  UErrorCode status = U_ZERO_ERROR;
  return AppendNormalizedDate(
      [&](UChar* buffer, int32_t capacity, UErrorCode* error) {
        return udat_format(format_, date, buffer, capacity, nullptr, error);
      },
      result, nullptr, status);
}

UnicodeString& DateFormat::format(UDate date, UnicodeString& result,
                                  FieldPositionIterator* positions,
                                  UErrorCode& status) const {
  UFieldPositionIterator* iterator =
      positions != nullptr ? positions->prepareForFormat(status) : nullptr;
  return AppendNormalizedDate(
      [&](UChar* buffer, int32_t capacity, UErrorCode* error) {
        return udat_formatForFields(format_, date, buffer, capacity, iterator,
                                    error);
      },
      result, positions, status);
}

UnicodeString& DateFormat::format(Calendar& calendar, UnicodeString& result,
                                  FieldPositionIterator* positions,
                                  UErrorCode& status) const {
  UFieldPositionIterator* iterator =
      positions != nullptr ? positions->prepareForFormat(status) : nullptr;
  return AppendNormalizedDate(
      [&](UChar* buffer, int32_t capacity, UErrorCode* error) {
        return udat_formatCalendarForFields(format_, calendar.toUCalendar(),
                                            buffer, capacity, iterator, error);
      },
      result, positions, status);
}

void DateFormat::setTimeZone(const TimeZone& zone) {
  if (calendar_ == nullptr) {
    return;
  }
  calendar_->setTimeZone(zone);
  udat_setCalendar(format_, calendar_->toUCalendar());
}

void DateFormat::adoptCalendar(Calendar* calendar) {
  if (calendar == nullptr) {
    return;
  }
  udat_setCalendar(format_, calendar->toUCalendar());
  delete calendar_;
  calendar_ = calendar;
}

SimpleDateFormat::SimpleDateFormat(const UnicodeString& pattern,
                                   const Locale& locale, UErrorCode& status)
    : DateFormat(
          [&]() {
            UnicodeString adjusted = AdjustDatePattern(pattern, locale);
            return udat_open(
                UDAT_PATTERN, UDAT_PATTERN, locale.getName(), nullptr, 0,
                reinterpret_cast<const UChar*>(adjusted.getBuffer()),
                adjusted.length(), &status);
          }(),
          locale) {}

SimpleDateFormat::SimpleDateFormat(UDateFormat* format, const Locale& locale)
    : DateFormat(format, locale) {}

SimpleDateFormat::~SimpleDateFormat() = default;

SimpleDateFormat* SimpleDateFormat::clone() const {
  UErrorCode status = U_ZERO_ERROR;
  UDateFormat* copy = udat_clone(format_, &status);
  return U_SUCCESS(status) && copy != nullptr
             ? new SimpleDateFormat(copy, locale_)
             : nullptr;
}

UnicodeString& SimpleDateFormat::toPattern(UnicodeString& result) const {
  UErrorCode status = U_ZERO_ERROR;
  return AppendFormatted(
      [&](UChar* buffer, int32_t capacity, UErrorCode* error) {
        return udat_toPattern(format_, false, buffer, capacity, error);
      },
      result, status);
}

const Locale& SimpleDateFormat::getSmpFmtLocale() const { return locale_; }

FormattedDateInterval::FormattedDateInterval() : result_(nullptr) {}

FormattedDateInterval::FormattedDateInterval(UFormattedDateInterval* result)
    : result_(result) {}

FormattedDateInterval::FormattedDateInterval(
    FormattedDateInterval&& other) noexcept
    : result_(other.result_) {
  other.result_ = nullptr;
}

FormattedDateInterval& FormattedDateInterval::operator=(
    FormattedDateInterval&& other) noexcept {
  if (this != &other) {
    udtitvfmt_closeResult(result_);
    result_ = other.result_;
    other.result_ = nullptr;
  }
  return *this;
}

FormattedDateInterval::~FormattedDateInterval() {
  udtitvfmt_closeResult(result_);
}

const UFormattedValue* FormattedDateInterval::asUFormattedValue(
    UErrorCode& status) const {
  return result_ != nullptr ? udtitvfmt_resultAsValue(result_, &status)
                            : nullptr;
}

DateIntervalFormat::DateIntervalFormat(std::u16string skeleton,
                                       std::string locale,
                                       std::u16string time_zone,
                                       UErrorCode& status)
    : format_(nullptr),
      skeleton_(std::move(skeleton)),
      locale_(std::move(locale)),
      time_zone_(std::move(time_zone)) {
  reopen(status);
}

DateIntervalFormat::~DateIntervalFormat() { udtitvfmt_close(format_); }

DateIntervalFormat* DateIntervalFormat::createInstance(
    const UnicodeString& skeleton, const Locale& locale, UErrorCode& status) {
  auto* result = new DateIntervalFormat(
      std::u16string(skeleton.getBuffer(), skeleton.length()), locale.getName(),
      {}, status);
  if (U_FAILURE(status) || result->format_ == nullptr) {
    delete result;
    return nullptr;
  }
  return result;
}

void DateIntervalFormat::reopen(UErrorCode& status) {
  if (U_FAILURE(status)) {
    return;
  }
  UDateIntervalFormat* replacement = udtitvfmt_open(
      locale_.c_str(), reinterpret_cast<const UChar*>(skeleton_.data()),
      skeleton_.size(),
      time_zone_.empty() ? nullptr
                         : reinterpret_cast<const UChar*>(time_zone_.data()),
      time_zone_.size(), &status);
  if (U_SUCCESS(status) && replacement != nullptr) {
    udtitvfmt_close(format_);
    format_ = replacement;
  }
}

DateIntervalFormat* DateIntervalFormat::clone() const {
  UErrorCode status = U_ZERO_ERROR;
  auto* result = new DateIntervalFormat(skeleton_, locale_, time_zone_, status);
  if (U_FAILURE(status) || result->format_ == nullptr) {
    delete result;
    return nullptr;
  }
  return result;
}

void DateIntervalFormat::setTimeZone(const TimeZone& zone) {
  UnicodeString id;
  zone.getID(id);
  time_zone_.assign(id.getBuffer(), id.length());
  UErrorCode status = U_ZERO_ERROR;
  reopen(status);
}

FormattedDateInterval DateIntervalFormat::formatToValue(
    Calendar& from, Calendar& to, UErrorCode& status) const {
  UFormattedDateInterval* result = udtitvfmt_openResult(&status);
  if (U_SUCCESS(status) && result != nullptr) {
    udtitvfmt_formatCalendarToResult(format_, from.toUCalendar(),
                                     to.toUCalendar(), result, &status);
  }
  if (U_FAILURE(status)) {
    udtitvfmt_closeResult(result);
    result = nullptr;
  }
  return FormattedDateInterval(result);
}

DateFormatSymbols::DateFormatSymbols(const Locale& locale, UErrorCode& status)
    : locale_(locale) {
  UDateTimePatternGenerator* generator = udatpg_open(locale.getName(), &status);
  udatpg_close(generator);
}

DateFormatSymbols::~DateFormatSymbols() = default;

UnicodeString& DateFormatSymbols::getTimeSeparatorString(
    UnicodeString& result) const {
  UErrorCode status = U_ZERO_ERROR;
  std::unique_ptr<DateTimePatternGenerator> generator(
      DateTimePatternGenerator::createInstance(locale_, status));
  UnicodeString pattern;
  if (generator != nullptr) {
    pattern = generator->getBestPattern(UnicodeString::fromUTF8("Hm"),
                                        UDATPG_MATCH_NO_OPTIONS, status);
  }

  int32_t hour_end = -1;
  int32_t minute_start = -1;
  for (int32_t i = 0; i < pattern.length(); ++i) {
    char16_t ch = pattern[i];
    if (ch == u'H' || ch == u'h' || ch == u'K' || ch == u'k') {
      hour_end = i + 1;
    } else if (ch == u'm' && hour_end >= 0) {
      minute_start = i;
      break;
    }
  }
  if (hour_end >= 0 && minute_start > hour_end) {
    for (int32_t i = hour_end; i < minute_start; ++i) {
      char16_t ch = pattern[i];
      if (ch != u' ' && ch != u'\'' && ch != u'\u200f' && ch != u'\u061c') {
        result.append(ch);
      }
    }
  }
  if (result.isEmpty()) {
    result.append(u':');
  }
  return result;
}

U_NAMESPACE_END
