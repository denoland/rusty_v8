// Copyright 2018-2026 the Deno authors. MIT license.

#include <cctype>
#include <cstring>
#include <string>
#include <utility>
#include <vector>

#include "unicode/localematcher.h"
#include "unicode/uenum.h"
#include "unicode/uloc.h"

U_NAMESPACE_BEGIN

namespace {

size_t FindExtension(const std::string& tag) {
  for (size_t index = 0; index + 2 < tag.size(); ++index) {
    if (tag[index] == '-' && tag[index + 2] == '-' &&
        std::isalnum(static_cast<unsigned char>(tag[index + 1]))) {
      return index;
    }
  }
  return std::string::npos;
}

Locale AddDesiredExtensions(const Locale& supported, const Locale& desired,
                            UErrorCode& status) {
  std::string desired_tag = desired.toLanguageTag<std::string>(status);
  if (U_FAILURE(status)) {
    return supported;
  }
  const size_t extension = FindExtension(desired_tag);
  if (extension == std::string::npos) {
    return supported;
  }

  std::string result = supported.toLanguageTag<std::string>(status);
  if (U_FAILURE(status)) {
    return supported;
  }
  result.append(desired_tag, extension);
  return Locale::forLanguageTag(result, status);
}

int32_t MatchIndex(const Locale& desired,
                   const std::vector<Locale>& supported) {
  if (supported.empty()) {
    return -1;
  }
  std::vector<const char*> names;
  names.reserve(supported.size());
  for (const Locale& locale : supported) {
    names.push_back(locale.getName());
  }
  UErrorCode status = U_ZERO_ERROR;
  UEnumeration* available = uenum_openCharStringsEnumeration(
      names.data(), static_cast<int32_t>(names.size()), &status);
  if (U_FAILURE(status) || available == nullptr) {
    return -1;
  }

  char matched[ULOC_FULLNAME_CAPACITY] = {};
  UAcceptResult result = ULOC_ACCEPT_FAILED;
  const char* desired_names[] = {desired.getName()};
  uloc_acceptLanguage(matched, sizeof(matched), &result, desired_names, 1,
                      available, &status);
  uenum_close(available);
  if (U_FAILURE(status) || result == ULOC_ACCEPT_FAILED) {
    return -1;
  }

  for (int32_t index = 0; index < static_cast<int32_t>(supported.size());
       ++index) {
    if (std::strcmp(matched, supported[index].getName()) == 0) {
      return index;
    }
  }
  return -1;
}

}  // namespace

LocaleMatcher::Result::Result(const Locale& desired, const Locale& supported,
                              int32_t desired_index, int32_t supported_index,
                              bool has_desired, bool has_supported)
    : desired_(desired),
      supported_(supported),
      desired_index_(desired_index),
      supported_index_(supported_index),
      has_desired_(has_desired),
      has_supported_(has_supported) {}

LocaleMatcher::Result::Result(Result&& other) noexcept = default;

LocaleMatcher::Result& LocaleMatcher::Result::operator=(
    Result&& other) noexcept = default;

LocaleMatcher::Result::~Result() = default;

Locale LocaleMatcher::Result::makeResolvedLocale(UErrorCode& status) const {
  if (U_FAILURE(status)) {
    return Locale();
  }
  // The desired locale carries the Unicode and transformed extensions which
  // must survive resolution. Its likely-subtag match has already selected a
  // supported locale before this point.
  if (has_desired_ && supported_index_ >= 0) {
    return AddDesiredExtensions(supported_, desired_, status);
  }
  return has_supported_ ? supported_ : Locale::getRoot();
}

LocaleMatcher::Builder::Builder() : default_(), has_default_(false) {}

LocaleMatcher::Builder::~Builder() = default;

LocaleMatcher::Builder& LocaleMatcher::Builder::addSupportedLocale(
    const Locale& locale) {
  supported_.push_back(locale);
  return *this;
}

LocaleMatcher::Builder& LocaleMatcher::Builder::setDefaultLocale(
    const Locale* locale) {
  if (locale == nullptr) {
    has_default_ = false;
  } else {
    default_ = *locale;
    has_default_ = true;
  }
  return *this;
}

LocaleMatcher LocaleMatcher::Builder::build(UErrorCode& status) const {
  if (U_FAILURE(status)) {
    return LocaleMatcher({}, default_, has_default_);
  }
  return LocaleMatcher(supported_, default_, has_default_);
}

LocaleMatcher::LocaleMatcher(const std::vector<Locale>& supported,
                             const Locale& default_locale, bool has_default)
    : supported_(supported),
      default_(default_locale),
      has_default_(has_default) {}

LocaleMatcher::LocaleMatcher(LocaleMatcher&& other) noexcept = default;

LocaleMatcher& LocaleMatcher::operator=(LocaleMatcher&& other) noexcept =
    default;

LocaleMatcher::~LocaleMatcher() = default;

LocaleMatcher::Result LocaleMatcher::match(const Locale& desired,
                                           int32_t desired_index) const {
  const int32_t best_index = MatchIndex(desired, supported_);
  if (best_index >= 0) {
    return Result(desired, supported_[best_index], desired_index, best_index,
                  true, true);
  }
  if (has_default_) {
    return Result(desired, default_, desired_index, -1, true, true);
  }
  if (!supported_.empty()) {
    return Result(desired, supported_.front(), desired_index, -1, true, true);
  }
  return Result(desired, Locale::getRoot(), desired_index, -1, true, false);
}

LocaleMatcher::Result LocaleMatcher::getBestMatchResult(
    const Locale& desired, UErrorCode& status) const {
  if (U_FAILURE(status)) {
    return Result(Locale::getRoot(), Locale::getRoot(), -1, -1, false, false);
  }
  return match(desired, 0);
}

LocaleMatcher::Result LocaleMatcher::getBestMatchResult(
    Locale::Iterator& desired, UErrorCode& status) const {
  if (U_FAILURE(status)) {
    return Result(Locale::getRoot(), Locale::getRoot(), -1, -1, false, false);
  }

  int32_t index = 0;
  while (desired.hasNext()) {
    const Locale& locale = desired.next();
    Result result = match(locale, index++);
    if (result.getSupportedIndex() >= 0) {
      return result;
    }
  }

  if (has_default_) {
    return Result(Locale::getRoot(), default_, -1, -1, false, true);
  }
  if (!supported_.empty()) {
    return Result(Locale::getRoot(), supported_.front(), -1, -1, false, true);
  }
  return Result(Locale::getRoot(), Locale::getRoot(), -1, -1, false, false);
}

U_NAMESPACE_END
