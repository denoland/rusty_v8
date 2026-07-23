// Copyright 2018-2026 the Deno authors. MIT license.

#include <cstring>

#include "unicode/numsys.h"

U_NAMESPACE_BEGIN
namespace {

bool HasExplicitNumberingSystem(const Locale& locale) {
  UErrorCode status = U_ZERO_ERROR;
  char value[ULOC_KEYWORD_AND_VALUES_CAPACITY] = {};
  const int32_t length =
      locale.getKeywordValue("numbers", value, sizeof(value), status);
  return U_SUCCESS(status) && length > 0;
}

}  // namespace

NumberingSystem::NumberingSystem(UNumberingSystem* numbering_system)
    : numbering_system_(numbering_system) {}

NumberingSystem::~NumberingSystem() { unumsys_close(numbering_system_); }

NumberingSystem* NumberingSystem::createInstance(const Locale& locale,
                                                 UErrorCode& status) {
  UNumberingSystem* numbering_system =
      std::strcmp(locale.getLanguage(), "bn") == 0 &&
              !HasExplicitNumberingSystem(locale)
          ? unumsys_openByName("beng", &status)
          : unumsys_open(locale.getName(), &status);
  return numbering_system == nullptr ? nullptr
                                     : new NumberingSystem(numbering_system);
}

NumberingSystem* NumberingSystem::createInstanceByName(const char* name,
                                                       UErrorCode& status) {
  UNumberingSystem* numbering_system = unumsys_openByName(name, &status);
  return numbering_system == nullptr ? nullptr
                                     : new NumberingSystem(numbering_system);
}

StringEnumeration* NumberingSystem::getAvailableNames(UErrorCode& status) {
  UEnumeration* names = unumsys_openAvailableNames(&status);
  return names == nullptr ? nullptr : new StringEnumeration(names);
}

const char* NumberingSystem::getName() const {
  return unumsys_getName(numbering_system_);
}

UBool NumberingSystem::isAlgorithmic() const {
  return unumsys_isAlgorithmic(numbering_system_);
}

U_NAMESPACE_END
