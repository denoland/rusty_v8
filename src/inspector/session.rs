use super::StringBuffer;
use crate::support::Delete;
use crate::support::Opaque;

extern "C" {

  fn v8_inspector__V8InspectorSession__DispatchProtocolMessage(
    session: *mut V8InspectorSession,
    message: *mut StringBuffer,
  );

  fn v8_inspector__V8InspectorSession__SchedulePauseOnNextStatement(
    session: *mut V8InspectorSession,
    break_reason: *mut StringBuffer,
    break_details: *mut StringBuffer,
  );
}

#[repr(C)]
pub struct V8InspectorSession(Opaque);

impl V8InspectorSession {
  pub fn dispatch_protocol_message(&mut self, message: &mut StringBuffer) {
    unsafe {
      v8_inspector__V8InspectorSession__DispatchProtocolMessage(
        self,
        &mut *message,
      )
    }
  }

  pub fn schedule_pause_on_next_statement(
    &mut self,
    reason: &mut StringBuffer,
    detail: &mut StringBuffer,
  ) {
    unsafe {
      v8_inspector__V8InspectorSession__SchedulePauseOnNextStatement(
        self,
        &mut *reason,
        &mut *detail,
      )
    }
  }
}

impl Delete for V8InspectorSession {
  fn delete(&'static mut self) {
    todo!()
  }
}
