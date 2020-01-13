use super::string_view::StringView;
use crate::support::Delete;

extern "C" {

  // -> *mut v8_inspector::V8InspectorSession
}

pub struct V8InspectorSession {}

impl V8InspectorSession {
  pub fn dispatch_protocol_message(&mut self, message: &StringView) {
    todo!()
  }

  pub fn schedule_pause_on_next_statement(&mut self) {
    todo!()
  }
}

impl Delete for V8InspectorSession {
  fn delete(&'static mut self) {
    todo!()
  }
}
