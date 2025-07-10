//! Rust wrapper for Apple Matrix Coprocessor (AMX) instructions
//!
//! This crate provides wrapper functions for the undocumented AMX instructions,
//! which are found in Apple Silicon processors.
//!
//! # Resources
//!
//!  - <https://gist.github.com/dougallj/7a75a3be1ec69ca550e7c36dc75e0d6f>
//!  - <https://www.realworldtech.com/forum/?threadid=187087&curpostid=187120>
//!
//! # Example
//!
//! ```rust
//! use amx::{Amx, XRow, YRow, XBytes, YBytes, ZRow};
//! let mut ctx = amx::AmxCtx::new().unwrap();
//! let x = [1,  2,  3,  4,  5,  6,  7,  8,  9,  10, 11, 12, 13, 14, 15, 16,
//!          17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32i16];
//! let y = [51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66,
//!          67, 68, 69, 70, 71, 72, 73, 74, 75, 76, 77, 78, 79, 80, 81, 82i16];
//! unsafe { ctx.load512(x.as_ptr(), XRow(0)) };
//! unsafe { ctx.load512(y.as_ptr(), YRow(0)) };
//! ctx.outer_product_i16_xy_to_z(
//!     Some(XBytes(0)),    // input from X starting from byte offset 0
//!     Some(YBytes(0)),    // input from Y starting from byte offset 0
//!     ZRow(0),            // output to Z starting from row offset 0
//!     false,              // don't accumulate
//! );
//! let z: [[i16; 32]; 64] = unsafe { std::mem::transmute(ctx.read_z()) };
//! for (x_i, &x) in x.iter().enumerate() {
//!     for (y_i, &y) in y.iter().enumerate() {
//!         assert_eq!(z[y_i * 2][x_i], x * y);
//!     }
//! }
//! ```
//!
//! # Registers
//!
//! ```rust
//! struct AmxState {
//!     /// "8 64-byte registers"
//!     x: [[u8; 64]; 8],
//!     /// "8 64-byte registers"
//!     y: [[u8; 64]; 8],
//!     /// "64 64-byte registers in an M-by-N matrix"
//!     z: [[u8; 64]; 64],
//! }
//! ```

mod emu;
mod genlut;
mod load_store;
mod ops;
mod regs;
pub use crate::{emu::*, genlut::*, load_store::*, ops::AmxOps, regs::*};

cfg_if::cfg_if! {
    if #[cfg(any(doc, target_arch = "aarch64"))] {
        mod nativectx;
        pub mod nativeops;
        pub use crate::nativectx::{AmxCtx, NewAmxCtxError};
    }
}

/// The prelude.
pub mod prelude {
    #[doc(no_inline)]
    pub use crate::{ops::AmxOps as _, Amx as _};
}

/// A high-level wrapper for AMX instructions.
///
/// An application can use this wrapper in one of the following ways:
///
///  - On a supported system, construct an [`AmxCtx`] by calling
///    [`AmxCtx::new`]. `AmxCtx` derefs to [`amx::nativeops::AmxOps`], which
///    implements [`AmxOps`]​ (the low-level wrapper), which has a
///    blanket impl of `Amx`.
///
///  - On a supported system, construct [`amx::nativeops::AmxOps`] directly by
///    calling [`amx::nativeops::AmxOps::new`]. This is unsafe because it does
///    not enable the current thread's AMX context automatically or check if AMX
///    is really available.
///
///  - On any system, construct [`AmxEmuCtx`] by calling [`AmxEmuCtx::default`].
///    It implements [`AmxOps`], which has a blanket impl of `Amx`.
///
/// [`amx::nativeops::AmxOps`]: crate::nativeops::AmxOps
/// [`amx::nativeops::AmxOps::new`]: crate::nativeops::AmxOps::new
pub trait Amx: crate::ops::AmxOps {
    /// Load 512 bits (64 bytes) from memory to the specified register row.
    #[inline(always)]
    #[track_caller]
    unsafe fn load512<T>(&mut self, ptr: *const T, row: impl LoadStore) {
        row.load512(self, ptr);
    }

    /// Load 1024 bits (128 bytes) from memory to the specified register row
    /// and the subsequent one. The specified pointer must be aligned to
    /// 128-byte boundaries.
    #[inline(always)]
    #[track_caller]
    unsafe fn load1024_aligned<T>(&mut self, ptr: *const T, row: impl LoadStore) {
        row.load1024_aligned(self, ptr);
    }

    /// Store 512 bits (64 bytes) the specified register row's contents to
    /// memory.
    #[inline(always)]
    #[track_caller]
    unsafe fn store512<T>(&mut self, ptr: *mut T, row: impl LoadStore) {
        row.store512(self, ptr);
    }

    /// Store 1024 bits (128 bytes) the specified register row and the
    /// subsequent one's contents to memory.  The specified pointer must be
    /// aligned to  128-byte boundaries.
    #[inline(always)]
    #[track_caller]
    unsafe fn store1024_aligned<T>(&mut self, ptr: *mut T, row: impl LoadStore) {
        row.store1024_aligned(self, ptr);
    }

    /// Load 512 bits (64 bytes) from memory to `z[index][0..64]` with interleaving.
    ///
    /// `index` must be in range `0..64`.
    #[inline(always)]
    #[track_caller]
    unsafe fn load512_interleaved<T>(&mut self, ptr: *const T, row: ZRow) {
        load512_z_interleaved(self, ptr, row);
    }

    /// Store 512 bits (64 bytes) `z[index][0..64]` to memory with interleaving.
    ///
    /// `index` must be in range `0..64`.
    #[inline(always)]
    #[track_caller]
    unsafe fn store512_interleaved<T>(&mut self, ptr: *mut T, row: ZRow) {
        store512_z_interleaved(self, ptr, row);
    }

    /// Read the whole contents of `x`.
    fn read_x(&mut self) -> [u8; 512] {
        let mut ret = std::mem::MaybeUninit::uninit();
        for i in 0..8 {
            // Safety: Writing in a memory region within `ret`
            unsafe {
                self.store512(
                    (ret.as_mut_ptr() as *mut u8).offset(i as isize * 64),
                    XRow(i),
                )
            };
        }
        // Safety: All elements are initialized
        unsafe { ret.assume_init() }
    }

    /// Read the whole contents of `y`.
    fn read_y(&mut self) -> [u8; 512] {
        let mut ret = std::mem::MaybeUninit::uninit();
        for i in 0..8 {
            // Safety: Writing in a memory region within `ret`
            unsafe {
                self.store512(
                    (ret.as_mut_ptr() as *mut u8).offset(i as isize * 64),
                    YRow(i),
                )
            };
        }
        // Safety: All elements are initialized
        unsafe { ret.assume_init() }
    }

    /// Read the whole contents of `z`.
    fn read_z(&mut self) -> [u8; 4096] {
        let mut ret = std::mem::MaybeUninit::uninit();
        for i in 0..64 {
            // Safety: Writing in a memory region within `ret`
            unsafe {
                self.store512(
                    (ret.as_mut_ptr() as *mut u8).offset(i as isize * 64),
                    ZRow(i),
                )
            };
        }
        // Safety: All elements are initialized
        unsafe { ret.assume_init() }
    }

    /// Calculate the outer product of `x: [i16; 32]` and `y: [i16; 32]` and write
    /// the output to every second row of `z: [[i16; 32]; 64]`.
    ///
    /// If `x_offset_bytes` and/or `y_offset_bytes` are `None`, the respective
    /// registers will be excluded from the operation (not performing
    /// multiplication).
    ///
    /// `z_index` must be in range `0..64`. Only the least significant bit of
    /// `z_index` will be taken into consideration.
    #[inline(always)]
    fn outer_product_i16_xy_to_z(
        &mut self,
        x_offset_bytes: Option<XBytes>,
        y_offset_bytes: Option<YBytes>,
        z_index: ZRow,
        accumulate: bool,
    ) {
        // FIXME: rustfmt doesn't like patterns in provided trait methods
        let z_index = z_index.0;
        debug_assert!(x_offset_bytes.unwrap_or_default().0 < 0x200);
        debug_assert!(y_offset_bytes.unwrap_or_default().0 < 0x200);
        debug_assert!(z_index < 64);
        // TODO: widening (i32 output)
        // TODO: vector output (reducing)
        self.mac16(
            (y_offset_bytes.unwrap_or_default().0
                | (x_offset_bytes.unwrap_or_default().0 << 10)
                | (z_index << 20)
                | (((!accumulate) as usize) << 27)
                | ((x_offset_bytes.is_none() as usize) << 28)
                | ((y_offset_bytes.is_none() as usize) << 29)) as u64,
        );
    }

    /// Perform (reverse) table lookup.
    #[inline(always)]
    fn lut(&mut self, input: impl LutIn, table: XRow, output: impl LutOut, ty: impl LutTy) {
        genlut::lut(self, input, table, output, ty);
    }
}

impl<T: AmxOps + ?Sized> Amx for T {}
