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
// extern crate funcLib;

mod models;

//package imports
use {
    crate::models::{lexer::Lexer, parser::{Expr, Parser, *}, typechecker::{
        SymbolTable, SyntaxChecker
    }, compiler::*,
    }, anyhow::Result, inkwell::{builder::Builder, OptimizationLevel, passes::PassManager, context::Context, module::Module, types::{BasicMetadataTypeEnum, BasicType, BasicTypeEnum}, values::*, AddressSpace, FloatPredicate, IntPredicate}, parse_display::Display, std::{
        collections::HashMap, env::{self, args}, ffi::CString, fmt, rc::Rc
    }
};

///////////////////////// Setup /////////////////////////

use std::fs::{self, File};
//imports
use std::{io::prelude::*, path::Path};
use std::process::{self, Command};
use inkwell::object_file::Symbol;
use inkwell::targets::{CodeModel, InitializationConfig, RelocMode, Target, TargetMachine, TargetTriple};
// use llvm_sys::target_machine::LLVMTargetMachineOptionsSetRelocMode;

///////////////////////// /Setup /////////////////////////



//The main section of the code
fn main() -> Result<(), Box<dyn std::error::Error>> {    
    // Get the path from command line arguments
    let path = env::args().nth(1).expect("Please specify an input file");
    let mut myLexer = Lexer::new(&path);
    println!("Lexer filename: {} \nCharacter count: {}", myLexer.inputFile.fileName, myLexer.inputFile.numChars);

    // Scan through the input
    myLexer.scanThrough();

    // println!("Lexer reporting: {:?}", myLexer.reports.clone());
    if (myLexer.reports.status) {
        println!("Error in lexer: {:?}", myLexer.reports.clone());
        return Ok(());
    } else {
        println!("Lexer returned successfully");
    }

    // Initialize the parser
    let mut myParser = Parser::new(&mut myLexer);


    // // Print the parser's token list (for debugging)
    // // println!("\n\nMy parser token list: ");
    // // myParser.printTokenList();

    //Parse the program and create the AST
    let mut programAst: Stmt;
    match myParser.startParse() {
        Ok((reporting, Some(stmt))) => {
            println!("Parsing completed successfully.");
            programAst = stmt;
        }
        Ok((reporting, None)) => {
            println!("\n\nParsing succeeded, but no programAST was returned.");
            return Ok(());
        }
        Err(reporting) => {
            eprintln!("\n\nParsing failed.");
            eprintln!("Reporting: {:?}", reporting);
            return Ok(());
        }
    }

    //Display the program AST (for dev/debugging)
    // programAst.display(0);

    //Initialize the checker global table for the checker
    let mut globalTable = SymbolTable::new();
    
    //Initialize the type checker
    let mut myChecker = SyntaxChecker::new(programAst.clone(), &mut globalTable, "Main".to_string());
    println!("\n\nTypeChecker Created");
    
    //Check the program
    let programValid: bool = myChecker.checkProgram();

    //Checks if the checker returned true
    if(!programValid){
        println!("\n\nError in program");
        return Ok(());
    } else {
        println!("\n\nProgram is valid");
    }

    //Initialize the global symbol table
    let mut globalTable: HashMap<String, PointerValue> = HashMap::new();

    //Creates the llvm context and intializes the code generator struct
    let context = Context::create();
    let mut myGen = Compiler::new(programAst.clone(), &context, &mut globalTable, "Program".to_string());
    println!("Created compiler");

    //Run the code generator, this returns an LLVM module that contains LLVM IR
    let ret = myGen.compileProgram();
    
    //Check the result of the code generator to ensure the module is valid
    let mut finalMod: Module;
    match ret{
        Ok(module) => {
            println!("\n\nModule generated");

            //Uncomment this to print entire llvm IR module
            // module.print_to_stderr();
            
            //Sets the final module
            finalMod = module.clone();
        }
        Err(errMsg) => {
            println!("Error with generation: {}", errMsg);
            return Ok(());
        }
    }

    //Initialize LLVM targets
    Target::initialize_all(&InitializationConfig::default());

    //Define the llvm target platform
    let targTriple = TargetMachine::get_default_triple();
    // let targTriple = TargetTriple::create(targArch);

    //Create the actual target object
    let target = Target::from_triple(&targTriple).expect("Failed to get target");
    let targetMachineCheck = target.create_target_machine(
        &targTriple,
        "generic",                        //For zen architecture machine
        "",                         //No feature inclusion
        OptimizationLevel::None,       //No optimizations
        RelocMode::Default,                  //default relocation
        CodeModel::Default,                  //default code model
    );

    //Extract the target machine value
    let mut targetMachine: TargetMachine;
    match targetMachineCheck{
        Some(target) => {
            targetMachine = target;
        }
        None => {
            println!("no target machine");
            return Ok(());
        }
    }

    //Define the path where the object file will be stored
    let path = Path::new("output.o");

    //Write the generated code to an object file
    let writeCode = targetMachine.write_to_file(&finalMod, inkwell::targets::FileType::Object, &path);
    if let Err(e) = writeCode {
        println!("Error generating object file: {}", e);
    }

    //Create the LLVM IR code file
    let outPath = Path::new("./out").with_extension("ll");
    finalMod.print_to_file(&outPath).expect("Could not print ll file");

    //Defines the path of the library (where the builtins are defined)
    let libPath = Path::new("./target/release/libfuncLib").with_extension("a");

    //Create the final output by using clang as the linker for the libray
    let finalOutput = Command::new("clang")
        .current_dir("./")
        .arg(&outPath)
        .arg(&libPath)
        .output()
        .expect("Clang Linker failed"
    );
    let checkSuccess = finalOutput.status;
    if !checkSuccess.success() {
        println!("Error in linking");
        return Ok(());
    }

    //Exit
    Ok(())
}