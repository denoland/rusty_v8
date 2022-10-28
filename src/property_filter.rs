#[repr(C)]
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct PropertyFilter(u32);

pub const ALL_PROPERTIES: PropertyFilter = PropertyFilter(0);

pub const ONLY_WRITABLE: PropertyFilter = PropertyFilter(1);

pub const ONLY_ENUMERABLE: PropertyFilter = PropertyFilter(2);

pub const ONLY_CONFIGURABLE: PropertyFilter = PropertyFilter(4);

pub const SKIP_STRINGS: PropertyFilter = PropertyFilter(8);

pub const SKIP_SYMBOLS: PropertyFilter = PropertyFilter(16);

impl PropertyFilter {
  /// Test if all property filters are set.
  #[inline(always)]
  pub fn is_all_properties(&self) -> bool {
    *self == ALL_PROPERTIES
  }

  /// Test if the only-writable property filter is set.
  #[inline(always)]
  pub fn is_only_writable(&self) -> bool {
    self.has(ONLY_WRITABLE)
  }

  /// Test if the only-enumerable property filter is set.
  #[inline(always)]
  pub fn is_only_enumerable(&self) -> bool {
    self.has(ONLY_ENUMERABLE)
  }

  /// Test if the only-configurable property filter is set.
  #[inline(always)]
  pub fn is_only_configurable(&self) -> bool {
    self.has(ONLY_CONFIGURABLE)
  }

  /// Test if the skip-strings property filter is set.
  #[inline(always)]
  pub fn is_skip_strings(&self) -> bool {
    self.has(SKIP_STRINGS)
  }

  /// Test if the skip-symbols property filter is set.
  #[inline(always)]
  pub fn is_skip_symbols(&self) -> bool {
    self.has(SKIP_SYMBOLS)
  }

  #[inline(always)]
  fn has(&self, that: Self) -> bool {
    let Self(lhs) = self;
    let Self(rhs) = that;
    0 != lhs & rhs
  }
}

// Identical to #[derive(Default)] but arguably clearer when made explicit.
impl Default for PropertyFilter {
  fn default() -> Self {
    ALL_PROPERTIES
  }
}

impl std::ops::BitOr for PropertyFilter {
  type Output = Self;

  fn bitor(self, Self(rhs): Self) -> Self {
    let Self(lhs) = self;
    Self(lhs | rhs)
  }
}

#[test]
fn test_attr() {
  assert!(ALL_PROPERTIES.is_all_properties());
  assert!(!ALL_PROPERTIES.is_only_writable());
  assert!(!ALL_PROPERTIES.is_only_enumerable());
  assert!(!ALL_PROPERTIES.is_only_configurable());
  assert!(!ALL_PROPERTIES.is_skip_strings());
  assert!(!ALL_PROPERTIES.is_skip_symbols());

  assert!(!ONLY_WRITABLE.is_all_properties());
  assert!(ONLY_WRITABLE.is_only_writable());
  assert!(!ONLY_WRITABLE.is_only_enumerable());
  assert!(!ONLY_WRITABLE.is_only_configurable());
  assert!(!ONLY_WRITABLE.is_skip_strings());
  assert!(!ONLY_WRITABLE.is_skip_symbols());

  assert!(!ONLY_ENUMERABLE.is_all_properties());
  assert!(!ONLY_ENUMERABLE.is_only_writable());
  assert!(ONLY_ENUMERABLE.is_only_enumerable());
  assert!(!ONLY_ENUMERABLE.is_only_configurable());
  assert!(!ONLY_ENUMERABLE.is_skip_strings());
  assert!(!ONLY_ENUMERABLE.is_skip_symbols());

  assert!(!ONLY_CONFIGURABLE.is_all_properties());
  assert!(!ONLY_CONFIGURABLE.is_only_writable());
  assert!(!ONLY_CONFIGURABLE.is_only_enumerable());
  assert!(ONLY_CONFIGURABLE.is_only_configurable());
  assert!(!ONLY_CONFIGURABLE.is_skip_strings());
  assert!(!ONLY_CONFIGURABLE.is_skip_symbols());

  assert!(!SKIP_STRINGS.is_all_properties());
  assert!(!SKIP_STRINGS.is_only_writable());
  assert!(!SKIP_STRINGS.is_only_enumerable());
  assert!(!SKIP_STRINGS.is_only_configurable());
  assert!(SKIP_STRINGS.is_skip_strings());
  assert!(!SKIP_STRINGS.is_skip_symbols());

  assert!(!SKIP_SYMBOLS.is_all_properties());
  assert!(!SKIP_SYMBOLS.is_only_writable());
  assert!(!SKIP_SYMBOLS.is_only_enumerable());
  assert!(!SKIP_SYMBOLS.is_only_configurable());
  assert!(!SKIP_SYMBOLS.is_skip_strings());
  assert!(SKIP_SYMBOLS.is_skip_symbols());

  assert_eq!(ALL_PROPERTIES, Default::default());
  assert_eq!(ONLY_WRITABLE, ALL_PROPERTIES | ONLY_WRITABLE);

  let attr = ONLY_WRITABLE | ONLY_WRITABLE | SKIP_STRINGS;
  assert!(!attr.is_all_properties());
  assert!(attr.is_only_writable());
  assert!(!attr.is_only_enumerable());
  assert!(!attr.is_only_configurable());
  assert!(attr.is_skip_strings());
  assert!(!attr.is_skip_symbols());
}
