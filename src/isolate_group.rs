// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.

use crate::support::Opaque;

#[repr(C)]
struct InternalIsolateGroup(Opaque);

extern "C" {
  fn v8__IsolateGroup__GetDefault() -> *const InternalIsolateGroup;
  fn v8__IsolateGroup__CanCreateNewGroups() -> bool;
  fn v8__IsolateGroup__Create() -> *const InternalIsolateGroup;

  fn v8__IsolateGroup__DESTRUCT(this: *mut IsolateGroup);
  fn v8__IsolateGroup__EQ(
    this: *const IsolateGroup,
    other: *const IsolateGroup,
  ) -> bool;
}

/// The set of V8 isolates in a process is partitioned into groups. Each group
/// has its own sandbox (if V8 was configured with support for the sandbox) and
/// pointer-compression cage (if configured with pointer compression).
///
/// By default, all isolates are placed in the same group. This is the most
/// efficient configuration in terms of speed and memory use. However, with
/// pointer compression enabled, total heap usage of isolates in a group cannot
/// exceed 4 GB, not counting array buffers and other off-heap storage. Using
/// multiple isolate groups can allow embedders to allocate more than 4GB of
/// objects with pointer compression enabled, if the embedder's use case can
/// span multiple isolates.
///
/// Creating an isolate group reserves a range of virtual memory addresses. A
/// group's memory mapping will be released when the last isolate in the group
/// is disposed, and there are no more live IsolateGroup objects that refer to
/// it.
///
/// Note that Isolate groups are reference counted, and the IsolateGroup type is
/// a reference to one.
///
/// Note that it's not going to be possible to pass shared JS objects across
/// IsolateGroup boundary.
#[repr(C)]
pub struct IsolateGroup(*const InternalIsolateGroup);

unsafe impl Send for IsolateGroup {}
unsafe impl Sync for IsolateGroup {}

impl IsolateGroup {
  /// Return true if new isolate groups can be created at run-time, or false if
  /// all isolates must be in the same group.
  pub fn can_create_new_groups() -> bool {
    unsafe { v8__IsolateGroup__CanCreateNewGroups() }
  }

  /// Get the default isolate group. If this V8's build configuration only
  /// supports a single group, this is a reference to that single group.
  /// Otherwise this is a group like any other, distinguished only in that it is
  /// the first group.
  pub fn get_default() -> Self {
    IsolateGroup(unsafe { v8__IsolateGroup__GetDefault() })
  }

  /// Create a new isolate group. If this V8's build configuration only supports
  /// a single group, abort.
  pub fn create() -> Self {
    IsolateGroup(unsafe { v8__IsolateGroup__Create() })
  }
}

impl Default for IsolateGroup {
  fn default() -> Self {
    IsolateGroup::get_default()
  }
}

impl Drop for IsolateGroup {
  fn drop(&mut self) {
    unsafe { v8__IsolateGroup__DESTRUCT(self) }
  }
}

impl Eq for IsolateGroup {}

impl PartialEq for IsolateGroup {
  fn eq(&self, other: &Self) -> bool {
    unsafe { v8__IsolateGroup__EQ(self, other) }
  }
}
