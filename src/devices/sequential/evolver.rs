#![allow(dead_code)]

use crate::sysex::{ PatternExp, ExpType};
use ExpType::*;

const SEQUENTIAL: u8 = 0x01;
const EVOLVER: u8 = 0x20;
const PROGRAM_PARAM: &'static [u8] = &[SEQUENTIAL, EVOLVER, 0x01, 0x01];

// pub fn program_parameter_matcher() -> SysexMatcher {
//     let mut tokens: Vec<_, 1> = Vec::new();
//     if pattern_match(buffer, &[Seq(DATA_HEADER), Cap(ValueU7)], &mut tokens) {
//         tokens.get(0).map(|(idx, _)| buffer[idx] == expected).or_else(false)
//     }
//     false
//     SysexMatcher::new(vec![Seq(PROGRAM_PARAM), Cap(ParamId), Cap(LsbValueU4), Cap(MsbValueU4)])
// }
