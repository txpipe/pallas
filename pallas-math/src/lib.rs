pub mod math;

// Ensure only one of `gmp` or `num` is enabled, not both.
#[cfg(all(feature = "gmp", feature = "num"))]
compile_error!("Features `gmp` and `num` are mutually exclusive.");

#[cfg(all(not(feature = "gmp"), not(feature = "num")))]
compile_error!("One of the features `gmp` or `num` must be enabled.");

#[cfg(feature = "gmp")]
pub mod math_gmp;

#[cfg(feature = "num")]
pub mod math_num;
