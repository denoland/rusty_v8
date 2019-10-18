
extern "C" {
using namespace v8_inspector;

void v8_inspector__StringBuffer__DELETE(StringBuffer& self) {
  delete &self;
}

const StringView& v8_inspector__StringBuffer__string(StringBuffer& self) {
  return self.string();
}

StringBuffer* v8_inspector__StringBuffer__create(const StringView& source) {
  return StringBuffer::create(source).release();
}
}