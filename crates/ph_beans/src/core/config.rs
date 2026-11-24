//! Config traits.

/// Names one type. Use [`OfConfig`] at the use site.
pub trait Config {
    /// The type this config stands for.
    type Type;
}

/// The type of the config. #Allowed typealias
pub type OfConfig<C> = <C as Config>::Type;
