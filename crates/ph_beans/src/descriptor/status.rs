//! Status markers and the ready-check.

/// The pipeline has been built.
pub struct Valid;

/// `init_state` has run and `.build()` has not.
pub struct NeedBuild;

/// The pipeline has not stored an init system.
pub struct NeedInit;

/// `init_state` ran twice.
pub struct ErrInitAlreadySet;

/// A warning status. The deprecated `.build()` path is a later slice.
pub struct WarnUnusedArgs;

/// The filter or emit event path is not well formed.
pub struct ErrBadEvent;

/// A live `Mut` is still held, so the next `.inject_resources` needs a wait.
pub struct ErrNeedWaitFilter;

/// `.map` was called more than [`crate::MAX_MAP_SYSTEMS`] times.
pub struct ErrTooManySystems;

/// Direct emit named an event that is not a child of the current one.
pub struct ErrNotChild;

/// `Status` may leave `setup`.
///
/// Blocking statuses do not implement this. The message names the status
/// and the pipeline tip.
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `{Self}` (tip `{T}`)",
    label = "fix Descriptor::Status",
    note = "one Status for the whole chain (build, events, accessors, DI)"
)]
// Generics: `T` tip.
pub trait PipelineOk<T> {}

impl<T> PipelineOk<T> for Valid {}

impl<T> PipelineOk<T> for WarnUnusedArgs {}
