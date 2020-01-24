use crate::support::CxxVTable;

#[repr(C)]
pub struct V8StackTrace {
  _cxx_vtable: CxxVTable,
}

// TODO(bnoordhuis) This needs to be fleshed out more but that can wait
// until it's actually needed.
