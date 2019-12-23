use std::mem::take;
use std::mem::MaybeUninit;

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
  Entered(S),
}

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
  pub fn enter(&'s mut self) -> &'s mut S {
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
      Uninit(b) => Entered(unsafe { b.assume_init() }),
      _ => unreachable!(),
    };

    match state {
      Entered(v) => v,
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
