// Copyright 2018-2026 the Deno authors. MIT license.

#include <algorithm>
#include <array>
#include <cctype>
#include <string>

#include "unicode/localebuilder.h"
#include "unicode/uloc.h"

U_NAMESPACE_BEGIN
namespace {

enum Component {
  kLanguage,
  kScript,
  kRegion,
  kVariant,
};

bool AllChars(const std::string& value, bool (*predicate)(unsigned char)) {
  return std::all_of(value.begin(), value.end(), [predicate](char ch) {
    return predicate(static_cast<unsigned char>(ch));
  });
}

bool IsAlpha(unsigned char ch) { return std::isalpha(ch) != 0; }

bool IsDigit(unsigned char ch) { return std::isdigit(ch) != 0; }

bool IsAlphaNumeric(unsigned char ch) { return std::isalnum(ch) != 0; }

bool IsValidVariant(const std::string& value) {
  size_t start = 0;
  while (start < value.size()) {
    const size_t end = value.find_first_of("-_", start);
    const size_t length =
        (end == std::string::npos ? value.size() : end) - start;
    const std::string subtag = value.substr(start, length);
    if (!AllChars(subtag, IsAlphaNumeric) ||
        !((length >= 5 && length <= 8) ||
          (length == 4 && IsDigit(subtag[0])))) {
      return false;
    }
    if (end == std::string::npos) {
      return true;
    }
    start = end + 1;
  }
  return value.empty();
}

bool IsValidComponent(const std::string& value, int component) {
  if (value.empty()) {
    return true;
  }
  switch (component) {
    case kLanguage:
      return AllChars(value, IsAlpha) &&
             ((value.size() >= 2 && value.size() <= 3) ||
              (value.size() >= 5 && value.size() <= 8));
    case kScript:
      return value.size() == 4 && AllChars(value, IsAlpha);
    case kRegion:
      return (value.size() == 2 && AllChars(value, IsAlpha)) ||
             (value.size() == 3 && AllChars(value, IsDigit));
    case kVariant:
      return IsValidVariant(value);
  }
  return false;
}

template <typename Get>
std::string GetComponent(const std::string& locale, Get get) {
  UErrorCode status = U_ZERO_ERROR;
  std::array<char, ULOC_FULLNAME_CAPACITY> stack_buffer = {};
  int32_t length =
      get(locale.c_str(), stack_buffer.data(), stack_buffer.size(), &status);
  if (status != U_BUFFER_OVERFLOW_ERROR) {
    return U_SUCCESS(status) ? std::string(stack_buffer.data(), length)
                             : std::string();
  }
  status = U_ZERO_ERROR;
  std::string result(length + 1, '\0');
  length = get(locale.c_str(), result.data(), result.size(), &status);
  return U_SUCCESS(status) ? std::string(result.data(), length) : std::string();
}

std::string Language(const std::string& locale) {
  return GetComponent(locale, uloc_getLanguage);
}

std::string Script(const std::string& locale) {
  return GetComponent(locale, uloc_getScript);
}

std::string Region(const std::string& locale) {
  return GetComponent(locale, uloc_getCountry);
}

std::string Variant(const std::string& locale) {
  return GetComponent(locale, uloc_getVariant);
}

std::string Extensions(const std::string& locale) {
  const size_t separator = locale.find('@');
  return separator == std::string::npos ? std::string()
                                        : locale.substr(separator);
}

}  // namespace

LocaleBuilder::LocaleBuilder() : locale_id_(), error_(U_ZERO_ERROR) {}

LocaleBuilder::~LocaleBuilder() = default;

LocaleBuilder& LocaleBuilder::setLocale(const Locale& locale) {
  if (locale.isBogus()) {
    error_ = U_ILLEGAL_ARGUMENT_ERROR;
  } else {
    locale_id_ = locale.getName();
  }
  return *this;
}

LocaleBuilder& LocaleBuilder::setLanguageTag(StringPiece tag) {
  UErrorCode status = U_ZERO_ERROR;
  Locale locale = Locale::forLanguageTag(tag, status);
  if (U_FAILURE(status)) {
    error_ = status;
  } else {
    locale_id_ = locale.getName();
  }
  return *this;
}

LocaleBuilder& LocaleBuilder::setLanguage(StringPiece language) {
  setComponent(language, kLanguage);
  return *this;
}

LocaleBuilder& LocaleBuilder::setScript(StringPiece script) {
  setComponent(script, kScript);
  return *this;
}

LocaleBuilder& LocaleBuilder::setRegion(StringPiece region) {
  setComponent(region, kRegion);
  return *this;
}

LocaleBuilder& LocaleBuilder::setVariant(StringPiece variant) {
  setComponent(variant, kVariant);
  return *this;
}

LocaleBuilder& LocaleBuilder::setUnicodeLocaleKeyword(StringPiece key,
                                                      StringPiece value) {
  if (U_FAILURE(error_)) {
    return *this;
  }
  const std::string unicode_key(key.data(), key.length());
  const std::string unicode_value(value.data(), value.length());
  const char* legacy_key = uloc_toLegacyKey(unicode_key.c_str());
  const char* legacy_value =
      unicode_value.empty()
          ? ""
          : uloc_toLegacyType(unicode_key.c_str(), unicode_value.c_str());
  if (legacy_key == nullptr || legacy_value == nullptr) {
    error_ = U_ILLEGAL_ARGUMENT_ERROR;
    return *this;
  }

  int32_t capacity = std::max<int32_t>(
      ULOC_FULLNAME_CAPACITY,
      locale_id_.size() + unicode_key.size() + unicode_value.size() + 32);
  for (;;) {
    std::string result(capacity, '\0');
    std::copy(locale_id_.begin(), locale_id_.end(), result.begin());
    UErrorCode status = U_ZERO_ERROR;
    const int32_t length = uloc_setKeywordValue(
        legacy_key, legacy_value, result.data(), result.size(), &status);
    if (status == U_BUFFER_OVERFLOW_ERROR) {
      capacity = length + 1;
      continue;
    }
    if (U_FAILURE(status)) {
      error_ = status;
    } else {
      locale_id_.assign(result.data(), length);
    }
    return *this;
  }
}

void LocaleBuilder::setComponent(StringPiece value, int component) {
  if (U_FAILURE(error_)) {
    return;
  }
  std::string replacement(value.data(), value.length());
  if (!IsValidComponent(replacement, component)) {
    error_ = U_ILLEGAL_ARGUMENT_ERROR;
    return;
  }
  const std::string extensions = Extensions(locale_id_);
  std::string language = Language(locale_id_);
  std::string script = Script(locale_id_);
  std::string region = Region(locale_id_);
  std::string variant = Variant(locale_id_);
  switch (component) {
    case kLanguage:
      language = std::move(replacement);
      break;
    case kScript:
      script = std::move(replacement);
      break;
    case kRegion:
      region = std::move(replacement);
      break;
    case kVariant:
      variant = std::move(replacement);
      break;
  }

  if (language.empty() &&
      (!script.empty() || !region.empty() || !variant.empty())) {
    language = "und";
  }
  locale_id_ = language;
  if (!script.empty()) {
    locale_id_ += "_" + script;
  }
  if (!region.empty()) {
    locale_id_ += "_" + region;
  }
  if (!variant.empty()) {
    locale_id_ += "_" + variant;
  }
  locale_id_ += extensions;
}

LocaleBuilder& LocaleBuilder::clearExtensions() {
  const size_t separator = locale_id_.find('@');
  if (separator != std::string::npos) {
    locale_id_.erase(separator);
  }
  return *this;
}

Locale LocaleBuilder::build(UErrorCode& status) {
  if (U_FAILURE(status)) {
    return Locale("", "", "", "");
  }
  if (U_FAILURE(error_)) {
    status = error_;
    return Locale("", "", "", "");
  }

  Locale locale(locale_id_.c_str(), "", "", "");
  if (locale.isBogus()) {
    status = U_ILLEGAL_ARGUMENT_ERROR;
  }
  return locale;
}

U_NAMESPACE_END
