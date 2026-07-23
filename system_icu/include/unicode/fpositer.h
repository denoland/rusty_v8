// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_FIELD_POSITION_ITERATOR_H_
#define V8_SYSTEM_ICU_FIELD_POSITION_ITERATOR_H_

#include <memory>

#include "unicode/fieldpos.h"
#include "unicode/ufieldpositer.h"

U_NAMESPACE_BEGIN

class FieldPositionIterator : public UObject {
 public:
  FieldPositionIterator();
  FieldPositionIterator(const FieldPositionIterator& other);
  FieldPositionIterator& operator=(const FieldPositionIterator& other);
  virtual ~FieldPositionIterator();

  UBool next(FieldPosition& position);
  UFieldPositionIterator* prepareForFormat(UErrorCode& status);
  void adjustForReplacement(int32_t begin, int32_t end,
                            int32_t replacement_length);

 private:
  struct State;
  std::shared_ptr<State> state_;
  size_t index_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_FIELD_POSITION_ITERATOR_H_
