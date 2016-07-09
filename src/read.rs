use std::io;
use std::io::prelude::*;
use super::{alloc, value};
#[cfg(none)]
pub fn skip_space<R: Read>(x: R) -> Option<u8> {
    for i in x.bytes() {
        if libc::isspace(i) {
            return Some(Ok(i))
        }
    }
    return None
}
use interp;

pub fn read<R: Read>(state: &mut interp::State, file: R, end_char: char) -> (R, value::Value) {
    enum State<'a> {
        InList,
        InVec,
        ReadEval,
        Recursive(usize),
    };
    let mut depth = 1;
    let ref mut heap = state.heap;
    let (mut local_root, mut current_handle) = (heap.new_handle(),
                                                heap.new_handle());
    let mut x = file.chars();
    let run_macros_self_evaluating = |mut value| while let Some(x) = state.pop()
    {
        match x {
            State::InList|State::InVec => {
                x.push(value);
                break
            }
            State::ReadEval => (),
            _ => unimplemented!(),
        }
    };
    'mainloop: while let Some(mut c) = x.next() {
        match c {
            '(' => read_state.push(State::InList),
            '\'' => {
                heap.push(false);
                heap.list(1)
                let new_pair = heap., state.empty_list);
                current_handle.set(local_pair);
                local_handle = heap.pair(state.symbol_table.quote, new_pair);
            }
            '#' => match x.next() {
                None => return Err("Missing character after #\\#"),
                Some(c) => {
                    match c {
                        '(' => state.push(ReadState::InVec),
                        '.' => state.push(ReadState::ReadEval),
                        '\\' => match x.next() {
                            None => return Err("Missing character after \"#\\\""),
                            Some(c) => c,
                        },
                        'f' => read_state.push(false),
                    }
                    continue 'mainloop
                }
            },
            '`' => unimplemented!(),
            ',' => unimplemented!(),
            ')' => { // End of token
                state.pop().map_or_else(|s| {
                    match s {
                        State::InList(handle) => {
                            handle.set(local_root)
                        }
                    }
                })
            }
            '"' => {
                let string = String::new();
                while let Some(new_char) = x.next() {
                    if new_char == '\\' {
                        string.push(try!(map_escape(try!(x.next.map_err()))))
                    }
                    else if new_char == '"' {
                        break
                    } else {
                        string.push(new_char)
                    }
                }
                run_macros_self_evaluating()
            }
        }
    }
}
