// Copyright 2018-2026 the Deno authors. MIT license.

#include <mutex>
#include <vector>

#include "unicode/fpositer.h"

U_NAMESPACE_BEGIN

FieldPosition::~FieldPosition() = default;

struct FieldPositionIterator::State {
  struct Entry {
    int32_t field;
    int32_t begin;
    int32_t end;
  };

  ~State() { ufieldpositer_close(iterator); }

  UFieldPositionIterator* iterator = nullptr;
  std::vector<Entry> entries;
  bool exhausted = false;
  std::mutex mutex;
};

FieldPositionIterator::FieldPositionIterator()
    : state_(std::make_shared<State>()), index_(0) {
  UErrorCode status = U_ZERO_ERROR;
  state_->iterator = ufieldpositer_open(&status);
}

FieldPositionIterator::FieldPositionIterator(const FieldPositionIterator& other)
    : state_(other.state_), index_(other.index_) {}

FieldPositionIterator& FieldPositionIterator::operator=(
    const FieldPositionIterator& other) {
  state_ = other.state_;
  index_ = other.index_;
  return *this;
}

FieldPositionIterator::~FieldPositionIterator() = default;

UFieldPositionIterator* FieldPositionIterator::prepareForFormat(
    UErrorCode& status) {
  if (U_FAILURE(status) || state_->iterator == nullptr) {
    if (U_SUCCESS(status)) {
      status = U_MEMORY_ALLOCATION_ERROR;
    }
    return nullptr;
  }
  std::lock_guard<std::mutex> lock(state_->mutex);
  state_->entries.clear();
  state_->exhausted = false;
  index_ = 0;
  return state_->iterator;
}

void FieldPositionIterator::adjustForReplacement(int32_t begin, int32_t end,
                                                 int32_t replacement_length) {
  std::lock_guard<std::mutex> lock(state_->mutex);
  while (!state_->exhausted) {
    int32_t entry_begin = 0;
    int32_t entry_end = 0;
    int32_t field =
        ufieldpositer_next(state_->iterator, &entry_begin, &entry_end);
    if (field < 0) {
      state_->exhausted = true;
    } else {
      state_->entries.push_back({field, entry_begin, entry_end});
    }
  }

  const int32_t delta = replacement_length - (end - begin);
  for (State::Entry& entry : state_->entries) {
    if (entry.begin >= end) {
      entry.begin += delta;
    } else if (entry.begin > begin) {
      entry.begin = begin;
    }
    if (entry.end >= end) {
      entry.end += delta;
    } else if (entry.end > begin) {
      entry.end = begin + replacement_length;
    }
  }
}

UBool FieldPositionIterator::next(FieldPosition& position) {
  std::lock_guard<std::mutex> lock(state_->mutex);
  if (index_ == state_->entries.size() && !state_->exhausted) {
    int32_t begin = 0;
    int32_t end = 0;
    int32_t field = ufieldpositer_next(state_->iterator, &begin, &end);
    if (field < 0) {
      state_->exhausted = true;
    } else {
      state_->entries.push_back({field, begin, end});
    }
  }
  if (index_ >= state_->entries.size()) {
    return false;
  }
  const State::Entry& entry = state_->entries[index_++];
  position.setField(entry.field);
  position.setBeginIndex(entry.begin);
  position.setEndIndex(entry.end);
  return true;
}

U_NAMESPACE_END
