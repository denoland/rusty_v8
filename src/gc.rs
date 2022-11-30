/// Applications can register callback functions which will be called before and
/// after certain garbage collection operations.  Allocations are not allowed in
/// the callback functions, you therefore cannot manipulate objects (set or
/// delete properties for example) since it is possible such operations will
/// result in the allocation of objects.
#[repr(C)]
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct GCType(u32);

pub const GC_TYPE_TYPE_SCAVENGE: GCType = GCType(1);

pub const GC_TYPE_MINOR_MARK_COMPACT: GCType = GCType(2);

pub const GC_TYPE_MARK_SWEEP_COMPACT: GCType = GCType(4);

pub const GC_TYPE_INCREMENTAL_MARKING: GCType = GCType(8);

pub const GC_TYPE_PROCESS_WEAK_CALLBACK: GCType = GCType(16);

pub const GC_TYPE_ALL: GCType = GCType(31);

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

pub const GC_CALLBACK_FLAGS_NO_FLAGS: GCCallbackFlags = GCCallbackFlags(0);

pub const GC_CALLBACK_FLAGS_CONSTRUCT_RETAINED_OBJECT_INFOS: GCCallbackFlags =
  GCCallbackFlags(2);

pub const GC_CALLBACK_FLAGS_FORCED: GCCallbackFlags = GCCallbackFlags(4);

pub const GC_CALLBACK_FLAGS_SYNCHRONOUS_PHANTOM_CALLBACK_PROCESSING:
  GCCallbackFlags = GCCallbackFlags(8);

pub const GC_CALLBACK_FLAGS_COLLECT_ALL_AVAILABLE_GARBAGE: GCCallbackFlags =
  GCCallbackFlags(16);

pub const GC_CALLBACK_FLAGS_COLLECT_ALL_EXTERNAL_MEMORY: GCCallbackFlags =
  GCCallbackFlags(32);

pub const GC_CALLBACK_FLAGS_SCHEDULE_IDLE_GARBAGE_COLLECTION: GCCallbackFlags =
  GCCallbackFlags(64);

impl std::ops::BitOr for GCCallbackFlags {
  type Output = Self;

  fn bitor(self, Self(rhs): Self) -> Self {
    let Self(lhs) = self;
    Self(lhs | rhs)
  }
}
