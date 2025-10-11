#[cfg(test)]
mod test_sandbox_use {
  #[test]
  #[cfg(feature = "v8_enable_sandbox")]
  fn test_sandbox_on() {
    assert!(v8::V8::is_sandboxed());
  }

  #[test]
  #[cfg(not(feature = "v8_enable_sandbox"))]
  fn test_sandbox_off() {
    assert!(!v8::V8::is_sandboxed());
  }
}
