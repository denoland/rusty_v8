// Copyright 2018-2026 the Deno authors. MIT license.

#include <algorithm>
#include <iterator>

#include "unicode/currunit.h"
#include "unicode/measunit.h"

U_NAMESPACE_BEGIN

namespace {

constexpr const char* kSanctionedUnits[] = {
    "acre",        "bit",         "byte",        "celsius",
    "centimeter",  "day",         "degree",      "fahrenheit",
    "fluid-ounce", "foot",        "gallon",      "gigabit",
    "gigabyte",    "gram",        "hectare",     "hour",
    "inch",        "kilobit",     "kilobyte",    "kilogram",
    "kilometer",   "liter",       "megabit",     "megabyte",
    "meter",       "microsecond", "mile",        "mile-scandinavian",
    "millimeter",  "milliliter",  "millisecond", "minute",
    "month",       "nanosecond",  "ounce",       "percent",
    "petabyte",    "pound",       "second",      "stone",
    "terabit",     "terabyte",    "week",        "yard",
    "year",
};

}  // namespace

MeasureUnit::MeasureUnit() : type_("none"), subtype_() {}

MeasureUnit::MeasureUnit(const char* type, const char* subtype)
    : type_(type), subtype_(subtype) {}

MeasureUnit::MeasureUnit(const MeasureUnit& other) = default;

MeasureUnit::MeasureUnit(MeasureUnit&& other) noexcept = default;

MeasureUnit& MeasureUnit::operator=(const MeasureUnit& other) = default;

MeasureUnit& MeasureUnit::operator=(MeasureUnit&& other) noexcept = default;

MeasureUnit::~MeasureUnit() = default;

bool MeasureUnit::operator==(const UObject& other) const {
  const auto& unit = static_cast<const MeasureUnit&>(other);
  return type_ == unit.type_ && subtype_ == unit.subtype_;
}

const char* MeasureUnit::getType() const { return type_.c_str(); }

const char* MeasureUnit::getSubtype() const { return subtype_.c_str(); }

const char* MeasureUnit::getIdentifier() const { return subtype_.c_str(); }

int32_t MeasureUnit::getAvailable(MeasureUnit* destination, int32_t capacity,
                                  UErrorCode& status) {
  if (U_FAILURE(status)) {
    return 0;
  }

  const int32_t count = std::size(kSanctionedUnits);
  if (destination == nullptr || capacity < count) {
    status = U_BUFFER_OVERFLOW_ERROR;
    return count;
  }

  for (int32_t index = 0; index < count; ++index) {
    destination[index] = MeasureUnit(
        kSanctionedUnits[index] == std::string("percent") ? "concentr" : "unit",
        kSanctionedUnits[index]);
  }
  return count;
}

MeasureUnit MeasureUnit::getPercent() {
  return MeasureUnit("concentr", "percent");
}

MeasureUnit MeasureUnit::getPermille() {
  return MeasureUnit("concentr", "permille");
}

MeasureUnit MeasureUnit::getYear() { return MeasureUnit("duration", "year"); }

MeasureUnit MeasureUnit::getMonth() { return MeasureUnit("duration", "month"); }

MeasureUnit MeasureUnit::getWeek() { return MeasureUnit("duration", "week"); }

MeasureUnit MeasureUnit::getDay() { return MeasureUnit("duration", "day"); }

MeasureUnit MeasureUnit::getHour() { return MeasureUnit("duration", "hour"); }

MeasureUnit MeasureUnit::getMinute() {
  return MeasureUnit("duration", "minute");
}

MeasureUnit MeasureUnit::getSecond() {
  return MeasureUnit("duration", "second");
}

MeasureUnit MeasureUnit::getMillisecond() {
  return MeasureUnit("duration", "millisecond");
}

MeasureUnit MeasureUnit::getMicrosecond() {
  return MeasureUnit("duration", "microsecond");
}

MeasureUnit MeasureUnit::getNanosecond() {
  return MeasureUnit("duration", "nanosecond");
}

CurrencyUnit::CurrencyUnit()
    : MeasureUnit("currency", "XXX"), iso_code_{u'X', u'X', u'X', 0} {}

CurrencyUnit::CurrencyUnit(ConstChar16Ptr iso_code, UErrorCode& status)
    : MeasureUnit(), iso_code_{} {
  const char16_t* code = iso_code;
  char ascii_code[4] = {};
  if (code == nullptr) {
    code = u"XXX";
  }
  for (int index = 0; index < 3; ++index) {
    if (code[index] > 0x7f) {
      status = U_ILLEGAL_ARGUMENT_ERROR;
      return;
    }
    ascii_code[index] = static_cast<char>(code[index]);
    iso_code_[index] = code[index];
  }
  *static_cast<MeasureUnit*>(this) = MeasureUnit("currency", ascii_code);
}

CurrencyUnit::CurrencyUnit(const CurrencyUnit& other) = default;

CurrencyUnit& CurrencyUnit::operator=(const CurrencyUnit& other) = default;

CurrencyUnit::~CurrencyUnit() = default;

U_NAMESPACE_END
