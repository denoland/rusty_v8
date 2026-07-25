// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_FORMATTED_VALUE_H_
#define V8_SYSTEM_ICU_FORMATTED_VALUE_H_

#include "unicode/appendable.h"
#include "unicode/uformattedvalue.h"
#include "unicode/unistr.h"
#include "unicode/uobject.h"

U_NAMESPACE_BEGIN

class ConstrainedFieldPosition : public UMemory {
 public:
  ConstrainedFieldPosition();
  ~ConstrainedFieldPosition();

  void reset();
  void constrainCategory(int32_t category);
  void constrainField(int32_t category, int32_t field);

  int32_t getCategory() const;
  int32_t getField() const;
  int32_t getStart() const;
  int32_t getLimit() const;

  UConstrainedFieldPosition* toUConstrainedFieldPosition() const {
    return field_position_;
  }

 private:
  UConstrainedFieldPosition* field_position_;
};

class FormattedValue {
 public:
  virtual ~FormattedValue();

  virtual UnicodeString toString(UErrorCode& status) const = 0;
  virtual UnicodeString toTempString(UErrorCode& status) const = 0;
  virtual Appendable& appendTo(Appendable& appendable,
                               UErrorCode& status) const = 0;
  virtual UBool nextPosition(ConstrainedFieldPosition& field_position,
                             UErrorCode& status) const = 0;
};

class CFormattedValue : public FormattedValue {
 public:
  virtual UnicodeString toString(UErrorCode& status) const override;
  virtual UnicodeString toTempString(UErrorCode& status) const override;
  virtual Appendable& appendTo(Appendable& appendable,
                               UErrorCode& status) const override;
  virtual UBool nextPosition(ConstrainedFieldPosition& field_position,
                             UErrorCode& status) const override;

 protected:
  virtual const UFormattedValue* asUFormattedValue(
      UErrorCode& status) const = 0;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_FORMATTED_VALUE_H_
