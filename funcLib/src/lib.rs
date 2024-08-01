#![allow(non_snake_case)]
#![allow(non_camel_case_types)]

use std::{ffi::CString, io};



#[no_mangle]
pub extern fn putinteger(val: i32) -> bool {
    println!("{}", val);
    return true;
}


#[no_mangle]
pub extern fn putfloat(val: f32) -> bool {
    println!("{}", val);
    return true;
}


#[no_mangle]
pub extern fn bool(val: bool) -> bool {
    println!("{}", val);
    return true;
}

#[no_mangle]
pub extern fn putstring(val: &CString) -> bool {
    println!("{:?}", val);
    return true;
}

#[no_mangle]
pub extern fn getinteger() -> i32 {
    let mut readIn = String::new();
    let stdIn = io::stdin();
    stdIn.read_line(&mut readIn).expect("No stdin value found");
    let intVal = readIn.trim().parse::<i32>().expect("Must provide an int for getInt");
    return intVal;
}

#[no_mangle]
pub extern fn getfloat() -> f32 {
    let mut readIn = String::new();
    let stdIn = io::stdin();
    stdIn.read_line(&mut readIn).expect("No stdin value found");
    let intVal = readIn.trim().parse::<f32>().expect("Must provide an float for getInt");
    return intVal;
}


#[no_mangle]
pub extern fn getbool() -> bool {
    let mut readIn = String::new();
    let stdIn = io::stdin();
    stdIn.read_line(&mut readIn).expect("No stdin value found");
    let intVal = readIn.trim().parse::<bool>().expect("Must provide an bool for getInt");
    return intVal;
}

#[no_mangle]
pub extern fn sqrt(input: i32) -> f64{
    let retval = f64::sqrt(input as f64);
    return retval;
}