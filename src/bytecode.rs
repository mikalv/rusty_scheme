use std::ptr;
use value;
use alloc;
use std::cell;

/// A bytecode object.  Consists of a header, the length of the bytecodes,
/// the actual bytecodes, and finally the constants vector (not actually part
/// of the BCO, but always allocated after it).
pub struct BCO {
    /// The standard header object
    header: usize,

    /// The length of the bytecodes
    bytecode_length: usize,

    /// Pointer to the constants vector
    constants_vector: cell::UnsafeCell<value::Value>,
}

pub fn get_constants_vector(bco: &BCO) -> &cell::UnsafeCell<value::Value> {
    &bco.constants_vector
}

/// The opcodes
#[repr(u8)]
#[derive(Copy, Clone, Debug)]
pub enum Opcode {
    /// Implements `cons`.  `src` is the stack index of the source,
    /// `src2` is the stack index of the destination.  `dst` must be 0, 1, or 2
    /// and refers to the number of words to pop off of the stack.
    /// Pushes the new pair onto the stack.
    Cons,

    /// Implements `car`.  `src` is the stack depth of the pair to take the `car`
    /// of.
    Car,

    /// `cdr`
    Cdr,

    /// `set-car!`
    SetCar,

    /// `set-cdr!`
    SetCdr,

    /// `pair?`
    IsPair,

    /// Addition
    Add,

    /// Subtraction
    Subtract,

    /// Multiplication
    Multiply,

    /// Division
    Divide,

    /// Exponentiation
    Power,

    /// Create an array
    MakeArray,

    /// Store to an array
    SetArray,

    /// Load from an array
    GetArray,

    /// Check for vector
    IsArray,

    /// Length of vector
    ArrayLen,

    /// Function call
    Call,

    /// Tail call
    TailCall,

    /// Return from a function
    Return,

    /// Create a closure
    Closure,

    /// Mutation of stack slots
    Set,

    /// Load from constant vector
    LoadConstant,

    /// Load from environment
    LoadEnvironment,

    /// Load from argument
    LoadArgument,

    /// Load from global
    LoadGlobal,

    /// Load `#f`
    LoadFalse,

    /// Load `#t`
    LoadTrue,

    /// Load the empty list
    LoadNil,

    /// Store to environment.  `src` is the stack index of the source.
    /// `dst` is the stack index of the destination.
    StoreEnvironment,

    /// Store to argument.  `src` is the index of the argument.
    StoreArgument,

    /// Store to global.  `src` is the index of the global in the constants
    /// vector.
    StoreGlobal,
}

#[derive(Copy, Clone, Debug)]
pub struct Bytecode {
    pub opcode: Opcode,
    pub src: u8,
    pub src2: u8,
    pub dst: u8,
}

pub enum BadByteCode {
    StackUnderflow {
        index: usize,
        depth: usize,
        min: usize,
    },
    EnvOutOfRange {
        index: usize,
        required_length: usize,
        actual_length: usize,
    },
}

pub fn allocate_bytecode(obj: &[u8], heap: &mut alloc::Heap) {
    use value::HeaderTag;
    let (val, _) = heap.alloc_raw((size_of!(BCO) + obj.len() + (size_of!(usize) - 1)) /
                                  size_of!(value::Value),
                                  HeaderTag::Bytecode);
    let bco_obj = val as *mut BCO;
    let consts_vector = heap.stack.pop().unwrap();
    heap.stack.push(value::Value::new(val as usize | value::RUST_DATA_TAG));
    unsafe {
        (*bco_obj).bytecode_length = obj.len();
        (*(*bco_obj).constants_vector.get()) = consts_vector;
        ptr::copy_nonoverlapping(obj.as_ptr(),
                                 (val as *mut u8).offset(size_of!(BCO) as isize),
                                 obj.len())
    }
}

pub enum SchemeResult {
    BadBytecode(BadByteCode),
}
#[cfg(none)]
pub fn verify_bytecodes(b: &[Bytecode],
                        argcount: u16,
                        is_vararg: bool,
                        environment_length: usize)
                        -> Result<(), BadByteCode> {
    let argcount: usize = argcount.into();
    let mut i = 0;
    let mut max_stack = 0;
    let mut current_depth: usize = 0;
    let mut current_stack: Vec<usize> = vec![];
    let iter = b.iter();

    macro_rules! check_stack {
        ($min: expr) => (if current_depth <= $exp {
            return Err(BadByteCode::StackUnderflow { index: i - 1,
                                                     depth: current_depth,
                                                     min: $min, })
        } else {})
    }
    macro_rules! check_argument {
        ($min: expr) => (if argcount <= $expr {
            return Err(BadByteCode::StackUnderflow { index: i - 1,
                                                     depth: current_depth,
                                                     min: $min, })
        } else {})
    }
    macro_rules! check_env {
        ($min: expr) => (if environment_length <= $expr {
            return Err(BadByteCode::EnvOutOfRange { index: i - 1,
                                                    required_length: $expr + 1,
                                                    actual_length:
                                                    environment_length,
            })
        })
    }
    while let Some(opcode) = iter.next() {
        i += 1;
        match try!(byte_to_opcode(opcode)) {
            Opcode::Cons => {
                check_stack!(2);
                current_depth -= 1;
            }
            Opcode::Car | Opcode::Cdr => {
                check_stack!(1);
            }
            Opcode::SetCar | Opcode::SetCdr => {
                check_stack!(2);
            }
            Opcode::IsPair => {
                check_stack!(1);
            }
            Opcode::PushTrue | Opcode::PushFalse | Opcode::PushNil => {
                current_depth += 1;
            }
            Opcode::LoadGlobal | Opcode::LoadConstant => {
                check_env!(src);
                current_depth += 1;
            }
            Opcode::LoadArgument => {
                check_argument!(src);
                current_depth += 1
            }
            Opcode::StoreArgument => {
                check_argument!(src);
                current_depth += 1
            }
            Opcode::LoadEnvironment => {
                check_stack!(src);
                current_depth += 1;
            }
            Opcode::ArraySet => {
                check_stack!(2);
                current_depth -= 1;
            }
            Opcode::ArrayGet => {
                check_stack!(2);
            }
            Opcode::IsArray => {
                check_stack!(1);
            }
            Opcode::Vector => try!(iter.next().ok_or(BadByteCode::EOF)).into(),
        }
    }
}
