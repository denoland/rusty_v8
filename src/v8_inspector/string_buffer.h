
extern "C" {
void v8_inspector__StringBuffer__DELETE(::v8_inspector::StringBuffer& self) {
  delete &self;
}

const ::v8_inspector::StringView& v8_inspector__StringBuffer__string(
    ::v8_inspector::StringBuffer& self) {
  return self.string();
}

::v8_inspector::StringBuffer* v8_inspector__StringBuffer__create(
    const ::v8_inspector::StringView& source) {
  return ::v8_inspector::StringBuffer::create(source).release();
}
}