// Copyright 2018-2026 the Deno authors. MIT license.

#include <algorithm>
#include <cstdlib>
#include <cstring>
#include <string>
#include <utility>
#include <vector>

#include "unicode/bytestream.h"
#include "unicode/locid.h"
#include "unicode/strenum.h"
#include "unicode/uenum.h"
#include "unicode/uloc.h"
#include "unicode/unistr.h"
#include "unicode/ustring.h"

U_NAMESPACE_BEGIN

namespace {

char* CopyString(const char* value) {
  const size_t length = std::strlen(value);
  auto* result = static_cast<char*>(std::malloc(length + 1));
  if (result != nullptr) {
    std::memcpy(result, value, length + 1);
  }
  return result;
}

std::string ToLanguageTag(const char* locale_id, UErrorCode& status) {
  const int32_t required =
      uloc_toLanguageTag(locale_id, nullptr, 0, true, &status);
  if (status != U_BUFFER_OVERFLOW_ERROR && U_FAILURE(status)) {
    return {};
  }
  status = U_ZERO_ERROR;
  std::string result(required + 1, '\0');
  const int32_t length = uloc_toLanguageTag(locale_id, result.data(),
                                            result.size(), true, &status);
  return U_SUCCESS(status) ? std::string(result.data(), length) : std::string();
}

std::string ParsedKeywordValue(const char* locale_id, const char* keyword,
                               UErrorCode& status) {
  const char* entry = std::strchr(locale_id, '@');
  if (entry == nullptr) {
    return {};
  }
  ++entry;
  const size_t keyword_length = std::strlen(keyword);
  while (*entry != '\0') {
    const char* equals = std::strchr(entry, '=');
    if (equals == nullptr) {
      break;
    }
    const char* end = std::strchr(equals + 1, ';');
    if (end == nullptr) {
      end = locale_id + std::strlen(locale_id);
    }
    if (static_cast<size_t>(equals - entry) == keyword_length &&
        std::memcmp(entry, keyword, keyword_length) == 0) {
      status = U_ZERO_ERROR;
      return std::string(equals + 1, end);
    }
    entry = *end == '\0' ? end : end + 1;
  }
  return {};
}

std::string KeywordValue(const char* locale_id, const char* keyword,
                         UErrorCode& status) {
  const int32_t required =
      uloc_getKeywordValue(locale_id, keyword, nullptr, 0, &status);
  if (status != U_BUFFER_OVERFLOW_ERROR && U_FAILURE(status)) {
    return ParsedKeywordValue(locale_id, keyword, status);
  }
  if (required == 0) {
    return {};
  }
  status = U_ZERO_ERROR;
  std::string result(required + 1, '\0');
  const int32_t length = uloc_getKeywordValue(locale_id, keyword, result.data(),
                                              result.size(), &status);
  if (U_SUCCESS(status)) {
    return std::string(result.data(), length);
  }

  // Older ICU versions can enumerate all keywords in a long locale ID but
  // fail to retrieve values near its capacity limit. The locale has already
  // been normalized, so recover the value from its key=value list directly.
  return ParsedKeywordValue(locale_id, keyword, status);
}

std::string ApplyModernLocaleAliases(std::string locale_id) {
  const size_t keywords = locale_id.find('@');
  const std::string base = locale_id.substr(0, keywords);
  const std::string suffix = keywords == std::string::npos
                                 ? std::string()
                                 : locale_id.substr(keywords);

  // These aliases were added after the ICU version shipped by older supported
  // macOS releases. Keep the small CLDR delta here rather than shipping locale
  // data alongside the executable.
  if (base == "cel__GAULISH") {
    return "xtg" + suffix;
  }
  if (base == "sr_CS") {
    return "sr_Cyrl_RS" + suffix;
  }
  if (base == "hy_SU") {
    return "hy_AM" + suffix;
  }
  if (base == "lv_SU") {
    return "lv_LV" + suffix;
  }

  struct LanguageAlias {
    const char* from;
    const char* to;
    const char* default_script;
    const char* default_region;
  };
  constexpr LanguageAlias kLanguageAliases[] = {
      {"arb", "ar", "", ""},    {"cmn", "zh", "", ""}, {"cnr", "sr", "", "ME"},
      {"kmr", "ku", "", ""},    {"scc", "sr", "", ""}, {"scr", "hr", "", ""},
      {"sh", "sr", "Latn", ""}, {"swh", "sw", "", ""}, {"tl", "fil", "", ""},
      {"uzn", "uz", "", ""},    {"zsm", "ms", "", ""},
  };
  for (const auto& alias : kLanguageAliases) {
    const size_t from_length = std::strlen(alias.from);
    if (base == alias.from || (base.size() > from_length &&
                               base.compare(0, from_length, alias.from) == 0 &&
                               base[from_length] == '_')) {
      UErrorCode status = U_ZERO_ERROR;
      char script[ULOC_SCRIPT_CAPACITY] = {};
      char region[ULOC_COUNTRY_CAPACITY] = {};
      char variant[ULOC_FULLNAME_CAPACITY] = {};
      uloc_getScript(base.c_str(), script, sizeof(script), &status);
      status = U_ZERO_ERROR;
      uloc_getCountry(base.c_str(), region, sizeof(region), &status);
      status = U_ZERO_ERROR;
      uloc_getVariant(base.c_str(), variant, sizeof(variant), &status);

      std::string result(alias.to);
      const char* resolved_script =
          script[0] == 0 ? alias.default_script : script;
      const char* resolved_region =
          region[0] == 0 ? alias.default_region : region;
      if (resolved_script[0] != 0) {
        result += "_";
        result += resolved_script;
      }
      if (resolved_region[0] != 0) {
        result += "_";
        result += resolved_region;
      } else if (variant[0] != 0) {
        result += "_";
      }
      if (variant[0] != 0) {
        result += "_";
        result += variant;
      }
      return result + suffix;
    }
  }
  return locale_id;
}

void LongLocaleToLanguageTag(const Locale& locale, ByteSink& sink,
                             UErrorCode& status) {
  std::string base_tag = ToLanguageTag(locale.getBaseName(), status);
  if (U_FAILURE(status)) {
    return;
  }

  std::vector<std::pair<std::string, std::string>> unicode_keywords;
  constexpr char kPosixSuffix[] = "-u-va-posix";
  if (base_tag.size() >= sizeof(kPosixSuffix) - 1 &&
      base_tag.compare(base_tag.size() - (sizeof(kPosixSuffix) - 1),
                       sizeof(kPosixSuffix) - 1, kPosixSuffix) == 0) {
    base_tag.resize(base_tag.size() - (sizeof(kPosixSuffix) - 1));
    unicode_keywords.emplace_back("va", "posix");
  }

  UEnumeration* keywords = uloc_openKeywords(locale.getName(), &status);
  if (U_FAILURE(status)) {
    return;
  }
  if (keywords != nullptr) {
    int32_t legacy_key_length = 0;
    const char* legacy_key;
    while ((legacy_key = uenum_next(keywords, &legacy_key_length, &status)) !=
           nullptr) {
      const char* unicode_key = uloc_toUnicodeLocaleKey(legacy_key);
      if (unicode_key == nullptr || std::strlen(unicode_key) != 2) {
        status = U_ILLEGAL_ARGUMENT_ERROR;
        break;
      }
      std::string legacy_value =
          KeywordValue(locale.getName(), legacy_key, status);
      if (U_FAILURE(status)) {
        break;
      }
      const char* unicode_value =
          uloc_toUnicodeLocaleType(legacy_key, legacy_value.c_str());
      if (unicode_value == nullptr) {
        status = U_ILLEGAL_ARGUMENT_ERROR;
        break;
      }
      unicode_keywords.emplace_back(unicode_key, unicode_value);
    }
    uenum_close(keywords);
    if (U_FAILURE(status)) {
      return;
    }
  }

  std::sort(unicode_keywords.begin(), unicode_keywords.end());
  std::string result = std::move(base_tag);
  if (!unicode_keywords.empty()) {
    result += "-u";
    for (const auto& [key, value] : unicode_keywords) {
      result += '-';
      result += key;
      if (value != "true") {
        result += '-';
        result += value;
      }
    }
  }
  sink.Append(result.data(), result.size());
}

}  // namespace

Locale& Locale::init(const char* locale_id, UBool canonicalize) {
  if (locale_id == nullptr) {
    locale_id = "";
  }

  UErrorCode status = U_ZERO_ERROR;
  char normalized[ULOC_FULLNAME_CAPACITY] = {};
  const int32_t normalized_length =
      canonicalize
          ? uloc_canonicalize(locale_id, normalized, sizeof(normalized),
                              &status)
          : uloc_getName(locale_id, normalized, sizeof(normalized), &status);

  std::string dynamic_normalized;
  const char* name = normalized;
  if (status == U_BUFFER_OVERFLOW_ERROR ||
      normalized_length >= static_cast<int32_t>(sizeof(normalized))) {
    status = U_ZERO_ERROR;
    dynamic_normalized.resize(normalized_length + 1);
    if (canonicalize) {
      uloc_canonicalize(locale_id, dynamic_normalized.data(),
                        dynamic_normalized.size(), &status);
    } else {
      uloc_getName(locale_id, dynamic_normalized.data(),
                   dynamic_normalized.size(), &status);
    }
    if (U_SUCCESS(status)) {
      dynamic_normalized[normalized_length] = '\0';
    }
    name = dynamic_normalized.c_str();
  } else if (U_SUCCESS(status) && normalized_length >= 0) {
    normalized[normalized_length] = '\0';
  }

  std::string aliased;
  if (U_FAILURE(status)) {
    name = "";
    fIsBogus = true;
  } else {
    fIsBogus = false;
    if (canonicalize) {
      aliased = ApplyModernLocaleAliases(name);
      name = aliased.c_str();
    }
  }

  if (fullName != nullptr && fullName != fullNameBuffer) {
    std::free(fullName);
  }
  if (baseName != nullptr && baseName != fullName &&
      baseName != fullNameBuffer) {
    std::free(baseName);
  }

  const size_t name_length = std::strlen(name);
  if (name_length < sizeof(fullNameBuffer)) {
    std::memcpy(fullNameBuffer, name, name_length + 1);
    fullName = fullNameBuffer;
  } else {
    fullName = CopyString(name);
    if (fullName == nullptr) {
      fullNameBuffer[0] = 0;
      fullName = fullNameBuffer;
      fIsBogus = true;
    }
  }

  status = U_ZERO_ERROR;
  const int32_t base_length = uloc_getBaseName(fullName, nullptr, 0, &status);
  std::string base;
  if (status == U_BUFFER_OVERFLOW_ERROR) {
    status = U_ZERO_ERROR;
    base.resize(base_length + 1);
    uloc_getBaseName(fullName, base.data(), base.size(), &status);
  }
  if (U_FAILURE(status)) {
    base = "";
    fIsBogus = true;
  }
  baseName = CopyString(base.c_str());
  if (baseName == nullptr) {
    baseName = fullName;
    fIsBogus = true;
  }

  status = U_ZERO_ERROR;
  uloc_getLanguage(fullName, language, sizeof(language), &status);
  status = U_ZERO_ERROR;
  uloc_getScript(fullName, script, sizeof(script), &status);
  status = U_ZERO_ERROR;
  uloc_getCountry(fullName, country, sizeof(country), &status);

  char variant[ULOC_FULLNAME_CAPACITY];
  status = U_ZERO_ERROR;
  uloc_getVariant(fullName, variant, sizeof(variant), &status);
  if (U_SUCCESS(status) && variant[0] != 0) {
    const char* location = std::strstr(baseName, variant);
    variantBegin =
        location == nullptr ? std::strlen(baseName) : location - baseName;
  } else {
    variantBegin = std::strlen(baseName);
  }
  return *this;
}

Locale::Locale()
    : variantBegin(0),
      fullName(fullNameBuffer),
      baseName(nullptr),
      fIsBogus(false) {
  fullNameBuffer[0] = 0;
  init(uloc_getDefault(), false);
}

Locale::Locale(const char* language_value, const char* country_value,
               const char* variant_value, const char* keywords_and_values)
    : variantBegin(0),
      fullName(fullNameBuffer),
      baseName(nullptr),
      fIsBogus(false) {
  fullNameBuffer[0] = 0;
  std::string id = language_value == nullptr ? "" : language_value;
  if (country_value != nullptr && country_value[0] != 0) {
    id += '_';
    id += country_value;
  }
  if (variant_value != nullptr && variant_value[0] != 0) {
    id += '_';
    id += variant_value;
  }
  if (keywords_and_values != nullptr && keywords_and_values[0] != 0) {
    id += '@';
    id += keywords_and_values;
  }
  init(id.c_str(), false);
}

Locale::Locale(const Locale& other)
    : variantBegin(0),
      fullName(fullNameBuffer),
      baseName(nullptr),
      fIsBogus(false) {
  fullNameBuffer[0] = 0;
  init(other.getName(), false);
}

Locale::Locale(Locale&& other) noexcept
    : variantBegin(0),
      fullName(fullNameBuffer),
      baseName(nullptr),
      fIsBogus(false) {
  fullNameBuffer[0] = 0;
  init(other.getName(), false);
}

Locale::~Locale() {
  if (baseName != nullptr && baseName != fullName &&
      baseName != fullNameBuffer) {
    std::free(baseName);
  }
  if (fullName != nullptr && fullName != fullNameBuffer) {
    std::free(fullName);
  }
}

Locale& Locale::operator=(const Locale& other) {
  if (this != &other) {
    init(other.getName(), false);
  }
  return *this;
}

Locale& Locale::operator=(Locale&& other) noexcept {
  if (this != &other) {
    init(other.getName(), false);
  }
  return *this;
}

Locale* Locale::clone() const { return new Locale(*this); }

Locale Locale::createFromName(const char* name) {
  return Locale(name, "", "", "");
}

Locale Locale::createCanonical(const char* name) {
  Locale locale(name, "", "", "");
  UErrorCode status = U_ZERO_ERROR;
  locale.canonicalize(status);
  return locale;
}

void Locale::canonicalize(UErrorCode& status) {
  if (U_SUCCESS(status)) {
    init(getName(), true);
    if (isBogus()) {
      status = U_ILLEGAL_ARGUMENT_ERROR;
    }
  }
}

void Locale::addLikelySubtags(UErrorCode& status) {
  if (U_FAILURE(status)) {
    return;
  }
  int32_t required = uloc_addLikelySubtags(getName(), nullptr, 0, &status);
  if (status != U_BUFFER_OVERFLOW_ERROR && U_FAILURE(status)) {
    return;
  }
  status = U_ZERO_ERROR;
  std::string locale_id(required + 1, '\0');
  uloc_addLikelySubtags(getName(), locale_id.data(), locale_id.size(), &status);
  if (U_SUCCESS(status)) {
    init(locale_id.c_str(), false);
  }
}

void Locale::minimizeSubtags(UErrorCode& status) {
  if (U_FAILURE(status)) {
    return;
  }
  int32_t required = uloc_minimizeSubtags(getName(), nullptr, 0, &status);
  if (status != U_BUFFER_OVERFLOW_ERROR && U_FAILURE(status)) {
    return;
  }
  status = U_ZERO_ERROR;
  std::string locale_id(required + 1, '\0');
  uloc_minimizeSubtags(getName(), locale_id.data(), locale_id.size(), &status);
  if (U_SUCCESS(status)) {
    init(locale_id.c_str(), false);
  }
}

UBool Locale::isRightToLeft() const {
  // These minority languages were added to CLDR's right-to-left metadata
  // after the ICU version on older supported macOS releases.
  return (std::strcmp(getLanguage(), "bcc") == 0 ||
          std::strcmp(getLanguage(), "pnb") == 0) ||
         uloc_isRightToLeft(getName());
}

int32_t Locale::getKeywordValue(const char* keyword, char* buffer,
                                int32_t capacity, UErrorCode& status) const {
  return uloc_getKeywordValue(getName(), keyword, buffer, capacity, &status);
}

StringEnumeration* Locale::createKeywords(UErrorCode& status) const {
  UEnumeration* keywords = uloc_openKeywords(getName(), &status);
  return keywords == nullptr ? nullptr : new StringEnumeration(keywords);
}

const char* Locale::getBaseName() const { return baseName; }

bool Locale::operator==(const Locale& other) const {
  return std::strcmp(getName(), other.getName()) == 0;
}

Locale Locale::forLanguageTag(StringPiece tag, UErrorCode& status) {
  if (U_FAILURE(status)) {
    return Locale("", "", "", "");
  }
  std::string input(tag.data(), tag.length());
  const int32_t required =
      uloc_forLanguageTag(input.c_str(), nullptr, 0, nullptr, &status);
  if (status != U_BUFFER_OVERFLOW_ERROR && U_FAILURE(status)) {
    return Locale("", "", "", "");
  }
  status = U_ZERO_ERROR;
  std::string locale_id(required + 1, '\0');
  int32_t parsed_length = 0;
  uloc_forLanguageTag(input.c_str(), locale_id.data(), locale_id.size(),
                      &parsed_length, &status);
  if (U_FAILURE(status) || parsed_length != tag.length()) {
    if (U_SUCCESS(status)) {
      status = U_ILLEGAL_ARGUMENT_ERROR;
    }
    return Locale("", "", "", "");
  }
  return Locale(locale_id.c_str(), "", "", "");
}

void Locale::toLanguageTag(ByteSink& sink, UErrorCode& status) const {
  if (U_FAILURE(status)) {
    return;
  }
  const int32_t required =
      uloc_toLanguageTag(getName(), nullptr, 0, true, &status);
  if (status != U_BUFFER_OVERFLOW_ERROR && U_FAILURE(status)) {
    if (status == U_ILLEGAL_ARGUMENT_ERROR && !isBogus()) {
      status = U_ZERO_ERROR;
      LongLocaleToLanguageTag(*this, sink, status);
    }
    return;
  }
  status = U_ZERO_ERROR;
  std::string tag(required + 1, '\0');
  const int32_t length =
      uloc_toLanguageTag(getName(), tag.data(), tag.size(), true, &status);
  if (U_SUCCESS(status)) {
    sink.Append(tag.data(), length);
  } else if (status == U_ILLEGAL_ARGUMENT_ERROR && !isBogus()) {
    status = U_ZERO_ERROR;
    LongLocaleToLanguageTag(*this, sink, status);
  }
}

void Locale::setUnicodeKeywordValue(StringPiece keyword, StringPiece value,
                                    UErrorCode& status) {
  if (U_FAILURE(status)) {
    return;
  }
  const std::string unicode_key(keyword.data(), keyword.length());
  const std::string unicode_value(value.data(), value.length());
  const char* legacy_key = uloc_toLegacyKey(unicode_key.c_str());
  const char* legacy_value =
      unicode_value.empty()
          ? ""
          : uloc_toLegacyType(unicode_key.c_str(), unicode_value.c_str());
  if (legacy_key == nullptr || legacy_value == nullptr) {
    status = U_ILLEGAL_ARGUMENT_ERROR;
    return;
  }

  int32_t capacity = std::max<int32_t>(
      ULOC_FULLNAME_CAPACITY,
      std::strlen(getName()) + unicode_key.size() + unicode_value.size() + 32);
  for (;;) {
    std::string result(capacity, '\0');
    std::strcpy(result.data(), getName());
    const int32_t length = uloc_setKeywordValue(
        legacy_key, legacy_value, result.data(), result.size(), &status);
    if (status == U_BUFFER_OVERFLOW_ERROR) {
      status = U_ZERO_ERROR;
      capacity = length + 1;
      continue;
    }
    if (U_SUCCESS(status)) {
      init(result.c_str(), false);
    }
    return;
  }
}

void Locale::getUnicodeKeywordValue(StringPiece keyword, ByteSink& sink,
                                    UErrorCode& status) const {
  if (U_FAILURE(status)) {
    return;
  }
  const std::string unicode_key(keyword.data(), keyword.length());
  const char* legacy_key = uloc_toLegacyKey(unicode_key.c_str());
  if (legacy_key == nullptr) {
    status = U_ILLEGAL_ARGUMENT_ERROR;
    return;
  }
  const int32_t required =
      uloc_getKeywordValue(getName(), legacy_key, nullptr, 0, &status);
  if (status != U_BUFFER_OVERFLOW_ERROR && U_FAILURE(status)) {
    return;
  }
  if (required == 0) {
    status = U_ILLEGAL_ARGUMENT_ERROR;
    return;
  }
  status = U_ZERO_ERROR;
  std::string value(required + 1, '\0');
  const int32_t length = uloc_getKeywordValue(
      getName(), legacy_key, value.data(), value.size(), &status);
  if (U_SUCCESS(status)) {
    value.resize(length);
    const char* unicode_value =
        uloc_toUnicodeLocaleType(unicode_key.c_str(), value.c_str());
    if (unicode_value == nullptr) {
      status = U_ILLEGAL_ARGUMENT_ERROR;
      return;
    }
    sink.Append(unicode_value, std::strlen(unicode_value));
  }
}

const Locale& Locale::getDefault() {
  static Locale locale(uloc_getDefault(), "", "", "");
  return locale;
}

const Locale& Locale::getRoot() {
  static Locale locale("");
  return locale;
}

void Locale::setDefault(const Locale& new_locale, UErrorCode& status) {
  uloc_setDefault(new_locale.getName(), &status);
}

UClassID Locale::getStaticClassID() {
  static char class_id = 0;
  return &class_id;
}

UClassID Locale::getDynamicClassID() const { return getStaticClassID(); }

Locale::Iterator::~Iterator() = default;

UnicodeString& UnicodeString::toUpper() {
  UErrorCode status = U_ZERO_ERROR;
  const int32_t required =
      u_strToUpper(nullptr, 0, reinterpret_cast<const UChar*>(getBuffer()),
                   length(), nullptr, &status);
  if (status != U_BUFFER_OVERFLOW_ERROR && U_FAILURE(status)) {
    return *this;
  }
  status = U_ZERO_ERROR;
  std::u16string result(required, u'\0');
  u_strToUpper(reinterpret_cast<UChar*>(result.data()), result.size(),
               reinterpret_cast<const UChar*>(getBuffer()), length(), nullptr,
               &status);
  if (U_SUCCESS(status)) {
    *this = UnicodeString(result);
  }
  return *this;
}

int8_t UnicodeString::doCaseCompare(int32_t start, int32_t requested_length,
                                    const char16_t* source,
                                    int32_t source_start, int32_t source_length,
                                    uint32_t options) const {
  if (isBogus()) {
    return -1;
  }

  start = std::clamp(start, 0, length());
  const int32_t actual_length =
      std::clamp(requested_length, 0, length() - start);
  if (source == nullptr) {
    static const char16_t empty[] = u"";
    source = empty;
    source_start = 0;
    source_length = 0;
  } else {
    source += std::max(source_start, 0);
  }
  UErrorCode status = U_ZERO_ERROR;
  const int32_t result = u_strCaseCompare(
      reinterpret_cast<const UChar*>(getBuffer() + start), actual_length,
      reinterpret_cast<const UChar*>(source), source_length, options, &status);
  if (U_FAILURE(status)) {
    return -1;
  }
  return result < 0 ? -1 : result > 0 ? 1 : 0;
}

U_NAMESPACE_END

extern "C" {

void* uprv_malloc(size_t size) { return std::malloc(size); }

void uprv_free(void* memory) { std::free(memory); }

}  // extern "C"
