//! Prelude module for no_std compatibility.
//!
//! This module re-exports types from core/alloc/std based on feature flags,
//! allowing the rest of the codebase to use a consistent import path.

// ═══════════════════════════════════════════════════════════════════════════════
// Core types (always available)
// ═══════════════════════════════════════════════════════════════════════════════

pub use core::{
    cell::{Cell, Ref, RefCell, RefMut},
    fmt,
    hash::{Hash, Hasher},
    iter::Peekable,
    mem,
    ptr::NonNull,
    str::CharIndices,
};

// ═══════════════════════════════════════════════════════════════════════════════
// Alloc types (conditional on std vs no_std)
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "std")]
pub use std::{
    boxed::Box,
    collections::VecDeque,
    format,
    rc::{Rc, Weak},
    string::{String, ToString},
    vec,
    vec::Vec,
};

#[cfg(not(feature = "std"))]
pub use alloc::{
    boxed::Box,
    collections::VecDeque,
    format,
    rc::{Rc, Weak},
    string::{String, ToString},
    vec,
    vec::Vec,
};

// ═══════════════════════════════════════════════════════════════════════════════
// HashMap - use hashbrown for no_std, std::collections for std
// ═══════════════════════════════════════════════════════════════════════════════

#[cfg(feature = "std")]
pub use std::collections::HashSet;

#[cfg(not(feature = "std"))]
pub use hashbrown::HashSet;

// FxHashMap/FxHashSet - use rustc-hash for std, define our own for no_std
#[cfg(feature = "std")]
pub use rustc_hash::{FxHashMap, FxHashSet};

#[cfg(not(feature = "std"))]
pub type FxHashMap<K, V> =
    hashbrown::HashMap<K, V, core::hash::BuildHasherDefault<rustc_hash::FxHasher>>;

#[cfg(not(feature = "std"))]
pub type FxHashSet<T> =
    hashbrown::HashSet<T, core::hash::BuildHasherDefault<rustc_hash::FxHasher>>;

// ═══════════════════════════════════════════════════════════════════════════════
// IndexMap/IndexSet - use FxHasher for both std and no_std
// ═══════════════════════════════════════════════════════════════════════════════

pub type IndexMap<K, V> =
    indexmap::IndexMap<K, V, core::hash::BuildHasherDefault<rustc_hash::FxHasher>>;

pub type IndexSet<T> =
    indexmap::IndexSet<T, core::hash::BuildHasherDefault<rustc_hash::FxHasher>>;

/// Create an empty IndexMap
#[inline]
pub fn index_map_new<K, V>() -> IndexMap<K, V>
where
    K: core::hash::Hash + Eq,
{
    indexmap::IndexMap::with_hasher(Default::default())
}

/// Create an IndexMap with the given capacity
#[inline]
pub fn index_map_with_capacity<K, V>(capacity: usize) -> IndexMap<K, V>
where
    K: core::hash::Hash + Eq,
{
    indexmap::IndexMap::with_capacity_and_hasher(capacity, Default::default())
}

/// Create an empty IndexSet
#[inline]
pub fn index_set_new<T>() -> IndexSet<T>
where
    T: core::hash::Hash + Eq,
{
    indexmap::IndexSet::with_hasher(Default::default())
}

/// Create an IndexSet with the given capacity
#[inline]
pub fn index_set_with_capacity<T>(capacity: usize) -> IndexSet<T>
where
    T: core::hash::Hash + Eq,
{
    indexmap::IndexSet::with_capacity_and_hasher(capacity, Default::default())
}

// ═══════════════════════════════════════════════════════════════════════════════
// Math functions - use std in std mode, libm in no_std mode
// ═══════════════════════════════════════════════════════════════════════════════

/// Math operations module providing cross-platform math functions
pub mod math {
    #[cfg(feature = "std")]
    #[inline]
    pub fn floor(x: f64) -> f64 {
        x.floor()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn floor(x: f64) -> f64 {
        libm::floor(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn ceil(x: f64) -> f64 {
        x.ceil()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn ceil(x: f64) -> f64 {
        libm::ceil(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn round(x: f64) -> f64 {
        x.round()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn round(x: f64) -> f64 {
        libm::round(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn trunc(x: f64) -> f64 {
        x.trunc()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn trunc(x: f64) -> f64 {
        libm::trunc(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn fract(x: f64) -> f64 {
        x.fract()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn fract(x: f64) -> f64 {
        x - libm::trunc(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn powf(base: f64, exp: f64) -> f64 {
        base.powf(exp)
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn powf(base: f64, exp: f64) -> f64 {
        libm::pow(base, exp)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn sqrt(x: f64) -> f64 {
        x.sqrt()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn sqrt(x: f64) -> f64 {
        libm::sqrt(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn ln(x: f64) -> f64 {
        x.ln()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn ln(x: f64) -> f64 {
        libm::log(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn log10(x: f64) -> f64 {
        x.log10()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn log10(x: f64) -> f64 {
        libm::log10(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn log2(x: f64) -> f64 {
        x.log2()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn log2(x: f64) -> f64 {
        libm::log2(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn exp(x: f64) -> f64 {
        x.exp()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn exp(x: f64) -> f64 {
        libm::exp(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn expm1(x: f64) -> f64 {
        x.exp_m1()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn expm1(x: f64) -> f64 {
        libm::expm1(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn sin(x: f64) -> f64 {
        x.sin()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn sin(x: f64) -> f64 {
        libm::sin(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn cos(x: f64) -> f64 {
        x.cos()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn cos(x: f64) -> f64 {
        libm::cos(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn tan(x: f64) -> f64 {
        x.tan()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn tan(x: f64) -> f64 {
        libm::tan(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn asin(x: f64) -> f64 {
        x.asin()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn asin(x: f64) -> f64 {
        libm::asin(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn acos(x: f64) -> f64 {
        x.acos()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn acos(x: f64) -> f64 {
        libm::acos(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn atan(x: f64) -> f64 {
        x.atan()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn atan(x: f64) -> f64 {
        libm::atan(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn atan2(y: f64, x: f64) -> f64 {
        y.atan2(x)
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn atan2(y: f64, x: f64) -> f64 {
        libm::atan2(y, x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn sinh(x: f64) -> f64 {
        x.sinh()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn sinh(x: f64) -> f64 {
        libm::sinh(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn cosh(x: f64) -> f64 {
        x.cosh()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn cosh(x: f64) -> f64 {
        libm::cosh(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn tanh(x: f64) -> f64 {
        x.tanh()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn tanh(x: f64) -> f64 {
        libm::tanh(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn asinh(x: f64) -> f64 {
        x.asinh()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn asinh(x: f64) -> f64 {
        libm::asinh(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn acosh(x: f64) -> f64 {
        x.acosh()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn acosh(x: f64) -> f64 {
        libm::acosh(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn atanh(x: f64) -> f64 {
        x.atanh()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn atanh(x: f64) -> f64 {
        libm::atanh(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn cbrt(x: f64) -> f64 {
        x.cbrt()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn cbrt(x: f64) -> f64 {
        libm::cbrt(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn log1p(x: f64) -> f64 {
        x.ln_1p()
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn log1p(x: f64) -> f64 {
        libm::log1p(x)
    }

    #[cfg(feature = "std")]
    #[inline]
    pub fn powi(base: f64, exp: i32) -> f64 {
        base.powi(exp)
    }

    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn powi(base: f64, exp: i32) -> f64 {
        libm::pow(base, exp as f64)
    }

    /// Euclidean remainder (modulo) - always returns positive result
    #[cfg(feature = "std")]
    #[inline]
    pub fn rem_euclid(x: f64, y: f64) -> f64 {
        x.rem_euclid(y)
    }

    /// Euclidean remainder (modulo) - always returns positive result
    #[cfg(not(feature = "std"))]
    #[inline]
    pub fn rem_euclid(x: f64, y: f64) -> f64 {
        let r = libm::fmod(x, y);
        if r < 0.0 { r + y.abs() } else { r }
    }
}
