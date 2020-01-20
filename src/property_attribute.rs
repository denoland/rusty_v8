#[repr(C)]
#[derive(Debug, Eq, PartialEq)]
pub struct PropertyAttribute(u32);

/// No property attributes.
pub const NONE: PropertyAttribute = PropertyAttribute(0);

/// Not writable. Corresponds to
/// `Object.defineProperty(o, "p", { writable: false })`.
pub const READ_ONLY: PropertyAttribute = PropertyAttribute(1);

/// Not enumerable. Corresponds to
/// `Object.defineProperty(o, "p", { enumerable: false })`.
pub const DONT_ENUM: PropertyAttribute = PropertyAttribute(2);

/// Not configurable. Corresponds to
/// `Object.defineProperty(o, "p", { configurable: false })`.
pub const DONT_DELETE: PropertyAttribute = PropertyAttribute(4);

impl PropertyAttribute {
  /// Test if no property attributes are set.
  pub fn is_none(&self) -> bool {
    *self == NONE
  }

  /// Test if the read-only property attribute is set.
  pub fn is_read_only(&self) -> bool {
    self.has(READ_ONLY)
  }

  /// Test if the non-enumerable property attribute is set.
  pub fn is_dont_enum(&self) -> bool {
    self.has(DONT_ENUM)
  }

  /// Test if the non-configurable property attribute is set.
  pub fn is_dont_delete(&self) -> bool {
    self.has(DONT_DELETE)
  }

  fn has(&self, that: Self) -> bool {
    let Self(lhs) = self;
    let Self(rhs) = that;
    0 != lhs & rhs
  }
}

// Identical to #[derive(Default)] but arguably clearer when made explicit.
impl Default for PropertyAttribute {
  fn default() -> Self {
    NONE
  }
}

impl std::ops::Add for PropertyAttribute {
  type Output = Self;

  fn add(self, Self(rhs): Self) -> Self {
    let Self(lhs) = self;
    Self(lhs + rhs)
  }
}

#[test]
fn test_attr() {
  assert!(NONE.is_none());
  assert!(!NONE.is_read_only());
  assert!(!NONE.is_dont_enum());
  assert!(!NONE.is_dont_delete());

  assert!(!READ_ONLY.is_none());
  assert!(READ_ONLY.is_read_only());
  assert!(!READ_ONLY.is_dont_enum());
  assert!(!READ_ONLY.is_dont_delete());

  assert!(!DONT_ENUM.is_none());
  assert!(!DONT_ENUM.is_read_only());
  assert!(DONT_ENUM.is_dont_enum());
  assert!(!DONT_ENUM.is_dont_delete());

  assert!(!DONT_DELETE.is_none());
  assert!(!DONT_DELETE.is_read_only());
  assert!(!DONT_DELETE.is_dont_enum());
  assert!(DONT_DELETE.is_dont_delete());

  assert_eq!(NONE, Default::default());
  assert_eq!(READ_ONLY, NONE + READ_ONLY);

  let attr = READ_ONLY + DONT_ENUM;
  assert!(!attr.is_none());
  assert!(attr.is_read_only());
  assert!(attr.is_dont_enum());
  assert!(!attr.is_dont_delete());
}
