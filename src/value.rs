//! The representation of Scheme values in RustyScheme.
//!
//! Inspired by the representation of Femtolisp.
//!
//! ### Representation
//!
//! | Type      | Representation |
//! |-----------|----------------|
//! |Fixnum     | As an immediate pointer, with tag 0 or 4.|
//! |Flonums    | As a pointer to a (boxed) floating-point number, with tag 1.|
//! |Pairs| As a pointer to a 2-tuple, with pointer tag 3. |
//! |Arrays| As an untagged, aligned pointer to a Rust slice. |
//! |Records| As a pointer to a Rust slice, with a special header for the GC that indicates how it should be marked.|
//! |Resources  | As a pointer into a 3-tuple, consisting of a GC header, a pointer to a `struct` that contains an object ID and custom equality, hashing, and other functions, and a pointer into memory not managed by the GC. |

use std::cell::Cell;
use std::mem;

// Same set used by Femtolisp
/// The tag of `fixnum`s
pub const NUM_TAG: usize = 0b000;

/// The tag of Rust-implemented functions.
pub const RUST_FUNC_TAG: usize = 0b001;

/// The tag of Scheme-implemented functions.
pub const FUNCTION_TAG: usize = 0b010;

/// The tag of Scheme vectors, records, and closures.
pub const VECTOR_TAG: usize = 0b011;

/// The tag of non-`fixnum` immediates, such as the empty list,
/// end-of-file object, the undefined value, and characters.
pub const NUM_TAG_2: usize = 0b100;

/// The tag of RustData – Rust values stored on the Scheme heap.
pub const RUST_DATA_TAG: usize = 0b101;

/// The tag of Symbols.
pub const SYMBOL_TAG: usize = 0b110;

/// The tag of Pairs
pub const PAIR_TAG: usize = 0b111;

//#[cfg(target_pointer_width = "16")]
//pub const SIZEOF_PTR: usize = 2;

#[cfg(target_pointer_width = "32")]
pub const SIZEOF_PTR: usize = 4;

#[cfg(target_pointer_width = "64")]
pub const SIZEOF_PTR: usize = 8;

/// The amount of memory occupied by a pair.
pub const SIZEOF_PAIR: usize = (3*self::SIZEOF_PTR + 0b111) >> 3;

/// Bitmask that includes the tag words of an object header.
pub const HEADER_TAG: usize = 0b111 << (self::SIZEOF_PTR*8 - 3);

/// The header of a pair.
pub const PAIR_HEADER: usize = HEADER_TAG | SIZEOF_PAIR;

/// The header of a vector.
pub const VECTOR_HEADER: usize = 0;

#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum Tags {
    Num,
    RustFunc,
    Function,
    Vector,
    Num2,
    RustData,
    Symbol,
    Pair,
}


impl Value {
    #[inline(always)]
    pub fn raw_tag(&self) -> usize {
        self.get() & 0b111
    }

    #[inline(always)]
    pub fn tag(&self) -> Tags {
        use self::Tags::*;
        match self.raw_tag() {
            NUM_TAG => Num,
            RUST_FUNC_TAG => RustFunc,
            FUNCTION_TAG => Function,
            VECTOR_TAG => Vector,
            NUM_TAG_2 => Num2,
            RUST_DATA_TAG => RustData,
            SYMBOL_TAG => Symbol,
            PAIR_TAG => Pair,
            _ => unsafe {
                enum Void {}
                match mem::transmute::<(),Void>(()) {}
            },
        }
    }
    #[inline(always)]
    pub fn leafp(&self) -> bool {
        self.raw_tag() & 0b11 != 0b11
    }
    #[inline(always)]
    pub fn both_fixnums(&self, other: &Self) -> bool {
        (self.get() | other.get()) & 0b11 == 0
    }
    #[inline(always)]
    pub fn self_evaluating(&self) -> bool {
        self.raw_tag() < 6
    }
    #[inline(always)]
    pub fn fixnump(&self) -> bool {
        self.raw_tag() & 0b11 == 0
    }
    #[inline(always)]
    pub fn pairp(&self) -> bool {
        self.tag() == Tags::Pair
    }
    #[inline(always)]
    pub fn flonump(&self) -> bool {
        unimplemented!()
    }
}

macro_rules! Ptr_Val {
    ($expr:expr) => {
        ($expr.contents.get() & !0b111) as *mut Value
    }
}

macro_rules! size_of {
    ($ty:ty) => {
        ::std::mem::size_of::<$ty>()
    }
}

/// A Scheme value.
///
/// Scheme values are garbage collected, so must never appear outside
/// the heap, stack, or handles.  The GC will invalidate any other `Value`,
/// creating a dangling pointer.
#[repr(C)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Value {
    pub contents: Cell<usize>,
}

/// A Scheme "vector-like thing".
///
/// Vector-like things are Scheme values with tag `Tags::Vector`.
/// They all consist of a header followed by a slice of Scheme values.
/// The number of Scheme words is always computable by
///
/// ```rust
/// /*
/// use std::marker::PhantomData;
/// use value;
/// let vector = value::Value { phantom: PhantomData, contents: 0 };
/// if vector.tag() != value::Tags::Vector {
///     None // not a vector-like thing
/// } else {
///     Some(unsafe {
///         *((vector.contents & !0b111) as *mut usize) &
///         (!0b111 << (::std::mem::size_of::<usize>()*8 - 3))
///     })
/// };
/// */
/// ```
///
/// which is exposed as the method `len`.
///
/// Vector-like things have their own tags, in the 3 most significant bits
/// of the header word.  They have the following meanings:
///
/// | Tag |Meaning|
/// |-----|-------|
/// |0b000|Vector (chosen to simplify bounds checks)|
/// |0b001|Record.  The first word points to a record descriptor
/// used to identify the record type.|
/// |0b010|Closure.  The first word is a reference to a function object
/// that stores the function of the closure.|
/// |Others|Reserved.  These may be later used by the run-time system.
///
/// This struct **cannot** be moved, because it is followed by Scheme
/// objects that are not a part of the object.  As such, it has no public
/// constructors, and can only be instantiated by reference.
#[repr(C)]
#[derive(Debug)]
pub struct Vector {
    /// Header.  Always has `0b000` as the 3 MSBs.
    header: usize,
}

/// A descriptor for a `Record`.
pub struct RecordDescriptor {
    /// Always a multiple of 8, but never zero.
    id: usize,
}

#[repr(C)]
#[derive(Debug)]
pub struct Record {
    /// Header.  Always starts with a nonzero 3 most significant bits.
    header: usize,

    /// Scheme values.
    data: [Value],
}

/// A (mutable) Scheme pair.  Subject to garbage collection.
#[repr(C)]
#[derive(Debug)]
pub struct Pair {
    pub header: usize,
    pub car: Value,
    pub cdr: Value,
}

/// A Scheme closure.  Subject to garbage collection.
#[repr(C)]
#[derive(Debug)]
pub struct Closure {
    header: usize,
    pub bytecode: Value, // a BCO
    pub environment: [Value],
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Instruction {
    pub opcode: u8,
    pub src: u8,
    pub src2: u8,
    pub dst: u8,
}

pub enum Kind {
    Pair(*mut Pair),
    Vector(*mut Vector),
    Fixnum(usize),
}

/// An object containing compiled Scheme bytecode.  Subject to garbage collection.
#[repr(C)]
#[derive(Debug)]
pub struct BCO {
    /// Header.  Indicates that this is a BCO.
    header: usize,

    /// Actual bytecode
    pub contents: [Instruction],
}

impl Value {
    #[inline(never)]
    pub fn slow_add(_first: &Self, _second: &Self) -> Result<Self, SchemeError> {
        unimplemented!()
    }

    #[inline(always)]
    pub fn subtract(_first: &Self, _second: &Self) -> Self {
        unimplemented!()
    }

    pub fn size(&self) -> Option<usize> {
        if self.fixnump() {
            None
        } else {
            Some(unsafe {
                *((self.contents.get() & !0b111) as *const usize) & !HEADER_TAG
            })
        }
    }
    pub fn set_car(&self, other: Value) -> Result<(), ()> {
        match self.kind() {
            Kind::Pair(pair) => unsafe { Ok((*pair).car.set(other)) },
            _ => Err(()),
        }
    }
    pub fn set_cdr(&self, other: Value) -> Result<(), ()> {
        match self.kind() {
            Kind::Pair(pair) => unsafe { Ok((*pair).cdr.set(other)) },
            _ => Err(()),
        }
    }
    pub fn car(&self) -> Result<Self, ()> {
        match self.kind() {
            Kind::Pair(pair) => unsafe { Ok((*pair).car.clone()) },
            _ => Err(())
        }
    }
    pub fn cdr(&self) -> Result<Self, ()> {
        match self.kind() {
            Kind::Pair(pair) => unsafe { Ok((*pair).cdr.clone()) },
            _ => Err(())
        }
    }
    pub fn new(contents: usize) -> Self {
        Value { contents: Cell::new(contents) }
    }
    pub fn set(&self, other: Self) -> () {
        self.contents.set(other.contents.get())
    }
    pub fn get(&self) -> usize {
        self.contents.get()
    }
    pub fn array_set(&self, index: usize, other: Value) -> Result<(), String> {
        let vec = match self.kind() {
            Kind::Vector(vec) => vec,
            _ => return Err("can't index a non-vector".to_owned()),
        };
        unsafe {
            if (*vec).header >= index {
                Err((if (*vec).header & HEADER_TAG == 0 {
                    "index out of bounds"
                } else {
                    "can't index a non-record"
                }).to_owned())
            } else {
                (*((vec as usize + index) as *const Value)).set(other);
                Ok(())
            }
        }
    }
    pub fn array_get(&self, index: usize) -> Result<*const Self, String> {
        let vec = match self.kind() {
            Kind::Vector(vec) => vec,
            _ => return Err("can't index a non-vector".to_owned()),
        };
        unsafe {
            if (*vec).header >= index {
                Err((if (*vec).header & HEADER_TAG == 0 {
                    "index out of bounds"
                } else {
                    "can't index a non-record"
                }).to_owned())
            } else {
                Ok((vec as usize + index) as *const Value)
            }
        }
    }

    pub fn kind(&self) -> Kind {
        match self.tag() {
            Tags::Pair => Kind::Pair(Ptr_Val!(self) as *mut Pair),
            Tags::Vector => Kind::Vector(Ptr_Val!(self) as *mut Vector),
            Tags::Num|Tags::Num2 => Kind::Fixnum(self.contents.get() >> 2),
            _ => unimplemented!(),
        }
    }
    pub fn as_fixnum(&self) -> Result<usize, &'static str> {
        match self.kind() {
            Kind::Fixnum(val) => Ok(val),
            _ => Err("not a fixnum"),
        }
    }
}

#[repr(C)]
pub struct Function {
    header: usize,
    bytecode: Value, // points to a byte code object
    constants: Value, // points to a a vector of constants
}

pub struct SchemeError(String);
pub struct Bignum;
impl Bignum {
    pub fn new_from_fixnums(_x: usize, _y: usize) -> ! {
        unimplemented!()
    }
}

pub unsafe fn float_val(val: &Value) -> f64 {
    *((val.get() & 0b111) as *const f64)
}

pub struct HashTable;
pub struct IOPort;
pub struct RustData;
