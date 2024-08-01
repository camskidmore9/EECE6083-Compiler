///////////////////////// Setup /////////////////////////

//Rules
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_parens)]
#![allow(unused_mut)]
#![allow(unused_variables)]

//Crate imports
extern crate anyhow;
extern crate parse_display;
extern crate utf8_chars;
extern crate unicode_segmentation;

//package imports
use {
    crate::models::{lexer::Lexer, parser::{Expr, Parser, *}, typechecker::{
        SymbolTable, SyntaxChecker
    }}, anyhow::Result, core::panic, inkwell::{builder::Builder, context::{self, Context}, module::Module, types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum, FunctionType, PointerType}, values::*, AddressSpace, FloatPredicate, IntPredicate}, parse_display::Display, std::{
        array, collections::HashMap, env::{self, args}, ffi::CString, fmt, rc::Rc
    }
};


//imports
use std::io::prelude::*;

//The enumeration for saving Token types, this is a list of every type of Token there is
#[derive(Clone, PartialEq)]
pub enum tokenTypeEnum{
    //Operators
    PLUS, 
    MINUS,
    LESS,
    GREATER,
    LESS_EQUALS,
    GREATER_EQUALS,
    SET_EQUALS,
    CHECK_EQUALS,
    NOT_EQUALS,
    MULTIPLY,
    DIVIDE,
    AND,
    OR,
    NOT,
    // OPERATOR,
    
    
    //Variable types
    INT,
    FLOAT, 
    STRING,

    //Word types
    IDENTIFIER, 
    
    //Keywords
    IF,
    ELSE,
    GLOBAL,
    VARIABLE,
    THEN,
    END,
    

    IF_RW, 
    LOOP_RW, 
    END_RW, 
    L_PAREN, 
    R_PAREN,
    L_BRACKET, 
    R_BRACKET,
    
    EOF,
    LETTER,
    UNACCOUNTED,
    WORD,
    RETURN,
    ERROR,
    PROGRAM,
    IS,
    BEGIN,
    PROCEDURE,
    SEMICOLON,
    COLON,
    PERIOD,
    END_PROGRAM,
    END_PROCEDURE,
    END_IF,
    END_FOR,
    COMMA,
    FOR,

    PROCEDURE_CALL,
    TRUE,
    FALSE,

    
    
}
//Used to print tokenTypeEnum values
impl fmt::Display for tokenTypeEnum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let variant_str = match self {
            tokenTypeEnum::PLUS => "PLUS",
            tokenTypeEnum::MINUS => "MINUS",
            tokenTypeEnum::IF_RW => "IF_RW",
            tokenTypeEnum::LOOP_RW => "LOOP_RW",
            tokenTypeEnum::END_RW => "END_RW",
            tokenTypeEnum::L_PAREN => "L_PAREN",
            tokenTypeEnum::R_PAREN => "R_PAREN",
            tokenTypeEnum::L_BRACKET => "L_BRACKET",
            tokenTypeEnum::R_BRACKET => "R_BRACKET",
            tokenTypeEnum::INT => "INT",
            tokenTypeEnum::FLOAT => "FLOAT",
            tokenTypeEnum::IDENTIFIER => "IDENTIFIER",
            tokenTypeEnum::LESS => "LESS",
            tokenTypeEnum::GREATER => "GREATER",
            tokenTypeEnum::LESS_EQUALS => "LESS_EQUALS",
            tokenTypeEnum::GREATER_EQUALS => "GREATER_EQUALS",
            tokenTypeEnum::EOF => "EOF",
            tokenTypeEnum::LETTER => "LETTER",
            tokenTypeEnum::UNACCOUNTED => "UNACCOUNTED",
            tokenTypeEnum::WORD => "WORD",
            tokenTypeEnum::STRING => "STRING",
            tokenTypeEnum::RETURN => "RETURN",
            tokenTypeEnum::SET_EQUALS => "SET_EQUALS",
            tokenTypeEnum::CHECK_EQUALS => "CHECK_EQUALS",
            tokenTypeEnum::ERROR => "ERROR",
            tokenTypeEnum::PROGRAM => "PROGRAM",
            tokenTypeEnum::IS => "IS",
            tokenTypeEnum::BEGIN => "BEGIN",
            tokenTypeEnum::PROCEDURE => "PROCEDURE",
            tokenTypeEnum::IF => "IF",
            tokenTypeEnum::ELSE => "ELSE",
            tokenTypeEnum::GLOBAL => "GLOBAL",
            tokenTypeEnum::VARIABLE => "VARIABLE",
            tokenTypeEnum::THEN => "THEN",
            tokenTypeEnum::END => "END",
            tokenTypeEnum::SEMICOLON => "SEMICOLON",
            tokenTypeEnum::COLON => "COLON",
            tokenTypeEnum::PERIOD => "PERIOD",
            tokenTypeEnum::END_PROCEDURE => "END_PROCEDURE",
            tokenTypeEnum::END_PROGRAM => "END_PROGRAM",
            tokenTypeEnum::END_IF => "END_IF",
            tokenTypeEnum::MULTIPLY => "MULTIPLY",
            tokenTypeEnum::DIVIDE => "DIVIDE",
            tokenTypeEnum::COMMA => "COMMA",
            tokenTypeEnum::END_FOR => "END_FOR",
            tokenTypeEnum::FOR => "FOR",
            tokenTypeEnum::PROCEDURE_CALL => "PROCEDURE_CALL",
            tokenTypeEnum::AND => "AND",
            tokenTypeEnum::OR => "OR",
            tokenTypeEnum::NOT => "NOT",
            tokenTypeEnum::NOT_EQUALS => "NOT_EQUALS",
            tokenTypeEnum::TRUE => "TRUE",
            tokenTypeEnum::FALSE => "FALSE",
            // tokenTypeEnum::OPERATOR => "OPERATOR",


        };
        write!(f, "{}", variant_str)
    }
}

///////////////////////// /Setup /////////////////////////


// The IR generator structure
pub struct Compiler<'ctx> {
    context: &'ctx Context,     //the llvm context
    module: Module<'ctx>,       //the llvm module
    builder: Builder<'ctx>,     //the llvm builder
    programAst: Stmt,           //the programAst that will be run through to generate llvm IR
    scope: i32,                 //The scope (i am not sure i need this)
    pub localTable: HashMap<String, PointerValue<'ctx>>, // Local table for the current scope
    pub globalTable: &'ctx mut HashMap<String, PointerValue<'ctx>>, // Shared global table
    pub name: String,
    pub i: i32,
}

impl<'ctx> Compiler<'ctx> {
    //constructor
    pub fn new(
        programAst: Stmt,
        context: &'ctx Context,
        globalTable: &'ctx mut HashMap<String, PointerValue<'ctx>>,
        name: String
    ) -> Compiler<'ctx> {
        let mut module = context.create_module("my_module");
        let mut builder = context.create_builder();
        let mut i: i32 = 0; 
        Compiler {
            programAst,
            scope: 0,
            localTable: HashMap::new(),
            globalTable,
            name,
            context,
            module,
            builder,
            i,
        }
    }


    /////////// COMPILE SECTIONS ///////////
    
    //The main function that is exposed, called by main to run through programAST that is stored
    pub fn compileProgram(&mut self) -> Result<&Module<'ctx>, String>{
        match self.programAst.clone(){
            Stmt::Program(progName, headerBox, bodyBox, lineNum) => {
                //Adds the built in functions
                self.defineGetInt();
                self.definePutInt();
                self.defineGetFloat();
                self.definePutBool();
                self.definePutFloat();
                
                //Creates the main function in module
                let i32Type = self.context.i32_type();
                let mainType = i32Type.fn_type(&[], false);
                let mut mainFunc = self.module.add_function("main", mainType, None);

                //Goes through the header and adds each line to the module
                let header = headerBox.clone();
                let mut progHeader = *header;
                let mainBuilder = self.context.create_builder();
                
                //Creates the local table for the main function
                let mut mainLocalTable: HashMap<String, PointerValue<'ctx>> = HashMap::new();
                //Initializes all of the main function variables as globals
                if let Stmt::Block(ref instrs, lineNum) = progHeader.clone() {
                    for instr in instrs {
                        self.compileStmt(instr.clone(), &mainBuilder, &mut mainLocalTable, mainFunc);
                    }
                } else {
                    println!("Problem with AST: header must be a Block");
                }


                //Creates the entrypoint at the main function
                let mainBlock = self.context.append_basic_block(mainFunc, "mainEntry");
                mainBuilder.position_at_end(mainBlock);
                // println!("Created entry point");

                // println!("Time to go through body");
                //Goes through the body and adds each line to the module
                let newBodyBox = bodyBox.clone();
                let mut body = *newBodyBox;

                // Check if the variable is a Block and iterate through it
                if let Stmt::Block(ref instrs, lineNum) = body.clone() {
                    for instr in instrs {
                        let good = self.compileStmt(instr.clone(), &mainBuilder, &mut mainLocalTable, mainFunc);
                    }
                } else {
                    panic!("Problem with AST: header must be a Block");
                }
                
                //Creates the main function return, a 0 for success
                let mainRet = i32Type.const_int(0, false);
                let _ = mainBuilder.build_return(Some(&mainRet));
            }
            _ => {
                let errMsg = format!("ProgramAst must be a Program Stmt");
                panic!("{}", errMsg);
            }
        }
        
        //Returns the completed module back to main
        return Ok(&self.module);
    }

    //The function that handles statements, it returns a bool (will actually only return true as false values will panic())
    fn compileStmt(&mut self, stmt: Stmt, builder: &Builder<'ctx>, localTable: &mut HashMap<String, PointerValue<'ctx>>, function: FunctionValue) -> bool{
        //A match case to handle every type of Stmt
        match stmt.clone(){
            //For local variable declarations
            Stmt::VarDecl(varName, varType, lineNum) => {
                //A match case to handle each variable type being defined
                match varType{
                    //For bool variables
                    VarType::Bool => {
                        let localType = self.context.bool_type();
                        let localName = varName.clone();

                        //Allocates space for the variable and retrieves its pointer
                        let localVarCheck = builder.build_alloca(localType.clone(), &localName.clone());
                        let localPtr: PointerValue;
                        match localVarCheck{
                            Ok(ptr) => {
                                localPtr = ptr.clone();
                            }
                            Err(err) => {
                                println!("Error allocating local bool variable {}", localName.clone());
                                panic!();
                            }
                        }
                        //Initializes the variable
                        let initVal = localType.const_int(0, false);
                        let _ = builder.build_store(localPtr, initVal);

                        //Inserts the variable into the local symbol table
                        localTable.insert(varName.clone(), localPtr);
                        
                        return true;
                    }
                    //floats
                    VarType::Float => {
                        let localType = self.context.f32_type();
                        let localName = varName.clone();
                        
                        let localVarCheck = builder.build_alloca(localType.clone(), &localName.clone());

                        let localPtr: PointerValue;
                        match localVarCheck{
                            Ok(ptr) => {
                                localPtr = ptr.clone();
                            }
                            Err(err) => {
                                println!("Error allocating local float variable {}", localName.clone());
                                panic!();
                            }
                        }

                        let initVal = localType.const_float(0.0);
                        let _ = builder.build_store(localPtr, initVal);

                        localTable.insert(varName.clone(), localPtr);
                        
                        return true;
                    }
                    //Ints
                    VarType::Int => {
                        let localType = self.context.i32_type();
                        let localName = varName.clone();
                        
                        
                        // let globVar = self.module.add_global(boolType.clone(), None, &boolName);
                        
                        let localVarCheck = builder.build_alloca(localType.clone(), &localName.clone());

                        let localPtr: PointerValue;
                        match localVarCheck{
                            Ok(ptr) => {
                                localPtr = ptr.clone();
                            }
                            Err(err) => {
                                println!("Error allocating local int variable {}: {}", localName.clone(), err);
                                panic!();
                            }
                        }

                        let initVal = localType.const_int(0, false);
                        let _ = builder.build_store(localPtr, initVal);

                        localTable.insert(varName.clone(), localPtr);
                        
                        return true;
                    }
                    //Strings, needs work
                    VarType::Str => {
                        let maxStringLen = 64 as u32 + 1;
                        let i8Type = self.context.i8_type();
                        let arrayType = i8Type.array_type(maxStringLen);
                        // let stringVal: Vec<IntValue> = 

                        
                        let localVarCheck = builder.build_alloca(arrayType.clone(), &varName.clone());

                        let localPtr: PointerValue;
                        match localVarCheck{
                            Ok(ptr) => {
                                localPtr = ptr.clone();
                            }
                            Err(err) => {
                                println!("Error allocating local str variable {}", varName.clone());
                                panic!();
                            }
                        }

                        let string = "EMPTY";
                        let stringBytes = string.as_bytes();
                        let arrayVal = self.context.const_string(stringBytes, false).clone();
            
            
                        // Wrap the array constant in a BasicValueEnum
                        let initVal = BasicValueEnum::ArrayValue(arrayVal);
                        let _ = builder.build_store(localPtr, initVal);

                        localTable.insert(varName.clone(), localPtr);
                        
                        return true;
                    }
                    //int arrays, needs work
                    VarType::IntArray(size) => {
                        let arrSize = size as u32;
                        let i32Type = self.context.i32_type();
                        let arrayType = i32Type.array_type(arrSize);
                        let globName = varName.clone();


                        //Adds to the local variables
                        let localVarCheck = builder.build_alloca(arrayType.clone(), &varName.clone());

                        let localPtr: PointerValue;
                        match localVarCheck{
                            Ok(ptr) => {
                                localPtr = ptr.clone();
                            }
                            Err(err) => {
                                println!("Error allocating local str variable {}", varName.clone());
                                panic!();
                            }
                        }
                        localTable.insert(varName.clone(), localPtr);
                        
                        return true;
                        
                    }
                }
                
            }
            //Global variable declarations
            Stmt::GlobVarDecl(varName, varType, lineNum) => {
                match varType{
                    //Bools
                    VarType::Bool => {
                        //Creates the variable
                        let boolType = self.context.bool_type();
                        let boolName = varName.clone();
                        //Adds it to the global scope
                        let globVar = self.module.add_global(boolType.clone(), None, &boolName);
                        let _ =  globVar.set_initializer(&boolType.const_int(0, false));
                        //Gets the pointer
                        let globPtr = globVar.as_pointer_value();
                        //Adds it to the global symbol table
                        self.globalTable.insert(varName.clone(), globPtr);
                        
                        return true;
                    }
                    //Floats
                    VarType::Float => {
                        let varType = self.context.f32_type();
                        let globName = varName.clone();
                        let globVar = self.module.add_global(varType.clone(), None, &globName);
                        let _ =  globVar.set_initializer(&varType.const_float(0.0));
                        
                        let globPtr = globVar.as_pointer_value();
                        self.globalTable.insert(varName.clone(), globPtr);
                        
                        return true;
                    }
                    //ints
                    VarType::Int => {
                        let varType = self.context.i32_type();
                        let globName = varName.clone();
                        let globVar = self.module.add_global(varType.clone(), None, &globName);
                        
                        let _ =  globVar.set_initializer(&varType.const_int(0, false));
                        
                        let globPtr = globVar.as_pointer_value();
                        self.globalTable.insert(varName.clone(), globPtr);
                        
                        return true;
                    }
                    //Strings (needs work)
                    VarType::Str => {

                        // Define the array type [65 x i8]
                        let max_string_len = 65;
                        let i8_type = self.context.i8_type();
                        let array_type = i8_type.array_type(max_string_len);

                        // Create a string that fits exactly 65 characters with padding and null terminator
                        let string_value = "A".repeat(64) + "\0";

                        // Convert the string into a byte array
                        let string_bytes = string_value.into_bytes();
                        
                        // // Create the constant array with the string bytes
                        // let const_array = array_type.const_array(
                        //     &string_bytes.iter().map(|&byte| i8_type.const_int(byte as u64, false).into()).collect::<Vec<BasicValueEnum>>()
                        // );

                        // Declare the global variable and initialize it
                        let glob_name = varName.clone();
                        let glob_var = self.module.add_global(array_type, Some(AddressSpace::default()), &glob_name);
                        let globPtr = glob_var.as_pointer_value();
                        // let test = unsafe { ArrayValue::new(string_value) };
                        let test = array_type.const_zero();
                        self.globalTable.insert(glob_name.clone(), globPtr);
                        glob_var.set_initializer(&test);
                        
                        return true;
                    }
                    //Integer arrays (needs work)
                    VarType::IntArray(size) => {
                        let arrSize = size as u32;
                        let i32Type = self.context.i32_type();
                        let arrayType = i32Type.array_type(arrSize);
                        let globName = varName.clone();


                        //Adds to the global variables
                        let globVar = self.module.add_global(arrayType.clone(), None, &globName);
                        let globPtr = globVar.as_pointer_value();
                        self.globalTable.insert(varName.clone(), globPtr);
                        
                        return true;
                        
                    }
                }
                
            }
            //For assigning a new value to a variable
            Stmt::Assign(variable, newValue, lineNum) => {
                //Initializes the values we need
                let mut variablePtr: PointerValue;
                let mut newEnumValue: BasicValueEnum;
                let mut varName: String;

                //Checks to make sure the value we are trying to assign to is a variable reference
                //They retrieves the pointer to the variable
                if let Expr::VarRef(ref targName) = variable {
                    // varName = targName.clone();
                    //Checks the local scope table for the variable pointer\
                    //This ensures that global overloading works
                    let checkLocVar = localTable.get(&targName.clone());
                    match checkLocVar{
                        Some(ptr) => {
                            //If it is found in the local table, it sets out intiialized variable for the pointer
                            // println!("Assigninig local variable {} at location {}", targName.clone(), ptr.clone());
                            variablePtr = ptr.clone();
                        }
                        //If its not found in the local table, checks the global table
                        None => {
                            let checkGlobVar = self.globalTable.get(&targName.clone());
                            match checkGlobVar{
                                Some(ptr) => {
                                    // println!("Assigninig global variable {} at location {}", targName.clone(), ptr.clone());
                                    variablePtr = ptr.clone();
                                }
                                //If its not found at at all, panics
                                None => {
                                    panic!("variable {} not found", targName.clone());
                                }
                            }
                        }
                    }
                }
                
                //If the variable we are assigning is a value in an array
                else if let Expr::ArrayRef(ref targName, indexExpr) = variable{
                    // varName = targName.clone();
                    let arrSize = 64 as u32;
                    let i32Type = self.context.i32_type().clone();
                    let arrayType = i32Type.array_type(arrSize).clone();
                    
                    //Gets the value of the index expression
                    let indexExprCheck = self.compileExpr(&*&indexExpr.clone(), builder, localTable);
                    let mut indexVal: BasicValueEnum;
                    match indexExprCheck{
                        Ok(val) => {
                            indexVal = val.clone();
                        }
                        Err(err) => {
                            println!("{}", err.clone());
                            panic!();
                        }
                    }

                    //Gets the loccation of the array
                    let mut arrayPtr: PointerValue;
                    let checkLocVar = localTable.get(&targName.clone());
                    match checkLocVar{
                        Some(ptr) => {
                            // println!("Assigninig local variable {} at location {}", targName.clone(), ptr.clone());
                            arrayPtr = ptr.clone();
                        }
                        None => {
                            let checkGlobVar = self.globalTable.get(&targName.clone());
                            match checkGlobVar{
                                Some(ptr) => {
                                    // println!("Assigninig global array variable {} at location {}", targName.clone(), ptr.clone());
                                    arrayPtr = ptr.clone();
                                }
                                None => {
                                    panic!("variable {} not found", targName.clone());
                                }
                            }
                        }
                    }

                    //Evaluates the index expression into a value
                    let mut indexInt: IntValue;
                    match indexVal{
                        BasicValueEnum::IntValue(val) => {
                            indexInt = val.clone();
                        }
                        BasicValueEnum::FloatValue(val) => {
                            let intType = self.context.i32_type().clone();
                            let intValue = builder.build_float_to_signed_int(val.clone(), intType, "float_to_int");
                            match intValue{
                                Ok(iVal) => {
                                    indexInt = iVal.clone();
                                }
                                Err(err) => {
                                    panic!("Error converting float to int");
                                }
                            }

                        }
                        _ => {
                            panic!("Can only index by integer");
                        }
                    }
                
                    // Get the pointer to the desired index
                    let intType = self.context.i32_type().clone();
                    let zero = intType.const_int(0, false);
                    let indexList = [zero, indexInt];
                    let checkIndexPtr = unsafe { builder.build_gep(arrayPtr, &indexList, "arrayIndexLoad") };
                    match checkIndexPtr{
                        Ok(ptr) => {
                            // println!("GOT ARRAY INDEX PTR");
                            variablePtr = ptr.clone();
                        }
                        Err(err) => {
                            panic!("Error getting array index ptr");
                        }
                    }
                }
                
                //Fails if not a variable
                else {
                    panic!("Cannot assing to a non variable");
                }

                //If the new value we are storing is an array reference, does the same as above but pulls the value instead of the ptr
                if let Expr::ArrayRef(ref targName, indexExpr) = newValue.clone() {
                    let arrSize = 64 as u32;
                    let i32Type = self.context.i32_type().clone();
                    let arrayType = i32Type.array_type(arrSize).clone();
                    
                    //Gets the value of the index expression
                    let indexExprCheck = self.compileExpr(&*&indexExpr.clone(), builder, localTable);
                    let mut indexVal: BasicValueEnum;
                    match indexExprCheck{
                        Ok(val) => {
                            indexVal = val.clone();
                        }
                        Err(err) => {
                            // println!("{}", err.clone());
                            panic!("Could error with index {}:", err.clone());
                        }
                    }

                    //Gets the pointer to the array
                    let mut arrayPtr: PointerValue;
                    let checkLocVar = localTable.get(&targName.clone());
                    match checkLocVar{
                        Some(ptr) => {
                            arrayPtr = ptr.clone();
                        }
                        None => {
                            let checkGlobVar = self.globalTable.get(&targName.clone());
                            match checkGlobVar{
                                Some(ptr) => {
                                    arrayPtr = ptr.clone();
                                }
                                None => {
                                    panic!("variable {} not found", targName.clone());
                                }
                            }
                        }
                    }

                    //Gets the value stored at that index
                    let mut indexInt: IntValue;
                    match indexVal{
                        BasicValueEnum::IntValue(val) => {
                            indexInt = val.clone();
                        }
                        BasicValueEnum::FloatValue(val) => {
                            let intType = self.context.i32_type().clone();
                            let intValue = builder.build_float_to_signed_int(val.clone(), intType, "float_to_int");
                            match intValue{
                                Ok(iVal) => {
                                    indexInt = iVal.clone();
                                }
                                Err(err) => {
                                    panic!("Error converting float to int");
                                }
                            }

                        }
                        _ => {
                            panic!("Can only index by integer");
                        }
                    }
                
                    // Get the pointer to the desired index
                    let variablePtr: PointerValue;
                    let checkIndexPtr = unsafe { builder.build_in_bounds_gep(arrayPtr, &[indexInt], "test") };
                    match checkIndexPtr{
                        Ok(ptr) => {
                            variablePtr = ptr.clone();
                        }
                        Err(err) => {
                            panic!("Error getting array index ptr");
                        }
                    }

                    //Gets the value at that pointer
                    let retValCheck = builder.build_load(variablePtr, "arrayIndexReference");
                    match retValCheck{
                        Ok(val) => {
                            // println!("ARRAY INDEX VALUE GOT {}", val.clone());
                            newEnumValue = val.clone();
                        }
                        Err(msg) => {
                            panic!("Error getting array index value");
                        }
                    }
                
                }
                
                //Evaluates the expression we are setting as the value
                else {
                    let checkNewValue = self.compileExpr(&newValue.clone(), builder, localTable);
                    match checkNewValue.clone(){
                        Ok(value) => {
                            
                            newEnumValue = value.clone();
                        }
                        Err(msg) => {
                            panic!("{}", msg.clone());
                        }
                    }
                }

                
                //Stores the final value a the pointer
                let mut finalVal = newEnumValue.clone();
                match finalVal{
                    BasicValueEnum::IntValue(intVal) => {
                        // println!("Stored int value {} in variable {}",intVal.clone(), varName.clone());
                        let _ = builder.build_store(variablePtr, intVal.clone());
                        return true;
                    }
                    BasicValueEnum::FloatValue(intVal) => {
                        // println!("Stored int value {} in variable {}",intVal.clone(), varName.clone());
                        let _ = builder.build_store(variablePtr, intVal.clone());
                        return true;
                    }
                    BasicValueEnum::ArrayValue(val) => {
                        // println!("ARRAY {}", val.clone());
                        let _ = builder.build_store(variablePtr, val.clone());
                        return true;
                    }
                    _ => {
                        println!("Not implemented for that type yet");
                        return true;
                    }
                }
            }
            //These are for blocks, aka vectors of expressions, just iterates through the vector and evaluates each one
            Stmt::Block(blockStmt, lineNum) => {
                for instr in blockStmt.clone() {
                    let good = self.compileStmt(instr.clone(), builder, localTable, function);
                    if (!good){
                        panic!("Error in block on line {}:", lineNum.clone());
                    } else {
                        //continue
                    }
                }
                return true;
            }
            //For error statements, these never make it here as they are caught by the error
            //checker but its defined here for completion
            Stmt::Error(err, lineNum) => {
                panic!("Somehow an error statemet made it to the compiler. Error from line {}", lineNum.clone());
            }
            //If the statement is a simple expression
            //This is something like a procedure call without assignment
            Stmt::Expr(exprStmt, lineNum) => {
                match (exprStmt.clone()){
                    _ => {
                        //Evaluates the expression
                        let checked = self.compileExpr(&exprStmt.clone(), builder, localTable);
                        match checked {
                            Ok(val) => {
                                return true;
                            }
                            Err(err) => {
                                panic!("Error: {}", err.clone());
                            }
                        }
                    }
                }
            }
            //Creates a for loop
            Stmt::For(assignment, condExpr, body, lineNum) => {
                
                //Parses the assignment first
                let mut iInitVal: BasicValueEnum;
                let mut iName: String;
                let assignStmt = Rc::clone(&assignment);
                if let Stmt::Assign(varRef, val, lineNum) = &*assignStmt.clone() {
                    if let Expr::VarRef(varName) = varRef.clone(){
                        //Checks the value of the iterator assignment
                        let iteratorValCheck = self.compileExpr(&val.clone(), &builder, localTable);
                        match iteratorValCheck{
                            Ok(val) => {
                                // println!("Iterator value: {}", val.clone());
                            }
                            Err(err) => {
                                panic!("Error parsing for loop iterator assignment: {}", err.clone());
                            }
                        }
                    }
                    else {
                        panic!("Error: For loop iterator must be a variable");
                    }
                }
                else {
                    panic!("Error: For loop assignment must be a variable assignment");
                }
                
                
                //set up the blocks of the for loop
                //This is the condition block, it is where the condition expression will be evaluated and checked
                let loopCond = self.context.append_basic_block(function, "forCond");
                //This is the body of the for loop, where the stuff happens
                let loopBody = self.context.append_basic_block(function, "forBody");
                //The is where the program goes after the for loop is finished
                let mergeFor = self.context.append_basic_block(function, "mergeFor");
                

                
                //Adds a branch to the conditional check at the current place in the instructions
                let _ = builder.build_unconditional_branch(loopCond);
                
                //Loop condition block so we can add instructions here
                let _ = builder.position_at_end(loopCond); 

                //Parse the condition into llvm values including loading values if they are variables
                let mut condOp1Val: BasicValueEnum;
                let mut condOp2Val: BasicValueEnum;
                let mut condOp: IntPredicate; 
                if let Expr::RelOp(op1Box, op, op2Box) = condExpr{
                   let op1 = *op1Box.clone();
                   let op2 = *op2Box.clone();
                    //Determins which operator we will use
                    match op{
                        Operator::Greater => {
                            condOp = IntPredicate::SGT;
                        }
                        Operator::Greater_Equal => {
                            condOp = IntPredicate::SGE;
                        }
                        Operator::Less => {
                            condOp = IntPredicate::SLT;
                        }
                        Operator::Less_Equal => {
                            condOp = IntPredicate::SLE;
                        }
                        Operator::Check_Equal => {
                            condOp = IntPredicate::EQ;
                        }
                        Operator::Not_Equals => {
                            condOp = IntPredicate::NE;
                        }
                        _ => {
                            panic!("For condition operator must be logical operator");
                        }
                    }
                    //First gets the values of both operands
                    let op1Res = self.compileExpr(&op1.clone(), &builder, localTable);
                    //Makes sure both results of checked operands are good
                    match op1Res{
                        Ok(res) => {
                            condOp1Val = res;
                        }
                        Err(msg) => {
                            panic!("Error in for loop condition");
                        }
                    }
                    let op2Res = self.compileExpr(&op2.clone(), &builder, localTable);
                    //Makes sure both results of checked operands are good
                    match op2Res{
                        Ok(res) => {
                            condOp2Val = res;
                        }
                        Err(msg) => {
                            panic!("Error in for loop condition");
                        }
                    }
                   
                } else {
                
                    panic!("For loop condition must be a logical operation");
                }

                //Checks/converts the values of the 2 operands, this is for compatability between int and float
                let mut op1Int: IntValue;
                let mut op2Int: IntValue;
                match condOp1Val{
                    BasicValueEnum::IntValue(val) => {
                        op1Int = val.clone();
                    }
                    BasicValueEnum::FloatValue(val) => {
                        let intType = self.context.i32_type().clone();
                        let intVal = builder.build_float_to_signed_int(val.clone(), intType.clone(), "floatToInt");
                        match intVal{
                            Ok(val) => {
                                op1Int = val.clone()
                            }
                            Err(msg) => {
                                panic!("Error converting float to int");
                            }
                        }
                    }
                    _ => {
                        panic!("For loop condition values must be numbers");
                    }
                }
                match condOp2Val{
                    BasicValueEnum::IntValue(val) => {
                        op2Int = val.clone();
                    }
                    BasicValueEnum::FloatValue(val) => {
                        let intType = self.context.i32_type().clone();
                        let intVal = builder.build_float_to_signed_int(val.clone(), intType.clone(), "floatToInt");
                        match intVal{
                            Ok(val) => {
                                op2Int = val.clone()
                            }
                            Err(msg) => {
                                panic!("Error converting float to int");
                            }
                        }
                    }
                    _ => {
                        panic!("For loop condition values must be numbers");
                    }
                }

                //Creates the condition statement
                let conditionCheck = builder.build_int_compare(condOp.clone(), op1Int.clone(), op2Int.clone(), "forLoopCondition");
                let condition: IntValue;
                match conditionCheck{
                    Ok(val) => {
                        condition = val.clone();
                    }
                    Err(msg) => {
                        panic!("Error creating condition");
                    }
                }

                //Adds a conditional branch that checks if the condition is met or if the loop should be taken
                let _ = builder.build_conditional_branch(condition, loopBody, mergeFor);

                //Move builder to the loop body to populate it
                builder.position_at_end(loopBody);

                //Populates the body with statements
                let bodyStmt = *body.clone();
                self.compileStmt(bodyStmt.clone(), &builder, localTable, function);

                //Branch the end of the loop to the condition box to reevaluate the condition
                let _ = builder.build_unconditional_branch(loopCond);

                //Moves builder to the end of the block
                builder.position_at_end(mergeFor);

                // println!("CREATED FOR LOOP ");
                return true;
            }
            //Creates and if/else check
            Stmt::If(condExpr, body, elseStmt, lineNum) => {
                //Parts of the if statement
                //The body of the if
                let ifBody = self.context.append_basic_block(function, "ifBody");
                //The body of the else
                let elseBody = self.context.append_basic_block(function, "elseBody");
                //The section that comes after the if statement where it will continue normal code execution
                let mergeBack = self.context.append_basic_block(function, "ifMerge");
                
                //Parse the condition
                let mut condOp1Val: BasicValueEnum;
                let mut condOp2Val: BasicValueEnum;
                let mut condOp: IntPredicate; 
                //If the condition is an relational operation
                if let Expr::RelOp(op1Box, op, op2Box) = condExpr.clone(){
                   let op1 = *op1Box.clone();
                   let op2 = *op2Box.clone();
                    match op{
                        Operator::Greater => {
                            condOp = IntPredicate::SGT;
                        }
                        Operator::Greater_Equal => {
                            condOp = IntPredicate::SGE;
                        }
                        Operator::Less => {
                            condOp = IntPredicate::SLT;
                        }
                        Operator::Less_Equal => {
                            condOp = IntPredicate::SLE;
                        }
                        Operator::Check_Equal => {
                            condOp = IntPredicate::EQ;
                        }
                        Operator::Not_Equals => {
                            condOp = IntPredicate::NE;
                        }
                        _ => {
                            panic!("For condition operator must be logical operator");
                        }
                    }
                    
                    //Evaluates the operands and gets their values
                    let op1Check = self.compileExpr(&op1.clone(), builder, localTable);
                    match op1Check{
                        Ok(val) => {
                            condOp1Val = val.clone();
                        }
                        Err(err) => {
                            panic!("Error getting if condition op 1: {}", err.clone());
                        }
                    }
                    let op2Check = self.compileExpr(&op2.clone(), builder, localTable);
                    match op2Check{
                        Ok(val) => {
                            condOp2Val = val.clone();
                        }
                        Err(err) => {
                            panic!("Error getting if condition op 2");
                        }
                    } 

                } 
                //If the condition is just a bool
                else if let Expr::BoolLiteral(boolVal) = condExpr.clone() {
                    let intBool = boolVal.clone() as u64;
                    let intVal = self.context.bool_type();
                    let boolConst = intVal.const_int(intBool.clone(), false);
                    let boolVal = BasicValueEnum::IntValue(boolConst.clone());

                    condOp1Val = boolVal.clone();
                    condOp2Val = boolVal.clone();
                    condOp = IntPredicate::EQ;

                } else {
                    panic!("If loop condition must evaluate to a bool");
                }
                
                //Parses operand returns
                let mut op1Int: IntValue;
                let mut op2Int: IntValue;
                match condOp1Val{
                    BasicValueEnum::IntValue(val) => {
                        op1Int = val.clone();
                    }
                    BasicValueEnum::FloatValue(val) => {
                        let intType = self.context.i32_type().clone();
                        let intVal = builder.build_float_to_signed_int(val.clone(), intType.clone(), "floatToInt");
                        match intVal{
                            Ok(val) => {
                                op1Int = val.clone()
                            }
                            Err(msg) => {
                                println!("Error converting float to int");
                                panic!();
                            }
                        }
                    }
                    _ => {
                        panic!("For loop condition values must be numbers");
                    }
                }
                match condOp2Val{
                    BasicValueEnum::IntValue(val) => {
                        op2Int = val.clone();
                    }
                    BasicValueEnum::FloatValue(val) => {
                        let intType = self.context.i32_type().clone();
                        let intVal = builder.build_float_to_signed_int(val.clone(), intType.clone(), "floatToInt");
                        match intVal{
                            Ok(val) => {
                                op2Int = val.clone()
                            }
                            Err(msg) => {
                                println!("Error converting float to int");
                                panic!();
                            }
                        }
                    }
                    _ => {
                        println!("For loop condition values must be numbers");
                        panic!();
                    }
                }

                //Creates the condition expression
                let conditionCheck = builder.build_int_compare(condOp.clone(), op1Int.clone(), op2Int.clone(), "ifCondition");
                let condition: IntValue;
                match conditionCheck{
                    Ok(val) => {
                        condition = val.clone();
                    }
                    Err(msg) => {
                        println!("Error creating condition");
                        panic!("Invalid condition");
                        
                    }
                }

                //Creates the conditional check
                let _ = builder.build_conditional_branch(condition, ifBody, elseBody);

                //Position at the end of the ifBody
                builder.position_at_end(ifBody);
                
                //Add to the if body
                let mut ifRet: bool = false;
                let bodyStmt = *body.clone();
                match bodyStmt.clone(){
                    Stmt::Block(stmtVec, lineNum) => {
                        for stmt in stmtVec.clone(){
                            match stmt.clone(){
                                Stmt::Return(val, lineNum) => {
                                    let checkedIfBody = self.compileStmt(stmt.clone(), builder, localTable, function);
                                    if checkedIfBody{
                                        //continue
                                    } else {
                                        panic!();
                                    }
                                    ifRet = true;
                                    break;
                                }
                                _ => {
                                    let checkedIfBody = self.compileStmt(stmt.clone(), builder, localTable, function);
                                    if checkedIfBody{
                                        //continue
                                    } else {
                                        panic!();
                                    }
                                    ifRet = false;
                                }
                            }
                        }
                    }
                    _ => {
                        panic!("If body must be a block");
                    }
                }

                //Checks if the body body contained a return, if it didnt, tells it to merge back at the end of the condition 
                if !ifRet{
                    let _ = builder.build_unconditional_branch(mergeBack);
                }

                //Move to the end of the else body
                builder.position_at_end(elseBody);

                let mut elseRet: bool = false;
                //Checks if there is an else statement
                match elseStmt.clone(){
                    //If there is an else, evalueates the stmt and adds to that section
                    Some(elseVal) => {
                        let elseStmt = *elseVal.clone();
                        match elseStmt{
                            Stmt::Return(val, lineNum) => {
                                let checkedIfBody = self.compileStmt(bodyStmt.clone(), &builder, localTable, function);
                                if checkedIfBody{
                                    //continue
                                } else {
                                    panic!("Error building if body");
                                }
                                elseRet = true;
                            }
                            Stmt::Block(stmtVec, lineNum) => {
                                for stmt in stmtVec.clone(){
                                    match stmt.clone(){
                                        Stmt::Return(val, lineNum) => {
                                            let checkedIfBody = self.compileStmt(bodyStmt.clone(), &builder, localTable, function);
                                            if checkedIfBody{
                                                //continue
                                            } else {
                                                panic!("Error building if body");
                                            }
                                            elseRet = true;
                                        }
                                        _ => {
                                            let checkedIfBody = self.compileStmt(bodyStmt.clone(), &builder, localTable, function);
                                            if checkedIfBody{
                                                //continue
                                            } else {
                                                println!("Error building if body");
                                                panic!();
                                            }
                                            elseRet = false;
                                        }
                                    }
                                }
                            }
                            _ => {
                                let checkedIfBody = self.compileStmt(bodyStmt.clone(), &builder, localTable, function);
                                if checkedIfBody{
                                    //continue
                                } else {
                                    println!("Error building if body");
                                    panic!();
                                }
                                elseRet = false;
                            }
                        }
                    }
                    None => {
                        // println!("If statement no else");
                        elseRet = false;
                    }
                }

                //If there was not a return in the else, it forces it to branch to the merge
                //This ensures there is no dangling sections of code without terminators
                if !elseRet {
                    let _ = builder.build_unconditional_branch(mergeBack);
                }
                    
                //Moves builder to the end of the block
                builder.position_at_end(mergeBack);

                // println!("CREATED if LOOP ");
                return true;
                
            }
            //For procedure declarations in the header
            Stmt::ProcDecl(procRetType, procName, params, headerBox, bodyBox, lineNum) => {
                
                

                let newProcName = format!("{}{}", self.scope.to_string(), procName.clone());
                let procName = newProcName;
                self.scope += 1; 


                println!("Creating proc {}", procName);
                

                //Creates the local variable hash table
                let mut procLocTable: HashMap<String, PointerValue<'ctx>> = HashMap::new();
                
                //Creates the local builder
                //We will be changing scopes so we make a new builder
                let procBuilder = self.context.create_builder();
                
                //Creates a vec for the param types
                let mut paramTypes: Vec<BasicTypeEnum> = Vec::new();

                //Parses the params
                let paramStmtBlock = *params.clone();
                match paramStmtBlock.clone(){
                    Stmt::Block(params, lineNum) => {
                        for param in params{
                            match param.clone(){
                                Stmt::VarDecl(varName, varType, lineNum) => {
                                    let mut paramType: BasicTypeEnum;
                                    match varType{
                                        VarType::Bool => {
                                            paramType = self.context.bool_type().as_basic_type_enum().clone();
                                        }
                                        VarType::Float => {
                                            paramType = self.context.f32_type().as_basic_type_enum().clone();
                    
                                        }
                                        VarType::Int => {
                                            paramType = self.context.i32_type().as_basic_type_enum().clone();
                    
                                        }
                                        VarType::IntArray(size) => {
                                            let arrSize = size as u32;
                                            let i32Type = self.context.i32_type();
                                            let arrayType = i32Type.array_type(arrSize);
                                            
                                            
                                            paramType = self.context.i32_type().as_basic_type_enum().clone();
                    
                                        }
                                        VarType::Str => {
                                            paramType = self.context.i8_type().as_basic_type_enum().clone();
                                            
                                        }
                                    }
                                    paramTypes.push(paramType.clone());
                                    
                                }
                                _ => {
                                    panic!("Function delcaration parameters can only be local variable declarations");
                                }
                            }
                        }
                    }
                    Stmt::VarDecl(varName, varType, lineNum) => {
                        let mut paramType: BasicTypeEnum;
                        match varType{
                            VarType::Bool => {
                                paramType = self.context.bool_type().as_basic_type_enum().clone();
                            }
                            VarType::Float => {
                                paramType = self.context.f32_type().as_basic_type_enum().clone();
        
                            }
                            VarType::Int => {
                                paramType = self.context.i32_type().as_basic_type_enum().clone();
        
                            }
                            VarType::IntArray(size) => {
                                let arrSize = size as u32;
                                let i32Type = self.context.i32_type();
                                let arrayType = i32Type.array_type(arrSize);
                                
                                
                                paramType = self.context.i32_type().as_basic_type_enum().clone();
        
                            }
                            VarType::Str => {
                                paramType = self.context.i8_type().as_basic_type_enum().clone();
                                
                            }
                        }
                        paramTypes.push(paramType.clone());
                        
                    }
                    _ => {
                        panic!("Function delcaration parameters can only be local variable declarations");
                    }
                }
                    
                // println!("Created param list");
                
                //Gets the procedure return type
                let mut procTypeEnum: BasicTypeEnum;
                match procRetType{
                    VarType::Bool => {
                        procTypeEnum = self.context.bool_type().as_basic_type_enum().clone();
                    }
                    VarType::Float => {
                        procTypeEnum = self.context.f32_type().as_basic_type_enum().clone();

                    }
                    VarType::Int => {
                        procTypeEnum = self.context.i32_type().as_basic_type_enum().clone();

                    }
                    VarType::IntArray(size) => {
                        let arrSize = size as u32;
                        let i32Type = self.context.i32_type();
                        let arrayType = i32Type.array_type(arrSize);
                        
                        
                        procTypeEnum = self.context.i32_type().as_basic_type_enum().clone();

                    }
                    VarType::Str => {
                        procTypeEnum = self.context.i8_type().as_basic_type_enum().clone();
                        
                    }
                }
                
                //Converts the parameters into a useable form for the inkwell function thing
                let paramTypesSlice: Vec<BasicMetadataTypeEnum> = paramTypes.iter().map(|&ty| ty.into()).collect();
                let paramTypesSlice = &paramTypesSlice[..];

                //creates the function type structure
                let funcType = procTypeEnum.fn_type(paramTypesSlice, false);

                //Adds the function to the module and gets its functionvalue
                let procFunVal = self.module.add_function(&procName.clone(), funcType, None);
                let function = procFunVal;


                //Creates the entrypoint at the procedure
                let procEntry = self.context.append_basic_block(procFunVal, "procEntry");
                procBuilder.position_at_end(procEntry);
                // println!("Created entry point");
                
                //Parses the parameter statement and initializes the variables, adding them to the local table
                let parmStmt = paramStmtBlock.clone();
                match parmStmt{
                    Stmt::VarDecl(varName, varType, lineNum) => {
                        let params = procFunVal.get_params();
                        let paramValue = params[1];
                        let paramName = varName.clone();
                        let paramType = paramValue.get_type();
                        //Allocates space
                        let allocaRes = procBuilder.build_alloca(paramType.clone(), &paramName);
                        let paramPtr: PointerValue;
                        match allocaRes{
                            Ok(val) => {
                                paramPtr = val;
                            }
                            Err(err) => {
                                panic!("Error allocating param space {}", err);
                            }
                        }

                        //Stores the parameter passed
                        let _ = procBuilder.build_store(paramPtr, paramValue.clone());

                        //Adds location to the hash table
                        procLocTable.insert(paramName.clone(), paramPtr.clone());


                    }
                    Stmt::Block(stmtVec, lineNum) => {
                        let mut i = 0;
                        for paramStmt in stmtVec.clone(){
                            let curStmt = paramStmt.clone();
                            match curStmt{
                                Stmt::VarDecl(varName, varType, lineNum) => {
                                    let params = procFunVal.get_params();
                                    let paramValue = params[i];
                                    let paramName = varName.clone();
                                    let paramType = paramValue.get_type();
                                    //Allocates space
                                    let allocaRes = procBuilder.build_alloca(paramType.clone(), &paramName);
                                    let paramPtr: PointerValue;
                                    match allocaRes{
                                        Ok(val) => {
                                            paramPtr = val;
                                        }
                                        Err(err) => {
                                            panic!("Error allocating param space {}", err);
                                        }
                                    }

                                    //Stores the parameter
                                    let _ = procBuilder.build_store(paramPtr, paramValue.clone());

                                    //Adds location to the hash table
                                    procLocTable.insert(paramName.clone(), paramPtr.clone());


                                }
                                _ => {
                                    panic!("Parameters must be variable declaration or block");
                                }
                            }
                            i += 1;
                        }
                    }
                    _ => {
                        panic!("Parameters must be variable declaration or block");
                    }
                }

                //Puts the builder at the end of the entrypoints
                procBuilder.position_at_end(procEntry);

                //Goes through the header and adds each line to the module, for declaring more local variables
                let header = headerBox.clone();
                let mut procHeader = *header;
                
                // Check if the variable is a Block and iterate through it
                if let Stmt::Block(ref instrs, lineNum) = procHeader.clone() {
                    for instr in instrs {
                        self.compileStmt(instr.clone(), &procBuilder, &mut procLocTable, function);
                    }
                } else {
                    panic!("Problem with procedure AST: header must be a Block");
                }

                // println!("procedure Header processed");

                //Creates the body of the procedure in the module
                let procBody = self.context.append_basic_block(procFunVal, "procBody");
                let _ = procBuilder.build_unconditional_branch(procBody);

                //Positions the builder at the end of the body
                procBuilder.position_at_end(procBody);

                // println!("Time to go through body");

                //Goes through the body and adds each line to the module
                let newBodyBox = bodyBox.clone();
                let mut body = *newBodyBox;

                // Check if the variable is a Block and iterate through it
                if let Stmt::Block(ref instrs, lineNum) = body.clone() {
                    for instr in instrs {
                        let good = self.compileStmt(instr.clone(), &procBuilder, &mut procLocTable, function);
                    }
                } else {
                    println!("Problem with proc AST: body must be a Block");
                }
                
                self.scope -= 1;

                // println!("Procedure created");
                return true;             
            }
            //The StringLiteral expression type was used for development and debugging
            //It should never make it here but is covered just in case
            Stmt::StringLiteral(str, lineNum) => {
                panic!("StringLiteral Stmt, this should never happen, line {}", lineNum.clone());
            }
            //Return expressions
            Stmt::Return(valueExpr, lineNum) => {
                let retValExpr = valueExpr.clone();
                //IF returning a variable value
                if let Expr::VarRef(varName) = retValExpr.clone(){
                    //This happens if there is no return variable, aka this is a void
                    if varName.clone() == ""{
                        let _ = builder.build_return(None);
                        return true;
                    } 
                    //If returning a variable
                    else {
                        //Evaluates the expression, getting the variables value
                        let exprCheck = self.compileExpr(&retValExpr.clone(), builder, localTable);
                        //Returns the correct type depending on what it evaluates to
                        //Creates the relevant return instruction in the module
                        match exprCheck {
                            Ok(val) => {
                                match val {
                                    BasicValueEnum::IntValue(int_val) => {
                                        let _ = builder.build_return(Some(&int_val));
                                    }
                                    BasicValueEnum::FloatValue(float_val) => {
                                        let _ = builder.build_return(Some(&float_val));
                                    }
                                    BasicValueEnum::PointerValue(ptr_val) => {
                                        let _ = builder.build_return(Some(&ptr_val));
                                    }
                                    BasicValueEnum::ArrayValue(array_val) => {
                                        let _ = builder.build_return(Some(&array_val));
                                    }
                                    BasicValueEnum::StructValue(struct_val) => {
                                        let _ = builder.build_return(Some(&struct_val));
                                    }
                                    BasicValueEnum::VectorValue(vector_val) => {
                                        let _ = builder.build_return(Some(&vector_val));
                                    }
                                }
                                return true;
                            }
                            Err(e) => {
                                // Handle the error case
                                panic!("Failed get return value: {}", e);
                            }
                        }
                        
                    }
                }
                //If we are not returning a variable and are instead returning a value
                else {
                    //Evaluates the value and returns the relevant type
                    let exprCheck = self.compileExpr(&retValExpr.clone(), builder, localTable);
                        match exprCheck {
                            Ok(val) => {
                                match val {
                                    BasicValueEnum::IntValue(int_val) => {
                                        let _ = builder.build_return(Some(&int_val));
                                    }
                                    BasicValueEnum::FloatValue(float_val) => {
                                        let _ = builder.build_return(Some(&float_val));
                                    }
                                    BasicValueEnum::PointerValue(ptr_val) => {
                                        let _ = builder.build_return(Some(&ptr_val));
                                    }
                                    BasicValueEnum::ArrayValue(array_val) => {
                                        let _ = builder.build_return(Some(&array_val));
                                    }
                                    BasicValueEnum::StructValue(struct_val) => {
                                        let _ = builder.build_return(Some(&struct_val));
                                    }
                                    BasicValueEnum::VectorValue(vector_val) => {
                                        let _ = builder.build_return(Some(&vector_val));
                                    }
                                }
                                return true;
                            }
                            Err(e) => {
                                // Handle the error case
                                println!("Failed get return value: {}", e);
                                panic!();
                            }
                        }
                }
                
            }
            //For the entire program statement, this shouldnt happen
            //because the only program statement should be handled by programCompiler
            Stmt::Program(name, headerBox, bodyBox, lineNum) => {
                panic!("Program Stmt, this should never happen. Statement on line {}", lineNum.clone());
            }
            
        }
        
    }

    //The function for evaluating expressions, it does all the necessary things and returns their value as
    //an inkwell BasicEnumValue
    fn compileExpr(&mut self, expr: &Expr, builder: &Builder<'ctx>, localTable: &mut HashMap<String, PointerValue<'ctx>>) -> Result<BasicValueEnum<'ctx>, String> {
        //The match case for all types of expressions
        match expr {
            //The literals, just converts the value into the relevant
            //LLVM type and returns it
            Expr::IntLiteral(value) => {
                let val = value.clone() as u64;
                let intType = self.context.i32_type().clone();
                let intVal = intType.const_int(val, false);
                return Ok(BasicValueEnum::IntValue(intVal));
            }
            Expr::FloatLiteral(value) => {
                // let val = value.clone() as f32;
                let floatType = self.context.f32_type().clone();
                let floatVal = floatType.const_float(value.clone().into());
                return Ok(BasicValueEnum::FloatValue(floatVal.clone()));
            }
            //This needs ironed outs
            Expr::StringLiteral(string) => {
                let stringBytes = string.as_bytes();
    
    
                let arrayVal = self.context.const_string(stringBytes, false).clone();
    
    
                // Wrap the array constant in a BasicValueEnum
                let basicArrayVal = BasicValueEnum::ArrayValue(arrayVal);
                return Ok(basicArrayVal.clone());
    
            }
            //This needs ironed out
            Expr::IntArrayLiteral(size, values) => {
                // println!("intarray NEEDS WRITTEN");
                let i32_type = self.context.i32_type();
                let intValue = i32_type.const_int(0, false);                
                return Ok(BasicValueEnum::IntValue(intValue));
            }
            Expr::BoolLiteral(boolVal) => {
                let boolType = self.context.custom_width_int_type(1).clone();
                let trueVal = BasicValueEnum::IntValue(boolType.const_int(1, false));
                let falseVal = BasicValueEnum::IntValue(boolType.const_int(0, false));
                match boolVal{
                    true => {
                        return Ok(trueVal);
                    }
                    false => {
                        return Ok(falseVal);
                    }
                }
            }
            
            
            //REFERENCES
            //For a variable reference, returns the stored value of the variable
            Expr::VarRef(varName) => {
                //Gets the value if defined in local scope
                let checkLocVar = localTable.get(&varName.clone());
                match checkLocVar{
                    Some(varPtr) => {
                        // println!("Loading local value {} at location {}", varName.clone(), varPtr.clone());
                        let loadedVal = builder.build_load(varPtr.clone(), &varName.clone());
                        match loadedVal{
                            Ok(val) => {
                                return Ok(val.clone());
                            }
                            Err(err) => {
                                panic!("{}", format!("Error with pointer to value {}", varName.clone()));
                            }
                        }
                    }
                    None => {
                        //Gets the value if in global scope
                        let checkGlobVar = self.globalTable.get(&varName.clone());
                            match checkGlobVar{
                                Some(varPtr) => {
                                    // println!("Loading local value {} at location {}", varName.clone(), varPtr.clone());
                                    
                                    let loadedVal = builder.build_load(varPtr.clone(), &varName.clone());
                                    match loadedVal{
                                        Ok(val) => {
                                            return Ok(val.clone());
                                        }
                                        Err(err) => {
                                            panic!("{}", format!("FFFError with pointer to value {}", varName.clone()));
                                        }
                                    }
                                }
                                None => {
                                    let errMsg = format!("Variable {} is not defined", varName.clone());
                                    panic!("{}", errMsg);
                                }
                            }
                    }
                }
                
            }
            //Array references
            Expr::ArrayRef(name, indexExpr) => {
                let targName = name.clone();
                let arrSize = 64 as u32;
                let i32Type = self.context.i32_type().clone();
                let arrayType = i32Type.array_type(arrSize).clone();
                
                //Gets the value of the index expression
                let indexExprCheck = self.compileExpr(&*&indexExpr.clone(), builder, localTable);
                let mut indexVal: BasicValueEnum;
                match indexExprCheck{
                    Ok(val) => {
                        indexVal = val.clone();
                    }
                    Err(err) => {
                        // println!("{}", err.clone());
                        let errMsg = format!("Could error with index {}", err.clone());
                        panic!("{}", errMsg.clone());
                    }
                }
    
                //Gets the pointer to the array 
                let mut arrayPtr: PointerValue;
                let checkLocVar = localTable.get(&targName.clone());
                match checkLocVar{
                    Some(ptr) => {
                        // println!("getting local array {} at location {}", targName.clone(), ptr.clone());
                        arrayPtr = ptr.clone();
                    }
                    None => {
                        let checkGlobVar = self.globalTable.get(&targName.clone());
                        match checkGlobVar{
                            Some(ptr) => {
                                // println!("Gettting global array index  {} at location {}", targName.clone(), ptr.clone());
                                arrayPtr = ptr.clone();
                            }
                            None => {
                                let errMsg = format!("variable {} not found", targName.clone());
                                panic!("{}", errMsg.clone());
                            }
                        }
                    }
                }
    
                //Does the necessary conversions to allow compatability
                let mut indexInt: IntValue;
                match indexVal{
                    BasicValueEnum::IntValue(val) => {
                        indexInt = val.clone();
                    }
                    BasicValueEnum::FloatValue(val) => {
                        let intType = self.context.i32_type().clone();
                        let intValue = builder.build_float_to_signed_int(val.clone(), intType, "float_to_int");
                        match intValue{
                            Ok(iVal) => {
                                indexInt = iVal.clone();
                            }
                            Err(err) => {
                                let errMsg = format!("Error converting float to int");
                                panic!("{}", errMsg.clone());
                            }
                        }
    
                    }
                    _ => {
                        let errMsg = format!("Can only index by integer");
                        panic!("{}", errMsg.clone());
                    }
                }
            
                // Get the pointer to the desired index
                let variablePtr: PointerValue;
                let intType = self.context.i32_type().clone();
                let zero = intType.const_int(0, false);
                let indexList = [zero, indexInt];
                let checkIndexPtr = unsafe { builder.build_gep(arrayPtr, &indexList, "arrayIndexLoad") };
                match checkIndexPtr{
                    Ok(ptr) => {
                        variablePtr = ptr.clone();
                    }
                    Err(err) => {
                        let errMsg = format!("Error getting array index ptr");
                        panic!("{}", errMsg);
                    }
                }
    
                //Gets the value at that pointer and returns it
                let retValCheck = builder.build_load(variablePtr, "arrayIndexReference");
                match retValCheck{
                    Ok(val) => {
                        return Ok(val.clone());
                    }
                    Err(msg) => {
                        let errMsg = format!("Error getting array index value");
                        panic!("{}", errMsg.clone());
                    }
                }
                
            }
            //Procedure call/reference
            Expr::ProcRef(procName, params) => {
                let realProcName = procName.clone();
                let origProcName = format!("{}{}", self.scope - 1, procName.clone());
                let newProcName: String;
                if (procName.contains("get") | procName.contains("put")){
                    newProcName = procName.clone();
                } else {
                    newProcName = format!("{}{}", self.scope.to_string(), procName.clone());
                }
                let procName = newProcName;
                self.scope += 1;    

                
                
                
                
                
                //Get the function value from the module
                let mut function: FunctionValue;
                let functionCheck = self.module.get_function(&procName.clone());
                match functionCheck{
                    Some(fun) => {
                        function = fun.clone();
                    }
                    None => {
                        let functionCheck = self.module.get_function(&&origProcName.clone());
                        match functionCheck{
                            Some(fun) => {
                                function = fun.clone();
                            }
                            None => {
                                let errMsg = format!("Function: {} not found in this scope", realProcName.clone());
                                panic!("{}", errMsg);
                            }
                        }
                    }
                }

                    


                //Compile params and add their values to a vector
                let mut compiledParams: Vec<BasicValueEnum> = Vec::new();
                if let Some(paramExprs) = params.clone(){
                    for param in paramExprs{
                        let paramCheck = self.compileExpr(&param.clone(), builder, localTable);
                        match paramCheck{
                            Ok(val) => {
                                compiledParams.push(val.clone());
                            }
                            Err(err) => {
                                let errMsg = format!("Error parsing function call param: {}", err.clone());
                                panic!("{}", errMsg.clone());
                            }
                        }
                    }
                }

                //COnvert teh vector of params to correct type for calling function
                let params: Vec<BasicMetadataValueEnum> = compiledParams.into_iter().map(|val| val.into()).collect();
                let parmVals = params.as_slice();




                //Create the function call
                let procCallRes = builder.build_call(function, parmVals, "callProc");
                self.scope -= 1;
                
                match procCallRes{
                    Ok(val) => {
                        let retVal = val.try_as_basic_value().left().unwrap();
                        return Ok(retVal.clone());
                    }
                    Err(err) => {
                        let errMsg = format!("Error calling procedure");
                        panic!("{}", errMsg);
                    }
                }
            }
            

            //EXPRESSIONS
            //Arithmetic operations
            Expr::ArthOp(op1, op, op2) => {
                //Sets up types to be used
                let intType = self.context.i32_type().clone();
                let floatType = self.context.f32_type().clone();
    
                //First evaluates the values of both operands
                let op1Res = self.compileExpr(&*op1.clone(), builder, localTable).clone();
                let op2Res = self.compileExpr(&*op2.clone(), builder, localTable).clone();
                let mut op1Val: BasicValueEnum;
                let mut op2Val: BasicValueEnum;
                //Makes sure both results of checked operands are good
                match op1Res.clone(){
                    Ok(res) => {
                        op1Val = res.clone();
                    }
                    Err(msg) => {
                        panic!("{}", msg.clone());
                    }
                }
                match op2Res.clone(){
                    Ok(res) => {
                        op2Val = res.clone();
                    }
                    Err(msg) => {
                        panic!("{}", msg.clone());
                    }
                }
    
                //Checks if either value is a float
                let op1IsFloat: bool;
                match op1Val.clone(){
                    BasicValueEnum::FloatValue(_) => {
                        op1IsFloat = true;
                    }
                    _ => {
                        op1IsFloat = false;
                    }
                };
                let op2IsFloat: bool;
                match op2Val.clone(){
                    BasicValueEnum::FloatValue(_) => {
                        op2IsFloat = true;
                    }
                    _ => {
                        op2IsFloat = false;
                    }
                };
    
                //a match case to handle the different types of operators
                match op.clone(){
                    //Addition
                    Operator::Add => {
                        //If either result is a float, casts the int to float and does the float addition
                        if op1IsFloat.clone() || op2IsFloat.clone() {
                            
                            //Checks if op1 is float, casts it to float if not
                            let op1Float: FloatValue;
                            match op1Val.clone() {
                                BasicValueEnum::FloatValue(val) => op1Float = val,
                                BasicValueEnum::IntValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_signed_int_to_float(val, floatType, "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op1Float = val.clone();
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            //Checks if op2 is float, casts it to float if not
                            let op2Float: FloatValue;
                            match op2Val.clone() {
                                BasicValueEnum::FloatValue(val) => {
                                    op2Float = val.clone();
                                }
                                BasicValueEnum::IntValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_signed_int_to_float(val, floatType, "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op2Float = val.clone();
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            //Creates the float addition and adds it to the statement
                            let retOp = builder.build_float_add(op1Float, op2Float, "addFloat");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::FloatValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                        } 
                        
                        // Both operands are integers
                        else {
                            let op1Int = op1Val.into_int_value();
                            let op2Int = op2Val.into_int_value();
                            let retOp = builder.build_int_add(op1Int.clone(), op2Int.clone(), "addInt");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::IntValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                            
                        }
                    }
                    //Subtraction
                    Operator::Sub => {
                        //If either result is a float
                        if op1IsFloat || op2IsFloat {
                            
                            //Checks if op1 is float, casts it to float if not
                            let op1Float: FloatValue;
                            match op1Val {
                                BasicValueEnum::FloatValue(val) => op1Float = val,
                                BasicValueEnum::IntValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_signed_int_to_float(val, floatType, "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op1Float = val.clone();
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            //Checks if op2 is float, casts it to float if not
                            let op2Float: FloatValue;
                            match op2Val {
                                BasicValueEnum::FloatValue(val) => {
                                    op2Float = val;
                                }
                                BasicValueEnum::IntValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_signed_int_to_float(val, floatType, "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op2Float = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            //Does the float add
                            let retOp = builder.build_float_sub(op1Float, op2Float, "subFloat");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::FloatValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                        } 
                        // Both operands are integers
                        else {
                            let op1Int = op1Val.into_int_value();
                            let op2Int = op2Val.into_int_value();
                            let retOp = builder.build_int_sub(op1Int.clone(), op2Int.clone(), "subInt");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::IntValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                            
                        }
                    } 
                    //Multiplication
                    Operator::Mul => {
                        //If either result is a float
                        if op1IsFloat || op2IsFloat {
                            
                            //Checks if op1 is float, casts it to float if not
                            let op1Float: FloatValue;
                            match op1Val {
                                BasicValueEnum::FloatValue(val) => op1Float = val,
                                BasicValueEnum::IntValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_signed_int_to_float(val, floatType, "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op1Float = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            //Checks if op2 is float, casts it to float if not
                            let op2Float: FloatValue;
                            match op2Val {
                                BasicValueEnum::FloatValue(val) => {
                                    op2Float = val;
                                }
                                BasicValueEnum::IntValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_signed_int_to_float(val, floatType, "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op2Float = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            //Does the float add
                            let retOp = builder.build_float_mul(op1Float, op2Float, "multiplyFloat");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::FloatValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                        } 
                        //Both operands are integers
                        else {
                            let op1Int = op1Val.clone().into_int_value();
                            let op2Int = op2Val.clone().into_int_value();
                            let retOp = builder.build_int_mul(op1Int.clone(), op2Int.clone(), "multiplyInt");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::IntValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                            
                        }
                    }
                    //division
                    Operator::Div => {
                        //If either result is a float
                        if op1IsFloat || op2IsFloat {
                            
                            //Checks if op1 is float, casts it to float if not
                            let op1Float: FloatValue;
                            match op1Val {
                                BasicValueEnum::FloatValue(val) => op1Float = val,
                                BasicValueEnum::IntValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_signed_int_to_float(val, floatType, "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op1Float = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            //Checks if op2 is float, casts it to float if not
                            let op2Float: FloatValue;
                            match op2Val {
                                BasicValueEnum::FloatValue(val) => {
                                    op2Float = val;
                                }
                                BasicValueEnum::IntValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_signed_int_to_float(val, floatType, "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op2Float = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            //Does the float add
                            let retOp = builder.build_float_div(op1Float, op2Float, "divideFloat");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::FloatValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                        } 
                        // Both operands are integers
                        else {
                            let op1Int = op1Val.into_int_value();
                            let op2Int = op2Val.into_int_value();
                            let retOp = builder.build_int_signed_div(op1Int.clone(), op2Int.clone(), "divideInt");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::IntValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                            
                        }
                    }
                    _ => {
                        //This should never happen because of parsing and error checking
                        panic!("Improper operator for arthimatic operation");
                    }
                }
            
            }
            //Relational operation
            Expr::RelOp(op1, op, op2) => {   
                
                //First gets the values of both operands
                let op1Res = self.compileExpr(&*op1.clone(), builder, localTable);
                let op2Res = self.compileExpr(&*op2.clone(), builder, localTable);
                let mut op1Val: BasicValueEnum;
                let mut op2Val: BasicValueEnum;
                //Makes sure both results of checked operands are good
                match op1Res{
                    Ok(res) => {
                        op1Val = res.clone();
                    }
                    Err(msg) => {
                        panic!("{}", msg.clone());
                    }
                }
                match op2Res{
                    Ok(res) => {
                        op2Val = res.clone();
                    }
                    Err(msg) => {
                        panic!("{}", msg.clone());
                    }
                }
    
                //Checks if either value is a float
                let op1IsFloat: bool;
                match op1Val.clone(){
                    BasicValueEnum::FloatValue(_) => {
                        op1IsFloat = true;
                    }
                    _ => {
                        op1IsFloat = false;
                    }
                };
                let op2IsFloat: bool;
                match op2Val.clone(){
                    BasicValueEnum::FloatValue(_) => {
                        op2IsFloat = true;
                    }
                    _ => {
                        op2IsFloat = false;
                    }
                };
    
                //a match case to handle the different types of operators
                match op{
                    Operator::Check_Equal => {
                        //If either result is a float
                        if op1IsFloat || op2IsFloat {
                            
                            //Checks if op1 is float, casts it to float if not
                            let op1Float: FloatValue;
                            match op1Val {
                                BasicValueEnum::FloatValue(val) => op1Float = val,
                                BasicValueEnum::IntValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_signed_int_to_float(val, self.context.f32_type(), "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op1Float = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            //Checks if op2 is float, casts it to float if not
                            let op2Float: FloatValue;
                            match op2Val {
                                BasicValueEnum::FloatValue(val) => {
                                    op2Float = val;
                                }
                                BasicValueEnum::IntValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_signed_int_to_float(val, self.context.f32_type(), "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op2Float = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            //Does the float equality check
                            let retOp = builder.build_float_compare(FloatPredicate::OEQ,op1Float, op2Float, "equalFloat");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::IntValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                        } 
                        // Both operands are integers
                        else {
                            let op1Int = op1Val.into_int_value();
                            let op2Int = op2Val.into_int_value();
                            let retOp = builder.build_int_compare(IntPredicate::EQ,op1Int, op2Int, "equalInt");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::IntValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                            
                        }
                    }
                    Operator::Greater => {
                        //If either result is a float
                        if op1IsFloat || op2IsFloat {
                            
                            //Checks if op1 is float, casts it to float if not
                            let op1Float: FloatValue;
                            match op1Val {
                                BasicValueEnum::FloatValue(val) => op1Float = val,
                                BasicValueEnum::IntValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_signed_int_to_float(val, self.context.f32_type(), "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op1Float = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for greater"),
                            };
    
                            //Checks if op2 is float, casts it to float if not
                            let op2Float: FloatValue;
                            match op2Val {
                                BasicValueEnum::FloatValue(val) => {
                                    op2Float = val;
                                }
                                BasicValueEnum::IntValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_signed_int_to_float(val, self.context.f32_type(), "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op2Float = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            //Does the float equality check
                            let retOp = builder.build_float_compare(FloatPredicate::OGT,op1Float, op2Float, "floatGreater");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::IntValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                        } 
                        // Both operands are integers
                        else {
                            let op1Int = op1Val.into_int_value();
                            let op2Int = op2Val.into_int_value();
                            let retOp = builder.build_int_compare(IntPredicate::SGT,op1Int, op2Int, "intGreater");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::IntValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                            
                        }
                    }
                    Operator::Greater_Equal => {
                        //If either result is a float
                        if op1IsFloat || op2IsFloat {
                            
                            //Checks if op1 is float, casts it to float if not
                            let op1Float: FloatValue;
                            match op1Val {
                                BasicValueEnum::FloatValue(val) => op1Float = val,
                                BasicValueEnum::IntValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_signed_int_to_float(val, self.context.f32_type(), "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op1Float = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            //Checks if op2 is float, casts it to float if not
                            let op2Float: FloatValue;
                            match op2Val {
                                BasicValueEnum::FloatValue(val) => {
                                    op2Float = val;
                                }
                                BasicValueEnum::IntValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_signed_int_to_float(val, self.context.f32_type(), "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op2Float = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            //Does the float equality check
                            let retOp = builder.build_float_compare(FloatPredicate::OGE,op1Float, op2Float, "floatGreaterEqual");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::IntValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                        } 
                        // Both operands are integers
                        else {
                            let op1Int = op1Val.into_int_value();
                            let op2Int = op2Val.into_int_value();
                            let retOp = builder.build_int_compare(IntPredicate::SGE,op1Int, op2Int, "intGreaterEqual");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::IntValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                            
                        }
                    }
                    Operator::Less => {
                        //If either result is a float
                        if op1IsFloat || op2IsFloat {
                            
                            //Checks if op1 is float, casts it to float if not
                            let op1Float: FloatValue;
                            match op1Val {
                                BasicValueEnum::FloatValue(val) => op1Float = val,
                                BasicValueEnum::IntValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_signed_int_to_float(val, self.context.f32_type(), "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op1Float = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            //Checks if op2 is float, casts it to float if not
                            let op2Float: FloatValue;
                            match op2Val {
                                BasicValueEnum::FloatValue(val) => {
                                    op2Float = val;
                                }
                                BasicValueEnum::IntValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_signed_int_to_float(val, self.context.f32_type(), "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op2Float = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            //Does the float equality check
                            let retOp = builder.build_float_compare(FloatPredicate::OLT,op1Float, op2Float, "floatLess");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::IntValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                        } 
                        // Both operands are integers
                        else {
                            let op1Int = op1Val.into_int_value();
                            let op2Int = op2Val.into_int_value();
                            let retOp = builder.build_int_compare(IntPredicate::SLT,op1Int, op2Int, "intLess");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::IntValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                            
                        }
                    }
                    Operator::Less_Equal => {
                        //If either result is a float
                        if op1IsFloat || op2IsFloat {
                            
                            //Checks if op1 is float, casts it to float if not
                            let op1Float: FloatValue;
                            match op1Val {
                                BasicValueEnum::FloatValue(val) => op1Float = val,
                                BasicValueEnum::IntValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_signed_int_to_float(val, self.context.f32_type(), "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op1Float = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            //Checks if op2 is float, casts it to float if not
                            let op2Float: FloatValue;
                            match op2Val {
                                BasicValueEnum::FloatValue(val) => {
                                    op2Float = val;
                                }
                                BasicValueEnum::IntValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_signed_int_to_float(val, self.context.f32_type(), "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op2Float = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            //Does the float equality check
                            let retOp = builder.build_float_compare(FloatPredicate::OLE,op1Float, op2Float, "floatLessEqual");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::IntValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                        } 
                        // Both operands are integers
                        else {
                            let op1Int = op1Val.into_int_value();
                            let op2Int = op2Val.into_int_value();
                            let retOp = builder.build_int_compare(IntPredicate::SLE,op1Int, op2Int, "intLessEqual");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::IntValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                            
                        }
                    }
                    Operator::Not_Equals => {
                        //If either result is a float
                        if op1IsFloat || op2IsFloat {
                            
                            //Checks if op1 is float, casts it to float if not
                            let op1Float: FloatValue;
                            match op1Val {
                                BasicValueEnum::FloatValue(val) => op1Float = val,
                                BasicValueEnum::IntValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_signed_int_to_float(val, self.context.f32_type(), "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op1Float = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for not equal"),
                            };
    
                            //Checks if op2 is float, casts it to float if not
                            let op2Float: FloatValue;
                            match op2Val {
                                BasicValueEnum::FloatValue(val) => {
                                    op2Float = val;
                                }
                                BasicValueEnum::IntValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_signed_int_to_float(val, self.context.f32_type(), "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op2Float = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            //Does the float equality check
                            let retOp = builder.build_float_compare(FloatPredicate::ONE,op1Float, op2Float, "floatNotEqual");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::IntValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                        } 
                        // Both operands are integers
                        else {
                            let op1Int = op1Val.into_int_value();
                            let op2Int = op2Val.into_int_value();
                            let retOp = builder.build_int_compare(IntPredicate::NE,op1Int, op2Int, "intNotEqual");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::IntValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                            
                        }
                    }
                    _ => {
                        //This should never happen because of parsing and error checking
                        panic!("Improper operator for logical operation");
                    }
                }
            
            }
            //Logical expressions
            Expr::LogOp(op1, op, op2) => {

                
                //First gets the values of both operands
                let op1Res = self.compileExpr(&*op1.clone(), builder, localTable);
                let op2Res = self.compileExpr(&*op2.clone(), builder, localTable);
                let mut op1Val: BasicValueEnum;
                let mut op2Val: BasicValueEnum;
                //Makes sure both results of checked operands are good
                match op1Res{
                    Ok(res) => {
                        op1Val = res;
                    }
                    Err(msg) => {
                        panic!("{}", msg.clone());
                    }
                }
                match op2Res{
                    Ok(res) => {
                        op2Val = res;
                    }
                    Err(msg) => {
                        panic!("{}", msg.clone());
                    }
                }
    
                //Checks if either value is a float
                let op1IsFloat: bool;
                match op1Val.clone(){
                    BasicValueEnum::FloatValue(_) => {
                        op1IsFloat = true;
                    }
                    _ => {
                        op1IsFloat = false;
                    }
                };
                let op2IsFloat: bool;
                match op2Val.clone(){
                    BasicValueEnum::FloatValue(_) => {
                        op2IsFloat = true;
                    }
                    _ => {
                        op2IsFloat = false;
                    }
                };
    
    
                //a match case to handle the different types of operators
                match op{
                    Operator::And => {
                        //If either result is a float
                        if op1IsFloat || op2IsFloat {
                            
                            //Checks if op1 is float, casts it to int if not
                            let op1Int: IntValue;
                            match op1Val {
                                BasicValueEnum::IntValue(val) => op1Int = val,
                                BasicValueEnum::FloatValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_float_to_signed_int(val, self.context.i32_type(), "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op1Int = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            //Checks if op2 is float, casts it to float if not
                            let op2Int: IntValue;
                            match op2Val {
                                BasicValueEnum::IntValue(val) => op2Int = val,
                                BasicValueEnum::FloatValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_float_to_signed_int(val, self.context.i32_type(), "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op2Int = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            let retOp = builder.build_and(op1Int, op2Int, "intAnd");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::IntValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
    
                        } 
                        // Both operands are integers
                        else {
                            let op1Int = op1Val.into_int_value();
                            let op2Int = op2Val.into_int_value();
                            let retOp = builder.build_and(op1Int, op2Int, "intAnd");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::IntValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                            
                        }
                        
                    }
                    Operator::Or => {
                        //If either result is a float
                        if op1IsFloat || op2IsFloat {
                            
                            //Checks if op1 is float, casts it to int if not
                            let op1Int: IntValue;
                            match op1Val {
                                BasicValueEnum::IntValue(val) => op1Int = val,
                                BasicValueEnum::FloatValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_float_to_signed_int(val, self.context.i32_type(), "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op1Int = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            //Checks if op2 is float, casts it to float if not
                            let op2Int: IntValue;
                            match op2Val {
                                BasicValueEnum::IntValue(val) => op2Int = val,
                                BasicValueEnum::FloatValue(val) => {
                                    // Convert integer to float if necessary
                                    let resConv = builder.build_float_to_signed_int(val, self.context.i32_type(), "intToFloat");
                                    match resConv{
                                        Ok(val) => {
                                            op2Int = val;
                                        }
                                        Err(errMsg) => {
                                            panic!("{}", format!("{}", errMsg));
                                        }
                                    }
                                },
                                _ => panic!("Unsupported type for addition"),
                            };
    
                            let retOp = builder.build_or(op1Int, op2Int, "intOr");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::IntValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
    
                        } 
                        // Both operands are integers
                        else {
                            let op1Int = op1Val.into_int_value();
                            let op2Int = op2Val.into_int_value();
                            let retOp = builder.build_or(op1Int, op2Int, "intOr");
                            match retOp{
                                Ok(result) => {
                                    return Ok(BasicValueEnum::IntValue(result.clone()));
                                }
                                Err(errMsg) => {
                                    panic!("{}", format!("{}", errMsg));
                                }
                            }
                            
                        }
                        
                    }
                    
                    _ => {
                        //This should never happen because of parsing and error checking
                        panic!("Improper operator for logical operation");
                    }
                }
                
            }
        }
    }
    
    /////////// /COMPILE SECTIONS ///////////



    /////////// /BUILT IN SECTIONS ///////////
    //This section defines and imports all of the built in functions
    //These functions all come from funcLib library and are defined in lib.rs
    //putinteger
    fn definePutInt(&mut self) {
        let intType = self.context.i32_type();
        let retType = self.context.bool_type();
        let paramTypes = vec![BasicMetadataTypeEnum::from(intType)];
        let parmVals = paramTypes.as_slice();
        let printFnType = retType.fn_type(parmVals, false);
        let putInt = self.module.add_function("putinteger", printFnType, None);        
    }
    //putbool
    fn definePutBool(&mut self) {
        let boolType = self.context.bool_type();
        let retType = self.context.bool_type();
        let paramTypes = vec![BasicMetadataTypeEnum::from(boolType)];
        let parmVals = paramTypes.as_slice();
        let printFnType = retType.fn_type(parmVals, false);
        let putInt = self.module.add_function("putbool", printFnType, None);        
    }
    //putfloat
    fn definePutFloat(&mut self) {
        let floatType = self.context.f32_type();
        let retType = self.context.bool_type();
        let paramTypes = vec![BasicMetadataTypeEnum::from(floatType)];
        let parmVals = paramTypes.as_slice();
        let printFnType = retType.fn_type(parmVals, false);
        let putInt = self.module.add_function("putfloat", printFnType, None);        
    }
    //putstring (NEEDS ADJUSTED)
    fn definePutStr(&mut self) {
        let i8_type = self.context.i8_type();
        let array_type = i8_type.array_type(65);
        let string_type = array_type.ptr_type(AddressSpace::default());
        let retType = self.context.bool_type();
        let paramTypes = vec![BasicMetadataTypeEnum::from(string_type)];
        let parmVals = paramTypes.as_slice();
        let printFnType = retType.fn_type(parmVals, false);
        let putInt = self.module.add_function("putstring", printFnType, None);
    }

    //getinteger
    fn defineGetInt(&mut self) {
        let intType = self.context.i32_type();
        let paramTypes = vec![];
        let parmVals = paramTypes.as_slice();
        let getIntType = intType.fn_type(parmVals, false);
        let putInt = self.module.add_function("getinteger", getIntType, None);
    }
    //getfloat
    fn defineGetFloat(&mut self) {
        let intType = self.context.f32_type();
        let paramTypes = vec![];
        let parmVals = paramTypes.as_slice();
        let getIntType = intType.fn_type(parmVals, false);
        let putInt = self.module.add_function("getfloat", getIntType, None);
    }
    //getbool
    fn defineGetBool(&mut self) {
        let intType = self.context.bool_type();
        let paramTypes = vec![];
        let parmVals = paramTypes.as_slice();
        let getIntType = intType.fn_type(parmVals, false);
        let putInt = self.module.add_function("getbool", getIntType, None);
    }

}

