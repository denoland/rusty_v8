// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_LIST_FORMATTER_H_
#define V8_SYSTEM_ICU_LIST_FORMATTER_H_

#include "unicode/formattedvalue.h"
#include "unicode/locid.h"
#include "unicode/ulistformatter.h"
#include "unicode/uobject.h"

U_NAMESPACE_BEGIN

class FormattedList : public UMemory, public CFormattedValue {
 public:
  FormattedList();
  explicit FormattedList(UFormattedList* result);
  FormattedList(FormattedList&& other) noexcept;
  FormattedList& operator=(FormattedList&& other) noexcept;
  virtual ~FormattedList() override;

 protected:
  virtual const UFormattedValue* asUFormattedValue(
      UErrorCode& status) const override;

 private:
  UFormattedList* result_;
};

class ListFormatter : public UObject {
 public:
  explicit ListFormatter(UListFormatter* formatter);
  virtual ~ListFormatter();

  static ListFormatter* createInstance(const Locale& locale,
                                       UListFormatterType type,
                                       UListFormatterWidth width,
                                       UErrorCode& status);
  FormattedList formatStringsToValue(const UnicodeString items[],
                                     int32_t item_count,
                                     UErrorCode& status) const;

 private:
  UListFormatter* formatter_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_LIST_FORMATTER_H_
