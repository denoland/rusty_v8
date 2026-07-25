// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_LOCALE_MATCHER_H_
#define V8_SYSTEM_ICU_LOCALE_MATCHER_H_

#include <vector>

#include "unicode/locid.h"
#include "unicode/uobject.h"

U_NAMESPACE_BEGIN

class LocaleMatcher : public UMemory {
 public:
  class Result : public UMemory {
   public:
    Result(Result&& other) noexcept;
    Result& operator=(Result&& other) noexcept;
    ~Result();

    int32_t getSupportedIndex() const { return supported_index_; }
    int32_t getDesiredIndex() const { return desired_index_; }
    Locale makeResolvedLocale(UErrorCode& status) const;

   private:
    Result(const Locale& desired, const Locale& supported,
           int32_t desired_index, int32_t supported_index, bool has_desired,
           bool has_supported);

    Locale desired_;
    Locale supported_;
    int32_t desired_index_;
    int32_t supported_index_;
    bool has_desired_;
    bool has_supported_;

    friend class LocaleMatcher;
  };

  class Builder : public UMemory {
   public:
    Builder();
    ~Builder();

    Builder& addSupportedLocale(const Locale& locale);
    Builder& setDefaultLocale(const Locale* locale);
    LocaleMatcher build(UErrorCode& status) const;

   private:
    std::vector<Locale> supported_;
    Locale default_;
    bool has_default_;
  };

  LocaleMatcher(LocaleMatcher&& other) noexcept;
  LocaleMatcher& operator=(LocaleMatcher&& other) noexcept;
  ~LocaleMatcher();

  Result getBestMatchResult(const Locale& desired, UErrorCode& status) const;
  Result getBestMatchResult(Locale::Iterator& desired,
                            UErrorCode& status) const;

 private:
  LocaleMatcher(const std::vector<Locale>& supported,
                const Locale& default_locale, bool has_default);

  Result match(const Locale& desired, int32_t desired_index) const;

  std::vector<Locale> supported_;
  Locale default_;
  bool has_default_;

  friend class Builder;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_LOCALE_MATCHER_H_
