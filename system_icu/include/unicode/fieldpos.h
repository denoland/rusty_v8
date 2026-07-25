// Copyright 2018-2026 the Deno authors. MIT license.

#ifndef V8_SYSTEM_ICU_FIELD_POSITION_H_
#define V8_SYSTEM_ICU_FIELD_POSITION_H_

#include "unicode/uobject.h"

U_NAMESPACE_BEGIN

class FieldPosition : public UObject {
 public:
  enum {
    DONT_CARE = -1,
  };

  FieldPosition() : field_(DONT_CARE), begin_index_(0), end_index_(0) {}
  explicit FieldPosition(int32_t field)
      : field_(field), begin_index_(0), end_index_(0) {}
  FieldPosition(const FieldPosition&) = default;
  FieldPosition& operator=(const FieldPosition&) = default;
  virtual ~FieldPosition();

  int32_t getField() const { return field_; }
  int32_t getBeginIndex() const { return begin_index_; }
  int32_t getEndIndex() const { return end_index_; }
  void setField(int32_t value) { field_ = value; }
  void setBeginIndex(int32_t value) { begin_index_ = value; }
  void setEndIndex(int32_t value) { end_index_ = value; }

 private:
  int32_t field_;
  int32_t begin_index_;
  int32_t end_index_;
};

U_NAMESPACE_END

#endif  // V8_SYSTEM_ICU_FIELD_POSITION_H_
