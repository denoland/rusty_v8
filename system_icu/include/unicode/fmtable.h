// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_FORMATTABLE_H_
#define V8_SYSTEM_ICU_FORMATTABLE_H_

#include <string>

#include "unicode/stringpiece.h"
#include "unicode/uobject.h"

U_NAMESPACE_BEGIN

class Formattable : public UObject {
 public:
  Formattable();
  explicit Formattable(double value);
  Formattable(StringPiece decimal, UErrorCode& status);
  Formattable(const Formattable& other);
  Formattable& operator=(const Formattable& other);
  virtual ~Formattable();

  bool isDecimal() const { return is_decimal_; }
  double getDouble() const { return double_value_; }
  const std::string& getDecimal() const { return decimal_value_; }

 private:
  bool is_decimal_;
  double double_value_;
  std::string decimal_value_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_FORMATTABLE_H_
