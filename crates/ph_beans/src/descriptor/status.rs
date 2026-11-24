//! Status markers and the ready-check.

/// The pipeline has been built.
pub struct Valid;

/// `init_state` has run and `.build()` has not.
pub struct NeedBuild;

/// The pipeline has not stored an init system.
pub struct NeedInit;

/// A warning status. The deprecated `.build()` path is a later slice.
pub struct WarnUnusedArgs;

/// `Status` may leave `setup`.
///
/// Blocking statuses do not implement this. The message names the status
/// and the pipeline tip.
#[diagnostic::on_unimplemented(
    message = "pipeline not ready: Descriptor::Status = `{Self}` (tip `{T}`)",
    label = "fix Descriptor::Status",
    note = "one Status for the whole chain (build, events, accessors, DI); \
            error codes: docs/errors.md"
)]
// Generics: `T` tip.
pub trait PipelineOk<T> {}

impl<T> PipelineOk<T> for Valid {}

impl<T> PipelineOk<T> for WarnUnusedArgs {}
