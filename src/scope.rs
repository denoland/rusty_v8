use std::marker::PhantomData;
use std::mem::size_of;
use std::mem::take;
use std::mem::MaybeUninit;
use std::ops::Deref;
use std::ops::DerefMut;

// Note: the 's lifetime is there to ensure that after entering a scope once,
// the same scope object can't ever be entered again.

/// A trait for defining scoped objects.
pub unsafe trait Scoped<'s>
where
  Self: Sized,
{
  type Args;
  fn enter_scope(buf: &mut MaybeUninit<Self>, args: Self::Args) -> ();
}

/// A RAII scope wrapper object that will, when the `enter()` method is called,
/// initialize and activate the guarded object.
pub struct Scope<'s, S>(ScopeState<'s, S>)
where
  S: Scoped<'s>;

enum ScopeState<'s, S>
where
  S: Scoped<'s>,
{
  Empty,
  New(S::Args),
  Uninit(MaybeUninit<S>),
  Ready(Entered<'s, S>),
}

/// A wrapper around the an instantiated and entered scope object.
#[repr(transparent)]
pub struct Entered<'s, S>(PhantomData<&'s ()>, S);

impl<'s, S> Scope<'s, S>
where
  S: Scoped<'s>,
{
  /// Create a new Scope object in unentered state.
  pub(crate) fn new(args: S::Args) -> Self {
    Self(ScopeState::New(args))
  }

  /// Initializes the guarded object and returns a mutable reference to it.
  /// A scope can only be entered once.
  pub fn enter(&'s mut self) -> &'s mut Entered<S> {
    assert_eq!(size_of::<S>(), size_of::<MaybeUninit<S>>());
    assert_eq!(size_of::<S>(), size_of::<Entered<S>>());

    use ScopeState::*;
    let state = &mut self.0;

    let args = match take(state) {
      New(f) => f,
      _ => unreachable!(),
    };

    *state = Uninit(MaybeUninit::uninit());
    let buf = match state {
      Uninit(b) => b,
      _ => unreachable!(),
    };

    S::enter_scope(buf, args);

    *state = match take(state) {
      Uninit(b) => Ready(unsafe { b.assume_init() }.into()),
      _ => unreachable!(),
    };

    match state {
      Ready(v) => &mut *v,
      _ => unreachable!(),
    }
  }
}

impl<'s, S> Default for ScopeState<'s, S>
where
  S: Scoped<'s>,
{
  fn default() -> Self {
    Self::Empty
  }
}

impl<'s, S> From<S> for Entered<'s, S> {
  fn from(value: S) -> Self {
    Self(PhantomData, value)
  }
}

impl<'s, S> Deref for Entered<'s, S> {
  type Target = S;
  fn deref(&self) -> &S {
    unsafe { &*(self as *const _ as *const S) }
  }
}

impl<'s, S> DerefMut for Entered<'s, S> {
  fn deref_mut(&mut self) -> &mut S {
    unsafe { &mut *(self as *mut _ as *mut S) }
  }
}
