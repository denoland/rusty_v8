// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.

use crate::support::int;
use crate::Name;

extern "C" {
  fn v8__Name__GetIdentityHash(this: *const Name) -> int;
}

impl Name {
  /// The `String` or `Symbol` specific equivalent of `Data::get_hash()`.
  /// This function is kept around for testing purposes only.
  #[doc(hidden)]
  pub fn get_identity_hash(&self) -> int {
    unsafe { v8__Name__GetIdentityHash(self) }
  }
}
