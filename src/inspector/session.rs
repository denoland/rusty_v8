use super::StringView;
use crate::support::Delete;
use crate::support::Opaque;

extern "C" {
  fn v8_inspector__V8InspectorSession__DELETE(
    this: &'static mut V8InspectorSession,
  );
  fn v8_inspector__V8InspectorSession__dispatchProtocolMessage(
    session: *mut V8InspectorSession,
    message: &StringView,
  );
  fn v8_inspector__V8InspectorSession__schedulePauseOnNextStatement(
    session: *mut V8InspectorSession,
    break_reason: &StringView,
    break_details: &StringView,
  );
}

#[repr(C)]
pub struct V8InspectorSession(Opaque);

impl V8InspectorSession {
  pub fn dispatch_protocol_message(&mut self, message: &StringView) {
    unsafe {
      v8_inspector__V8InspectorSession__dispatchProtocolMessage(self, message)
    }
  }

  pub fn schedule_pause_on_next_statement(
    &mut self,
    reason: &StringView,
    detail: &StringView,
  ) {
    unsafe {
      v8_inspector__V8InspectorSession__schedulePauseOnNextStatement(
        self, reason, detail,
      )
    }
  }
}

impl Delete for V8InspectorSession {
  fn delete(&'static mut self) {
    unsafe { v8_inspector__V8InspectorSession__DELETE(self) };
  }
}
