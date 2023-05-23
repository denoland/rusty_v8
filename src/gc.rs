/// Applications can register callback functions which will be called before and
/// after certain garbage collection operations.  Allocations are not allowed in
/// the callback functions, you therefore cannot manipulate objects (set or
/// delete properties for example) since it is possible such operations will
/// result in the allocation of objects.
#[repr(C)]
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct GCType(u32);

impl GCType {
  pub const SCAVENGE: Self = Self(1 << 0);

  pub const MINOR_MARK_COMPACT: Self = Self(1 << 1);

  pub const MARK_SWEEP_COMPACT: Self = Self(1 << 2);

  pub const INCREMENTAL_MARKING: Self = Self(1 << 3);

  pub const PROCESS_WEAK_CALLBACKS: Self = Self(1 << 4);

  pub const ALL: Self = Self(31);
}

impl std::ops::BitOr for GCType {
  type Output = Self;

  fn bitor(self, Self(rhs): Self) -> Self {
    let Self(lhs) = self;
    Self(lhs | rhs)
  }
}

/// GCCallbackFlags is used to notify additional information about the GC
/// callback.
///   - GCCallbackFlagConstructRetainedObjectInfos: The GC callback is for
///     constructing retained object infos.
///   - GCCallbackFlagForced: The GC callback is for a forced GC for testing.
///   - GCCallbackFlagSynchronousPhantomCallbackProcessing: The GC callback
///     is called synchronously without getting posted to an idle task.
///   - GCCallbackFlagCollectAllAvailableGarbage: The GC callback is called
///     in a phase where V8 is trying to collect all available garbage
///     (e.g., handling a low memory notification).
///   - GCCallbackScheduleIdleGarbageCollection: The GC callback is called to
///     trigger an idle garbage collection.
#[repr(C)]
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct GCCallbackFlags(u32);

impl GCCallbackFlags {
  pub const NONE: Self = Self(0);

  pub const CONSTRUCT_RETAINED_OBJECT_INFOS: Self = Self(1 << 1);

  pub const FORCED: Self = Self(1 << 2);

  pub const SYNCHRONOUS_PHANTOM_CALLBACK_PROCESSING: Self = Self(1 << 3);

  pub const COLLECT_ALL_AVAILABLE_GARBAGE: Self = Self(1 << 4);

  pub const COLLECT_ALL_EXTERNAL_MEMORY: Self = Self(1 << 5);

  pub const SCHEDULE_IDLE_GARBAGE_COLLECTION: Self = Self(1 << 6);
}

impl std::ops::BitOr for GCCallbackFlags {
  type Output = Self;

  fn bitor(self, Self(rhs): Self) -> Self {
    let Self(lhs) = self;
    Self(lhs | rhs)
  }
}
