use rusty_v8 as v8;

#[test]
fn wee8() {
  use v8::wee8::*;
  unsafe {
    let engine = wasm_engine_new();
    let store = wasm_store_new(engine);
    wasm_store_delete(store);
    wasm_engine_delete(engine);
  }
}
