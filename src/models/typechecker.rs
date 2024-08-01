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
    anyhow::Result, parse_display::Display, std::{
        collections::HashMap, env, fmt, fs::{
            read_to_string, File
        }, hash::Hash, io::{
            prelude::*, BufRead, BufReader, Read
        }, path::Path, rc::Rc


    }, unicode_segmentation::UnicodeSegmentation,
    utf8_chars::BufReadCharsExt,
    crate::tokenTypeEnum,
    crate::models::lexer::{
        Lexer,
        Token,
        tokenGroup,
    },
    crate::models::parser::{
        Parser,
        Stmt,
        Expr,
        VarType,
    },
    std::io::prelude::*,

};

///////////////////////// /Setup /////////////////////////



///////////////////////// TYPE CHECKING SECTION /////////////////////////
//The main type checking structure
pub struct SyntaxChecker<'a> {
    pub valid: bool,                        //The validity of the program
    pub ast: Stmt,                          //the program AST
    pub scope: i32,                         //The scope
    pub localTable: SymbolTable,           //The local table, stays in the scope
    pub globalTable: &'a mut SymbolTable,   //The global table, passed through every scope
    pub name: String,                       //the name of the program (or procedure if in a nested scope)
    pub checked: bool,                      //Set to true when the checking has been finished (used by procedures when scope increases)
}
//The methods within typeChecker
impl<'a> SyntaxChecker<'a> {
    //The constructor
    pub fn new(mut programAst: Stmt, globalTable: &'a mut SymbolTable, name: String) -> SyntaxChecker<'a> {
        SyntaxChecker{
            valid: true,
            ast: programAst.clone(),
            scope: 0,
            localTable: SymbolTable::new(),
            globalTable,
            name,
            checked: false,
        }
    }

    //The constructor when starting a new scope, passes the global table into itself
    pub fn newScope<'b>(
        &'b mut self, 
        procAst: Stmt, 
        curScope: i32, 
        name: String
    ) -> SyntaxChecker<'b>
    where
        'a: 'b,
    {
        SyntaxChecker {
            valid: true,
            ast: procAst,
            scope: curScope + 1,
            localTable: SymbolTable::new(),
            globalTable: self.globalTable,
            name,
            checked: false,
        }
    }

    //The main outward facing checker, checks the two parts of the program
    pub fn checkProgram(&mut self) -> bool {
        match &self.ast.clone() {
            Stmt::Program(name, header, body, lineNum) => {
                
                //Parses and checks the header
                let head = header.clone();
                let mut progHeader = *head;
                // Check if the variable is a Block and iterate through it
                if let Stmt::Block(ref instrs, lineNum) = progHeader.clone() {
                    for instr in instrs {
                        let good = self.checkStmt(instr.clone());
                        if (!good){
                            println!("Error in header:");
                            instr.display(0);
                            return false;
                        } else {
                        }
                    }
                } else {
                    println!("Problem with AST: header must be a Block");
                }

                // println!("Finished checking header:");

                //Parses and checks the body
                let main = body.clone();
                let mut progBody = *main;
                // Check if the variable is a Block and iterate through it
                if let Stmt::Block(ref instrs, lineNum) = progBody {
                    for instr in instrs {
                        let good = self.checkStmt(instr.clone());
                        if (!good){
                            println!("Error in body:");
                            return false;
                        } else {
                            //continue
                        }
                    }
                } else {
                    println!("Problem with AST: header must be a Block");
                }
                self.checked = true;
                return true
            }
            _ => {
                println!("TypeChecker must be passed a Program AST");
                return false;
            }
        }
    }
    

    //For checking the compatability between 2 variable/constant types
    fn checkTypeCompatability(&mut self, target: VarType, new: VarType) -> bool {
        match target.clone(){
            VarType::Bool => {
                match new.clone(){
                    VarType::Bool => {
                        return true;
                    }
                    VarType::Float => {
                        return false;
                    }
                    VarType::Int => {
                        return true;
                    }
                    VarType::Str => {
                        return false;
                    }
                    VarType::IntArray(size) => {
                        return false;
                    }
                }
            }
            VarType::Float => {
                match new.clone(){
                    VarType::Bool => {
                        return false;
                    }
                    VarType::Float => {
                        return true;
                    }
                    VarType::Int => {
                        return true;
                    }
                    VarType::Str => {
                        return false;
                    }
                    VarType::IntArray(size) => {
                        return false;
                    }
                }
            }
            VarType::Int => {
                // println!("Checking int");
                match new.clone(){
                    VarType::Bool => {
                        return true;
                    }
                    VarType::Float => {
                        return true;
                    }
                    VarType::Int => {
                        return true;
                    }
                    VarType::Str => {
                        return false;
                    }
                    VarType::IntArray(size) => {
                        return false;
                    }
                }
            }
            VarType::Str => {
                match new.clone(){
                    VarType::Bool => {
                        return false;
                    }
                    VarType::Float => {
                        return false;
                    }
                    VarType::Int => {
                        return false;
                    }
                    VarType::Str => {
                        return true;
                    }
                    VarType::IntArray(size) => {
                        return false;
                    }
                }
            }
            VarType::IntArray(targSize) => {
                match new.clone(){
                    VarType::Bool => {
                        return false;
                    }
                    VarType::Float => {
                        return false;
                    }
                    VarType::Int => {
                        return false;
                    }
                    VarType::Str => {
                        return false;
                    }
                    VarType::IntArray(newSize) => {
                        if(targSize == newSize){
                            return true;
                        } else {
                            return false;
                        }
                    }
                }
            }
        }
    }

    //For chacking compatability between a target vartype and an expression (for when your assigning a value)
    fn checkExprTypeCompatability(&mut self, target: VarType, new: Expr) -> bool {
        let checked = self.checkExpr(new.clone());
        if checked {
            match target.clone(){
                VarType::Bool => {
                    match new{
                        //Literals
                        Expr::IntLiteral(val) => {
                            return true;
                        }
                        Expr::FloatLiteral(val) => {
                            return false;
                        }
                        Expr::StringLiteral(val) => {
                            return false;
                        }
                        Expr::BoolLiteral(val) => {
                            return true;
                        }
                        Expr::IntArrayLiteral(size, val) => {
                            return false;
                        }
                    
                        //References
                        Expr::VarRef(varName) => {
                            let varTypeLocCheck = self.localTable.getType(&varName.clone());
                            match varTypeLocCheck{
                                Some(varType) => {
                                    let compat = self.checkTypeCompatability(target.clone(), varType.clone());
                                    return compat;
                                }
                                None => {
                                    let varGlobTypeCheck = self.localTable.getType(&varName.clone());
                                    match varGlobTypeCheck{
                                        Some(varType) => {
                                            let compat = self.checkTypeCompatability(target.clone(), varType.clone());
                                            return compat;
                                        }
                                        None => {
                                            println!("Variable {} not defined", varName.clone());
                                            return false;
                                        }
                                    }
                                }
                            }
                            
                            
                        }
                        //References
                        Expr::ProcRef(varName, params) => {
                            let procTypeLocCheck = self.localTable.getType(&varName.clone());
                            match procTypeLocCheck{
                                Some(varType) => {
                                    let compat = self.checkTypeCompatability(target.clone(), varType.clone());
                                    return compat;
                                }
                                None => {
                                    let varGlobTypeCheck = self.localTable.getType(&varName.clone());
                                    match varGlobTypeCheck{
                                        Some(varType) => {
                                            let compat = self.checkTypeCompatability(target.clone(), varType.clone());
                                            return compat;
                                        }
                                        None => {
                                            println!("Variable {} not defined", varName.clone());
                                            return false;
                                        }
                                    }
                                }
                            }
                            
                            
                        }
                        Expr::ArrayRef(name, index) => {
                            return true;
                        }
                        
                        //Operations
                        Expr::ArthOp(op1, op, op2) => {
                            return true;
                        }
                        Expr::LogOp(op1, op, op2) => {
                            return true;
                        }
                        Expr::RelOp(op1, op, op2) => {
                            return true;
                        }

                    }
                }
                VarType::Float => {
                    match new{
                        //Literals
                        Expr::IntLiteral(val) => {
                            return true;
                        }
                        Expr::FloatLiteral(val) => {
                            return true;
                        }
                        Expr::StringLiteral(val) => {
                            // println!("STRINGLITERAL {}", val.clone());
                            //For testing with putfloat, something weird happeining. TBD
                            if val == "floatval"{
                                return true;
                            } else{
                                return false;
                            }
                        }
                        Expr::BoolLiteral(val) => {
                            return false;
                        }
                        Expr::IntArrayLiteral(size, val) => {
                            return false;
                        }
                    
                        //References
                        Expr::VarRef(varName) => {
                            let varTypeLocCheck = self.localTable.getType(&varName.clone());
                            match varTypeLocCheck{
                                Some(varType) => {
                                    let compat = self.checkTypeCompatability(target.clone(), varType.clone());
                                    return compat;
                                }
                                None => {
                                    let varGlobTypeCheck = self.localTable.getType(&varName.clone());
                                    match varGlobTypeCheck{
                                        Some(varType) => {
                                            let compat = self.checkTypeCompatability(target.clone(), varType.clone());
                                            return compat;
                                        }
                                        None => {
                                            println!("Variable {} not defined", varName.clone());
                                            return false;
                                        }
                                    }
                                }
                            }
                            
                            
                        }
                        //References
                        Expr::ProcRef(varName, params) => {
                            
                            
                            
                            let procTypeLocCheck = self.localTable.getType(&varName.clone());
                            match procTypeLocCheck{
                                Some(varType) => {
                                    let compat = self.checkTypeCompatability(target.clone(), varType.clone());
                                    return compat;
                                }
                                None => {
                                    let varGlobTypeCheck = self.localTable.getType(&varName.clone());
                                    match varGlobTypeCheck{
                                        Some(varType) => {
                                            let compat = self.checkTypeCompatability(target.clone(), varType.clone());
                                            return compat;
                                        }
                                        None => {
                                            println!("Variable {} not defined", varName.clone());
                                            return false;
                                        }
                                    }
                                }
                            }
                            
                            
                        }
                        Expr::ArrayRef(name, index) => {
                            return true;
                        }
                        
                        //Operations
                        Expr::ArthOp(op1, op, op2) => {
                            return true;
                        }
                        Expr::LogOp(op1, op, op2) => {
                            return false;
                        }
                        Expr::RelOp(op1, op, op2) => {
                            return false;
                        }

                    }
                }
                VarType::Int => {
                    match new{
                        //Literals
                        Expr::IntLiteral(val) => {
                            return true;
                        }
                        Expr::FloatLiteral(val) => {
                            return true;
                        }
                        Expr::StringLiteral(val) => {
                            return false;
                        }
                        Expr::BoolLiteral(val) => {
                            return true;
                        }
                        Expr::IntArrayLiteral(size, val) => {
                            return false;
                        }
                    
                        //References
                        Expr::VarRef(varName) => {
                            let varTypeLocCheck = self.localTable.getType(&varName.clone());
                            match varTypeLocCheck{
                                Some(varType) => {
                                    let compat = self.checkTypeCompatability(target.clone(), varType.clone());
                                    return compat;
                                }
                                None => {
                                    let varGlobTypeCheck = self.globalTable.getType(&varName.clone());
                                    match varGlobTypeCheck{
                                        Some(varType) => {
                                            let compat = self.checkTypeCompatability(target.clone(), varType.clone());
                                            return compat;
                                        }
                                        None => {
                                            println!("Variable {} not defined", varName.clone());
                                            return false;
                                        }
                                    }
                                }
                            }
                            
                            
                        }
                        //References
                        Expr::ProcRef(varName, params) => {
                            
                            let procTypeLocCheck = self.localTable.getType(&varName.clone());
                            match procTypeLocCheck{
                                Some(varType) => {
                                    let compat = self.checkTypeCompatability(target.clone(), varType.clone());
                                    return compat;
                                }
                                None => {
                                    let varGlobTypeCheck = self.localTable.getType(&varName.clone());
                                    match varGlobTypeCheck{
                                        Some(varType) => {
                                            let compat = self.checkTypeCompatability(target.clone(), varType.clone());
                                            return compat;
                                        }
                                        None => {
                                            println!("Variable {} not defined", varName.clone());
                                            return false;
                                        }
                                    }
                                }
                            }
                            
                            
                        }
                        Expr::ArrayRef(name, index) => {
                            return true;
                        }
                        
                        //Operations
                        Expr::ArthOp(op1, op, op2) => {
                            return true;
                        }
                        Expr::LogOp(op1, op, op2) => {
                            return true;
                        }
                        Expr::RelOp(op1, op, op2) => {
                            return true;
                        }

                    }
                }
                VarType::IntArray(targetSizee) => {
                    match new{
                        //Literals
                        Expr::IntLiteral(val) => {
                            return false;
                        }
                        Expr::FloatLiteral(val) => {
                            return false;
                        }
                        Expr::StringLiteral(val) => {
                            return false;
                        }
                        Expr::BoolLiteral(val) => {
                            return false;
                        }
                        Expr::IntArrayLiteral(size, val) => {
                            if (targetSizee == size){
                                return true;
                            } else {
                                return false
                            }
                        }
                    
                        //References
                        Expr::VarRef(varName) => {
                            let varTypeLocCheck = self.localTable.getType(&varName.clone());
                            match varTypeLocCheck{
                                Some(varType) => {
                                    let compat = self.checkTypeCompatability(target.clone(), varType.clone());
                                    return compat;
                                }
                                None => {
                                    let varGlobTypeCheck = self.localTable.getType(&varName.clone());
                                    match varGlobTypeCheck{
                                        Some(varType) => {
                                            let compat = self.checkTypeCompatability(target.clone(), varType.clone());
                                            return compat;
                                        }
                                        None => {
                                            println!("Variable {} not defined", varName.clone());
                                            return false;
                                        }
                                    }
                                }
                            }
                            
                            
                        }
                        //References
                        Expr::ProcRef(varName, params) => {
                            
                            
                            
                            let procTypeLocCheck = self.localTable.getType(&varName.clone());
                            match procTypeLocCheck{
                                Some(varType) => {
                                    let compat = self.checkTypeCompatability(target.clone(), varType.clone());
                                    return compat;
                                }
                                None => {
                                    let varGlobTypeCheck = self.localTable.getType(&varName.clone());
                                    match varGlobTypeCheck{
                                        Some(varType) => {
                                            let compat = self.checkTypeCompatability(target.clone(), varType.clone());
                                            return compat;
                                        }
                                        None => {
                                            println!("Variable {} not defined", varName.clone());
                                            return false;
                                        }
                                    }
                                }
                            }
                            
                            
                        }
                        Expr::ArrayRef(name, index) => {
                            return false;
                        }
                        
                        //Operations
                        Expr::ArthOp(op1, op, op2) => {
                            return false;
                        }
                        Expr::LogOp(op1, op, op2) => {
                            return false;
                        }
                        Expr::RelOp(op1, op, op2) => {
                            return false;
                        }

                    }
                }
                VarType::Str => {
                    match new{
                        //Literals
                        Expr::IntLiteral(val) => {
                            return false;
                        }
                        Expr::FloatLiteral(val) => {
                            return false;
                        }
                        Expr::StringLiteral(val) => {
                            return true;
                        }
                        Expr::BoolLiteral(val) => {
                            return false;
                        }
                        Expr::IntArrayLiteral(size, val) => {
                            return false;
                        }
                    
                        //References
                        Expr::VarRef(varName) => {
                            let varTypeLocCheck = self.localTable.getType(&varName.clone());
                            match varTypeLocCheck{
                                Some(varType) => {
                                    let compat = self.checkTypeCompatability(target.clone(), varType.clone());
                                    return compat;
                                }
                                None => {
                                    let varGlobTypeCheck = self.localTable.getType(&varName.clone());
                                    match varGlobTypeCheck{
                                        Some(varType) => {
                                            let compat = self.checkTypeCompatability(target.clone(), varType.clone());
                                            return compat;
                                        }
                                        None => {
                                            println!("Variable {} not defined", varName.clone());
                                            return false;
                                        }
                                    }
                                }
                            }
                            
                            
                        }
                        //References
                        Expr::ProcRef(varName, params) => {
                            
                            
                            
                            let procTypeLocCheck = self.localTable.getType(&varName.clone());
                            match procTypeLocCheck{
                                Some(varType) => {
                                    let compat = self.checkTypeCompatability(target.clone(), varType.clone());
                                    return compat;
                                }
                                None => {
                                    let varGlobTypeCheck = self.localTable.getType(&varName.clone());
                                    match varGlobTypeCheck{
                                        Some(varType) => {
                                            let compat = self.checkTypeCompatability(target.clone(), varType.clone());
                                            return compat;
                                        }
                                        None => {
                                            println!("Variable {} not defined", varName.clone());
                                            return false;
                                        }
                                    }
                                }
                            }
                            
                            
                        }
                        Expr::ArrayRef(name, index) => {
                            return false;
                        }
                        
                        //Operations
                        Expr::ArthOp(op1, op, op2) => {
                            return false;
                        }
                        Expr::LogOp(op1, op, op2) => {
                            return false;
                        }
                        Expr::RelOp(op1, op, op2) => {
                            return false;
                        }

                    }
                }
            }
        } else {
            return false;
        }
    }


    //This is used to check the type of a variable or procedure in first the local then global scope
    //Returns the type if found
    fn checkVar(&mut self, varName: String) -> Option<VarType> {
        //Checks the local scope first
        let checkedLoc = self.localTable.getType(&varName.clone());
        match checkedLoc{
            Some(var) => {
                //this means the variable exists locally
                return Some(var);
            }
            None => {
                //this means it was not found locally, checking globally
                let checkedGlob = self.globalTable.getType(&varName.clone());
                match checkedGlob{
                    Some(var) => {
                        //it was found globally
                        return Some(var);
                    }
                    None => {
                        //This means it was not found globally or locally
                        return None;
                    }
                }
            }
        }
    }

    pub fn checkExpr(&mut self, mut checkExpr: Expr) -> bool{
        match checkExpr.clone(){
            //Literals
            Expr::IntLiteral(val) => {
                return true;
            }
            Expr::FloatLiteral(val) => {
                return true;
            }
            Expr::StringLiteral(val) => {
                return true;
            }
            Expr::BoolLiteral(val) => {
                return true;
            }
            Expr::IntArrayLiteral(size, array) => {
                return true;
            }
            
            //References
            Expr::VarRef(varName) => {
                //Gets the type if defined in local scope
                let checkLocVar = self.localTable.get(&varName.clone());
                match checkLocVar{
                    Some(var) => {
                        if var.hashType != HashItemType::Variable {
                            println!("{} is not a variable", varName.clone());
                            return false;
                        } else {
                            return true;
                        }
                    }
                    None => {
                        let checkGlobVar = self.globalTable.get(&varName.clone());
                            match checkGlobVar{
                                Some(var) => {
                                    if var.hashType != HashItemType::Variable {
                                        println!("{} is not a variable", varName.clone());
                                        return false;
                                    } else {
                                        return true;
                                    }
                                }
                                None => {
                                    println!("Variable {} is not defined", varName.clone());
                                    return false;
                                }
                            }
                    }
                }
                
            }
            Expr::ProcRef(procName, params) => {
                if (self.checked.clone() == false) & (self.name.clone() == procName.clone()){
                    return true;
                } else {
                    //Gets the type if defined in local scope
                    let checkProc = self.localTable.get(&procName.clone());
                    match checkProc.clone(){
                        Some(proc) => {
                            if let HashItemType::Procedure(procAst, procParamList, mut procSt) = proc.hashType.clone() {
                                //Proc found, need to check params now
                                match params.clone(){
                                    Some(paramsVec) => {
                                        if (procParamList.len() == paramsVec.len()) {
                                            //the numbers are correct at least
                                            let mut i = 0;
                                            //Checks all of the params
                                            for param in paramsVec.clone() {
                                                let targetTypeCheck = procSt.getType(&procParamList[i].clone());
                                                match targetTypeCheck{
                                                    Some(targetType) => {
                                                        let compatable = self.checkExprTypeCompatability(targetType.clone(), param.clone());
                                                        if compatable {
                                                            //Continue to checking next param
                                                        } else {
                                                            println!("Error with call to procedure {}: param {} is type {}, which is incompatible with given type {}", procName.clone(), procParamList[i].clone(), targetType.clone(), param.clone());
                                                            return false;
                                                        }
                                                    }
                                                    None => {
                                                        println!("Some sort of error with the procedure symbol table. Could not located defined parameter in table");
                                                        return false;
                                                    }
                                                }
                                                i += 1;
                                            }
                                            return true;

                                        } else {
                                            println!("Error with call to procedure {}: {} params required, {} provided", procName.clone(), paramsVec.len().to_string(), procParamList.len().clone().to_string())
                                        }
                                    }
                                    None => {
                                        if (procParamList.len() == 0){
                                            return true;
                                        } else {
                                            println!("Procedure call to {} missing parameters", procName.clone());
                                            return false;
                                        }
                                    }
                                }
                                return true;
                            } else {
                                println!("{} is not defined as a procedure", procName.clone());
                                return false;
                            }
                        }
                        None => {
                            //CHECK IN GLOBAL SCOPE GOES HERE
                            //Gets the type if defined in local scope
                            let checkGlobProc = self.globalTable.get(&procName.clone());
                            match checkGlobProc.clone(){
                                Some(proc) => {
                                    if let HashItemType::Procedure(procAst, procParamList, mut procSt) = proc.hashType.clone() {
                                        //Proc found, need to check params now
                                        match params.clone(){
                                            Some(paramsVec) => {
                                                if (procParamList.len() == paramsVec.len()) {
                                                    //the numbers are correct at least
                                                    let mut i = 0;
                                                    //Checks all of the params
                                                    for param in paramsVec.clone() {
                                                        let targetTypeCheck = procSt.getType(&procParamList[i].clone());
                                                        match targetTypeCheck{
                                                            Some(targetType) => {
                                                                let compatable = self.checkExprTypeCompatability(targetType.clone(), param.clone());
                                                                if compatable {
                                                                    //Continue to checking next param
                                                                } else {
                                                                    println!("Error with call to procedure {}: param {} is type {}, which is incompatible with given type {}", procName.clone(), procParamList[i].clone(), targetType.clone(), param.clone());
                                                                    return false;
                                                                }
                                                            }
                                                            None => {
                                                                println!("Some sort of error with the procedure symbol table. Could not located defined parameter in table");
                                                                return false;
                                                            }
                                                        }
                                                        i += 1;
                                                    }
                                                    return true;

                                                } else {
                                                    println!("Error with call to procedure {}: {} params required, {} provided", procName.clone(), paramsVec.len().to_string(), procParamList.len().clone().to_string());
                                                    return false;
                                                }
                                            }
                                            None => {
                                                if (procParamList.len() == 0){
                                                    return true;
                                                } else {
                                                    println!("Procedure call to {} missing parameters", procName.clone());
                                                    return false;
                                                }
                                            }
                                        }
                                    } else {
                                        println!("{} is not defined as a procedure", procName.clone());
                                        return false;
                                    }
                                }
                                None => {
                                    println!("Procedure {} is not defined", procName.clone());
                                    return false;
                                }
                            }
                        }
                    }
                }
                
            }
            Expr::ArrayRef(varName, indexExpr) => {
                let existVar: VarType;
                let checkLocVar = self.localTable.get(&varName.clone());
                match checkLocVar{
                    Some(var) => {
                        if var.hashType != HashItemType::Variable {
                            println!("{} is not a variable", varName.clone());
                            return false;
                        } else {
                            existVar = var.clone().getType().clone();
                        }
                    }
                    None => {
                        let checkGlobVar = self.globalTable.get(&varName.clone());
                            match checkGlobVar{
                                Some(var) => {
                                    if var.hashType != HashItemType::Variable {
                                        println!("{} is not a variable", varName.clone());
                                        return false;
                                    } else {
                                        existVar = var.clone().getType().clone();
                                    }
                                }
                                None => {
                                    println!("Variable {} is not defined", varName.clone());
                                    return false;
                                }
                            }
                    }
                }
                
                match existVar{
                    VarType::IntArray(size) => {
                        let checkedExpr =  self.checkExpr(*indexExpr);
                        if checkedExpr {
                            return true;
                        }
                        else {
                            println!("Error with array index");
                            return false;
                        }
                    }
                    _ => {
                        println!("Variable {} is not an array", varName.clone());
                        return false;
                    }
                }                
            }
            
            //Operations
            Expr::ArthOp(op1, op, op2) => {
                //First checks operand 1 to ensure it is valid
                let checkedOp1 = self.checkExpr(*op1.clone());
                if !checkedOp1 {
                    println!("Error in operand one of arithmetic operation");
                    return false;
                }
                //Checks operand 2
                let checkedOp2 = self.checkExpr(*op2.clone());
                if !checkedOp2{
                    println!("Error in operand two of arithmetic operation");
                    return false;
                }

                //Since both are good, need to ensure both are compatabile with ArthOps
                match *op1 {
                    Expr::IntLiteral(val) => {
                        //continue
                    }
                    Expr::FloatLiteral(val) => {
                        //continue
                    }
                    Expr::StringLiteral(val) => {
                        println!("Cannot use string in arithmetic operation");
                        return false;
                    }
                    Expr::BoolLiteral(val) => {
                        println!("Cannot use boolean as operand in arithmetic operation");
                        return false;
                    }
                    Expr::IntArrayLiteral(size, val) => {
                        println!("Cannot use entire array in arithmetic operation");
                        return false;
                    }
                    Expr::VarRef(varName) => {
                        let mut op1Type: VarType;
                        let op1TypeCheck = self.checkVar(varName.clone());
                        match op1TypeCheck{
                            Some(foundType) => {
                                op1Type = foundType;
                            }
                            None => {
                                println!("Referenced to undefined {}", varName.clone());
                                return false;
                            }
                        }
                    
                        //Now we have to check if the type is compatible with the arthop
                        match op1Type{
                            VarType::Float => {
                                //continue
                            }
                            VarType::Int => {
                                //continue
                            }
                            VarType::IntArray(size) => {
                                //continue
                            }
                            _ => {
                                println!("Cannot use variable {} of type {} in arithmetic operation", varName.clone(), op1Type.clone());
                                return false;
                            }
                        }
                    
                    }
                    Expr::ProcRef(procName, params) => {
                        if (self.checked.clone() == false) & (self.name.clone() == procName.clone()){
                            return true;
                        } else {    let mut op1Type: VarType;
                            let op1TypeCheck = self.checkVar(procName.clone());
                            match op1TypeCheck{
                                Some(foundType) => {
                                    op1Type = foundType;
                                }
                                None => {
                                    println!("Referenced to undefined {}", procName.clone());
                                    return false;
                                }
                            }
                        
                            //Now we have to check if the type is compatible with the arthop
                            match op1Type{
                                VarType::Float => {
                                    //continue
                                }
                                VarType::Int => {
                                    //continue
                                }
                                _ => {
                                    println!("Cannot use procedure {} of type {} in arithmetic operation", procName.clone(), op1Type.clone());
                                    return false;
                                }
                            }
                        }
                    }
                    Expr::ArrayRef(varName, indexExpr) => {
                        //continue
                    }
                    Expr::ArthOp(operand1, op, operand2) => {
                        //continue
                    }
                    Expr::LogOp(operand1, oeprator, operand2) => {
                        println!("Cannot use a logical operation as an operand in arithmetic operation");
                        return false;
                    }
                    Expr::RelOp(operand1, operator, operand2) => {
                        println!("Cannot use a relational operation as an operand in arithmetic operation");
                        return false;
                    }
                }

                //Checks the compatability of operand 2
                match *op2 {
                    Expr::IntLiteral(val) => {
                        //continue
                    }
                    Expr::FloatLiteral(val) => {
                        //continue
                    }
                    Expr::StringLiteral(val) => {
                        println!("Cannot use string in arithmetic operation");
                        return false;
                    }
                    Expr::BoolLiteral(val) => {
                        println!("Cannot use boolean as operand in arithmetic operation");
                        return false;
                    }
                    Expr::IntArrayLiteral(size, val) => {
                        println!("Cannot use entire array in arithmetic operation");
                        return false;
                    }
                    Expr::VarRef(varName) => {
                        let mut op1Type: VarType;
                        let op1TypeCheck = self.checkVar(varName.clone());
                        match op1TypeCheck{
                            Some(foundType) => {
                                op1Type = foundType;
                            }
                            None => {
                                println!("Referenced to undefined {}", varName.clone());
                                return false;
                            }
                        }
                    
                        //Now we have to check if the type is compatible with the arthop
                        match op1Type{
                            VarType::Float => {
                                //continue
                            }
                            VarType::Int => {
                                //continue
                            }
                            VarType::IntArray(size) => {
                                //continue
                            }
                            _ => {
                                println!("Cannot use variable {} of type {} in arithmetic operation", varName.clone(), op1Type.clone());
                                return false;
                            }
                        }
                    
                    }
                    Expr::ProcRef(procName, params) => {
                        if (self.checked.clone() == false) & (self.name.clone() == procName.clone()){
                            return true;
                        } else {
                            let mut op1Type: VarType;
                            let op1TypeCheck = self.checkVar(procName.clone());
                            match op1TypeCheck{
                                Some(foundType) => {
                                    op1Type = foundType;
                                }
                                None => {
                                    println!("Referenced to undefined {}", procName.clone());
                                    return false;
                                }
                            }
                        
                            //Now we have to check if the type is compatible with the arthop
                            match op1Type{
                                VarType::Float => {
                                    //continue
                                }
                                VarType::Int => {
                                    //continue
                                }
                                _ => {
                                    println!("Cannot use procedure {} of type {} in arithmetic operation", procName.clone(), op1Type.clone());
                                    return false;
                                }
                            }
                        }
                    }
                    Expr::ArrayRef(varName, indexExpr) => {
                        //continue
                    }
                    Expr::ArthOp(operand1, op, operand2) => {
                        //continue
                    }
                    Expr::LogOp(operand1, oeprator, operand2) => {
                        println!("Cannot use a logical operation as an operand in arithmetic operation");
                        return false;
                    }
                    Expr::RelOp(operand1, operator, operand2) => {
                        println!("Cannot use a relational operation as an operand in arithmetic operation");
                        return false;
                    }
                }


                //Now that we are here and everything has been checked, we are good
                return true;
            }
            
            Expr::LogOp(op1, op, op2) => {
                //First checks operand 1 to ensure it is valid
                let checkedOp1 = self.checkExpr(*op1.clone());
                if !checkedOp1 {
                    println!("Error in operand one of arithmetic operation");
                    return false;
                }
                //Checks operand 2
                let checkedOp2 = self.checkExpr(*op2.clone());
                if !checkedOp2{
                    println!("Error in operand two of arithmetic operation");
                    return false;
                }

                //Since both are good, need to ensure both are compatabile with ArthOps
                match *op1 {
                    Expr::IntLiteral(val) => {
                        //continue
                    }
                    Expr::FloatLiteral(val) => {
                        println!("Cannot use float as operand in logical operation");
                        return false;
                    }
                    Expr::StringLiteral(val) => {
                        println!("Cannot use string as operand in arithmetic operation");
                        return false;
                    }
                    Expr::BoolLiteral(val) => {
                        println!("Cannot use string as operand in arithmetic operation");
                        return false;
                    }
                    Expr::IntArrayLiteral(size, val) => {
                        println!("Cannot use entire array as operand in logical operation");
                        return false;
                    }
                    Expr::VarRef(varName) => {
                        let mut op1Type: VarType;
                        let op1TypeCheck = self.checkVar(varName.clone());
                        match op1TypeCheck{
                            Some(foundType) => {
                                op1Type = foundType;
                            }
                            None => {
                                println!("Referenced to undefined {}", varName.clone());
                                return false;
                            }
                        }
                    
                        //Now we have to check if the type is compatible with the arthop
                        match op1Type{
                            VarType::Int => {
                                //continue
                            }
                            VarType::IntArray(size) => {
                                //continue
                            }
                            _ => {
                                println!("Cannot use variable {} of type {} in logical operation", varName.clone(), op1Type.clone());
                                return false;
                            }
                        }
                    
                    }
                    Expr::ProcRef(procName, params) => {
                        let mut op1Type: VarType;
                        let op1TypeCheck = self.checkVar(procName.clone());
                        match op1TypeCheck{
                            Some(foundType) => {
                                op1Type = foundType;
                            }
                            None => {
                                println!("Referenced to undefined {}", procName.clone());
                                return false;
                            }
                        }
                    
                        //Now we have to check if the type is compatible with the arthop
                        match op1Type{
                            VarType::Int => {
                                //continue
                            }
                            _ => {
                                println!("Cannot use procedure {} of type {} in logical operation", procName.clone(), op1Type.clone());
                                return false;
                            }
                        }
                    }
                    Expr::ArrayRef(varName, indexExpr) => {
                        //continue
                    }
                    Expr::ArthOp(operand1, op, operand2) => {
                        //continue
                    }
                    Expr::LogOp(operand1, oeprator, operand2) => {
                        println!("Cannot use a logical operation as an operand in logical operation");
                        return false;
                    }
                    Expr::RelOp(operand1, operator, operand2) => {
                        println!("Cannot use a relational operation as an operand in logical operation");
                        return false;
                    }
               }


                //Checks the compatability of operand 2
                match *op2 {
                    Expr::IntLiteral(val) => {
                        //continue
                    }
                    Expr::FloatLiteral(val) => {
                        println!("Cannot use float as operand in logical operation");
                        return false;
                    }
                    Expr::StringLiteral(val) => {
                        println!("Cannot use string as operand in arithmetic operation");
                        return false;
                    }
                    Expr::BoolLiteral(val) => {
                        println!("Cannot use string as operand in arithmetic operation");
                        return false;
                    }
                    Expr::IntArrayLiteral(size, val) => {
                        println!("Cannot use entire array as operand in logical operation");
                        return false;
                    }
                    Expr::VarRef(varName) => {
                        let mut op1Type: VarType;
                        let op1TypeCheck = self.checkVar(varName.clone());
                        match op1TypeCheck{
                            Some(foundType) => {
                                op1Type = foundType;
                            }
                            None => {
                                println!("Referenced to undefined {}", varName.clone());
                                return false;
                            }
                        }
                    
                        //Now we have to check if the type is compatible with the arthop
                        match op1Type{
                            VarType::Int => {
                                //continue
                            }
                            VarType::IntArray(size) => {
                                //continue
                            }
                            _ => {
                                println!("Cannot use variable {} of type {} in logical operation", varName.clone(), op1Type.clone());
                                return false;
                            }
                        }
                    
                    }
                    Expr::ProcRef(procName, params) => {
                        let mut op1Type: VarType;
                        let op1TypeCheck = self.checkVar(procName.clone());
                        match op1TypeCheck{
                            Some(foundType) => {
                                op1Type = foundType;
                            }
                            None => {
                                println!("Referenced to undefined {}", procName.clone());
                                return false;
                            }
                        }
                    
                        //Now we have to check if the type is compatible with the arthop
                        match op1Type{
                            VarType::Int => {
                                //continue
                            }
                            _ => {
                                println!("Cannot use procedure {} of type {} in logical operation", procName.clone(), op1Type.clone());
                                return false;
                            }
                        }
                    }
                    Expr::ArrayRef(varName, indexExpr) => {
                        //continue
                    }
                    Expr::ArthOp(operand1, op, operand2) => {
                        //continue
                    }
                    Expr::LogOp(operand1, oeprator, operand2) => {
                        println!("Cannot use a logical operation as an operand in logical operation");
                        return false;
                    }
                    Expr::RelOp(operand1, operator, operand2) => {
                        println!("Cannot use a relational operation as an operand in logical operation");
                        return false;
                    }
                }

                //Now that we are here and everything has been checked, we are good
                return true;
            }
            Expr::RelOp(op1, op, op2) => {
                //First checks operand 1 to ensure it is valid
                let checkedOp1 = self.checkExpr(*op1.clone());
                if !checkedOp1 {
                    println!("Error in operand one of relational operation");
                    return false;
                }
                //Checks operand 2
                let checkedOp2 = self.checkExpr(*op2.clone());
                if !checkedOp2{
                    println!("Error in operand two of relational operation");
                    return false;
                }

                //Since both are good, need to ensure both are compatabile with ArthOps
                match *op1 {
                    Expr::IntLiteral(val) => {
                        //continue
                    }
                    Expr::FloatLiteral(val) => {
                        //continue
                    }
                    Expr::StringLiteral(val) => {
                        println!("Cannot use string as operand in relational operation");
                        return false;
                    }
                    Expr::BoolLiteral(val) => {
                        //continue
                    }
                    Expr::IntArrayLiteral(size, val) => {
                        println!("Cannot use entire array as operand in logical operation");
                        return false;
                    }
                    Expr::VarRef(varName) => {
                        let mut op1Type: VarType;
                        let op1TypeCheck = self.checkVar(varName.clone());
                        match op1TypeCheck{
                            Some(foundType) => {
                                op1Type = foundType;
                            }
                            None => {
                                println!("Referenced to undefined {}", varName.clone());
                                return false;
                            }
                        }
                    
                        //Now we have to check if the type is compatible with the arthop
                        match op1Type{
                            VarType::Int => {
                                //continue
                            }
                            VarType::Float => {
                                //continue
                            }
                            VarType::Bool => {
                                //continue
                            }
                            _ => {
                                println!("Cannot use variable {} of type {} in relational operation", varName.clone(), op1Type.clone());
                                return false;
                            }
                        }
                    
                    }
                    Expr::ProcRef(procName, params) => {
                        let mut op1Type: VarType;
                        let op1TypeCheck = self.checkVar(procName.clone());
                        match op1TypeCheck{
                            Some(foundType) => {
                                op1Type = foundType;
                            }
                            None => {
                                println!("Referenced to undefined {}", procName.clone());
                                return false;
                            }
                        }
                    
                        //Now we have to check if the type is compatible with the arthop
                        match op1Type{
                            VarType::Int => {
                                //continue
                            }
                            VarType::Float => {
                                //continue
                            }
                            VarType::Bool => {
                                //continue
                            }
                            _ => {
                                println!("Cannot use procedure {} of type {} in relational operation", procName.clone(), op1Type.clone());
                                return false;
                            }
                        }
                    }
                    Expr::ArrayRef(varName, indexExpr) => {
                        //continue
                    }
                    Expr::ArthOp(operand1, op, operand2) => {
                        //continue
                    }
                    Expr::LogOp(operand1, oeprator, operand2) => {
                        //continue
                    }
                    Expr::RelOp(operand1, operator, operand2) => {
                        //continue
                    }
               }


               //Checks the compatability of operand 2
                match *op2 {
                    Expr::IntLiteral(val) => {
                        //continue
                    }
                    Expr::FloatLiteral(val) => {
                        //continue
                    }
                    Expr::StringLiteral(val) => {
                        println!("Cannot use string as operand in relational operation");
                        return false;
                    }
                    Expr::BoolLiteral(val) => {
                        //continue
                    }
                    Expr::IntArrayLiteral(size, val) => {
                        println!("Cannot use entire array as operand in logical operation");
                        return false;
                    }
                    Expr::VarRef(varName) => {
                        let mut op1Type: VarType;
                        let op1TypeCheck = self.checkVar(varName.clone());
                        match op1TypeCheck{
                            Some(foundType) => {
                                op1Type = foundType;
                            }
                            None => {
                                println!("Referenced to undefined {}", varName.clone());
                                return false;
                            }
                        }
                    
                        //Now we have to check if the type is compatible with the arthop
                        match op1Type{
                            VarType::Int => {
                                //continue
                            }
                            VarType::Float => {
                                //continue
                            }
                            VarType::Bool => {
                                //continue
                            }
                            _ => {
                                println!("Cannot use variable {} of type {} in relational operation", varName.clone(), op1Type.clone());
                                return false;
                            }
                        }
                    
                    }
                    Expr::ProcRef(procName, params) => {
                        let mut op1Type: VarType;
                        let op1TypeCheck = self.checkVar(procName.clone());
                        match op1TypeCheck{
                            Some(foundType) => {
                                op1Type = foundType;
                            }
                            None => {
                                println!("Referenced to undefined {}", procName.clone());
                                return false;
                            }
                        }
                    
                        //Now we have to check if the type is compatible with the arthop
                        match op1Type{
                            VarType::Int => {
                                //continue
                            }
                            VarType::Float => {
                                //continue
                            }
                            VarType::Bool => {
                                //continue
                            }
                            _ => {
                                println!("Cannot use procedure {} of type {} in relational operation", procName.clone(), op1Type.clone());
                                return false;
                            }
                        }
                    }
                    Expr::ArrayRef(varName, indexExpr) => {
                        //continue
                    }
                    Expr::ArthOp(operand1, op, operand2) => {
                        //continue
                    }
                    Expr::LogOp(operand1, oeprator, operand2) => {
                        //continue
                    }
                    Expr::RelOp(operand1, operator, operand2) => {
                        //continue
                    }
                }

                //Now that we are here and everything has been checked, we are good
                return true;
            }
        }
    }
    
    //Checks each statement one at a time, returns a bool if there's an error
    pub fn checkStmt(&mut self, mut checkStmt: Stmt) -> bool{
        match (checkStmt){
            //For checking and declaring local variables
            Stmt::VarDecl(varName, varType, lineNum) => {
                if self.scope != 0 {
                    let defined = self.localTable.checkItem(&varName.clone());
                    if(defined){
                        println!("Error: variable: {} defined twice", varName.clone());
                        return false;
                    } else {
                        let item = HashItem::newVar(varName.clone(), varType.clone());
                        self.localTable.symTab.insert(varName.clone(), item.clone());
                        return true;
                    }
                } else {
                    let defined = self.globalTable.checkItem(&varName.clone());
                    if(defined){
                        println!("Error: variable: {} defined twice", varName.clone());
                        return false;
                    } else {
                        let item = HashItem::newVar(varName.clone(), varType.clone());
                        self.globalTable.symTab.insert(varName.clone(), item.clone());
                        return true;
                    }
                }
                
                
            }
            //For checking and declaring global variables
            Stmt::GlobVarDecl(varName, varType, lineNum) => {

                let defined = self.globalTable.checkItem(&varName.clone());
                if(defined){
                    println!("Error: variable: {} defined twice", varName.clone());
                    return false;
                } else {
                    let item = HashItem::newVar(varName.clone(), varType.clone());
                    self.globalTable.symTab.insert(varName.clone(), item.clone());
                    return true;
                }
                    
            }
            //For checking a procedure
            Stmt::ProcDecl(retType, procName, params, header, body, lineNum) => {
                // println!("procedure declaration");
                let procAst = Stmt::Program(procName.clone(), header.clone(), body.clone(), lineNum.clone());
                
                let mut paramStrings: Vec<String> = Vec::new();

                let curScope = self.scope.clone();

                let mut procChecker: SyntaxChecker = self.newScope(procAst, curScope, procName.clone());
                //Iterates through the parameters, registering them in the Symboltable and copying the names to the list of params
                if let Stmt::Block(ref instrs, lineNum) = *params.clone() {
                    for instr in instrs {
                        let good = procChecker.checkStmt(instr.clone());
                        if (!good){
                            println!("Error in Procedure parameter definition on line {}:", lineNum.clone());
                            return false;
                        } else {
                            match instr.clone(){
                                Stmt::VarDecl(varName, VarType, lineNum) => {
                                    paramStrings.push(varName.clone());
                                }
                                _ => {
                                    println!("Error with procedure {} declaration on line {}:\n Procedure parameters must be variable declarations in the following format:\n    variable <identifier> : <type_mark>", procName.clone(), lineNum.clone());
                                    return false;
                                }
                            }
                        }
                    }
                } else {
                    println!("Error in Procedure parameter definition on line {}:", lineNum.clone());
                    // instr.display(0);
                    return false;
                }


                //Checks the procedure to make sure its all good
                let procGood = procChecker.checkProgram();


                //If the procedure is good, appends to the symboltable and moved on
                if(!procGood){
                    println!("Error in procedure {} defined on line {}", procName.clone(), lineNum.clone());
                    return false;
                } else {
                    if curScope != 0 {
                        //Sets up the things and inserts the procedure into the symboltable
                        let mut procItemType = HashItemType::newProcItem(body.clone(), paramStrings.clone(), procChecker.localTable.clone());
                        let mut procItem: HashItem = HashItem::newProc(procName.clone(), retType.clone(), procItemType);
                        self.localTable.symTab.insert(procName.clone(), procItem.clone());
                        
                        return true;
                    } else {
                        //Sets up the things and inserts the procedure into the symboltable
                        let mut procItemType = HashItemType::newProcItem(body.clone(), paramStrings.clone(), procChecker.localTable.clone());
                        let mut procItem: HashItem = HashItem::newProc(procName.clone(), retType.clone(), procItemType);
                        self.globalTable.symTab.insert(procName.clone(), procItem.clone());
                        return true;
                    }
                }
            }
            //For checking a variable assignment
            Stmt::Assign(valueToAssign, newValue, lineNum) => {
                if let Expr::VarRef(ref targName) = valueToAssign {
                    //Check if variable assignment is in the local table
                    let mut targValue: HashItem; 
                    //Looks for the value in the local then global table, retrieves it if so
                    if !(self.localTable.checkItem(targName)){
                        if !(self.globalTable.checkItem(targName)){
                            println!("Attempting to assign value to undeclared variable: {} on line: {}", targName.clone(), lineNum.clone());
                            return false;
                        } else {
                            let gotValue = self.globalTable.get(targName);
                            match gotValue{
                                Some(val) => {
                                    targValue = val.clone();
                                }
                                None => {
                                    println!("Error with value {} on line: {}", targName.clone(), lineNum.clone());
                                    return false;
                                }
                            }
                        }
                    } else {
                        let gotValue = self.localTable.get(targName);
                        match gotValue{
                            Some(val) => {
                                targValue = val.clone();
                            }
                            None => {
                                println!("Error with value {} on line: {}", targName.clone(), lineNum.clone());
                                return false;
                            }
                        }
                    }
                    
                    //Checks if value being assigned to is a variable
                    if targValue.hashType != HashItemType::Variable {
                        println!("On line: {}, cannot assign value to procedure", lineNum.clone());
                        return false;
                    }
                    
                    //Checks to ensure that new value matches target value
                    let targType = targValue.getType();
                    match targType{
                        VarType::Int => {
                            match newValue.clone(){
                                //Literals
                                Expr::IntLiteral(val) => {
                                    return true;
                                }
                                Expr::FloatLiteral(val) => {
                                    return true;
                                }
                                Expr::ArrayRef(name, index) => {
                                    let checked = self.checkExpr(newValue.clone());
                                    if checked {
                                        return true;
                                    } else {
                                        println!("Error with array reference on line {}", lineNum.clone());
                                        return false;
                                    }
                                }
                                Expr::BoolLiteral(val) => {
                                    return true;
                                }
                                Expr::StringLiteral(val) => {
                                    println!("Error on line {}:\n Cannot assign string to variable of type int", lineNum.clone());
                                    return false;
                                }
                                Expr::IntArrayLiteral(size, array) => {
                                    println!("Cannot assign array to variable of type {}", targType.clone());
                                    return false;
                                }

                                //Operations
                                Expr::ArthOp(op1, op, op2) => {
                                    let checked = self.checkExpr(newValue.clone());
                                    if(checked){
                                        return true;
                                    } else {
                                        println!("Error in arithmetic operation on line {}", lineNum.clone());
                                        return false;
                                    }
                                }
                                Expr::LogOp(op1, op, op2) => {
                                    let checked = self.checkExpr(newValue.clone());
                                    if(checked){
                                        return true;
                                    } else {
                                        println!("Error in logical operation on line {}", lineNum.clone());
                                        return false;
                                    }
                                }
                                Expr::RelOp(op1, op, op2) => {
                                    let checked = self.checkExpr(newValue.clone());
                                    if(checked){
                                        return true;
                                    } else {
                                        println!("Error in relational operation on line {}", lineNum.clone());
                                        return false;
                                    }
                                }
                                
                                //Calls/references
                                Expr::ProcRef(procName, params) => {
                                    if (self.checked.clone() == false) & (self.name.clone() == procName.clone()){
                                        return true;
                                    } else {    
                                        let mut procType: VarType;
                                        //Checks if procedure is defined
                                        let checkLocProc = self.localTable.getType(&procName.clone());
                                        match checkLocProc{
                                            Some(proc) => {
                                                procType = proc;
                                            }
                                            None => {
                                                let checkGlobProc = self.globalTable.getType(&procName.clone());
                                                match checkGlobProc{
                                                    Some(proc) => {
                                                        procType = proc
                                                    }
                                                    None => {
                                                        println!("Error on line {}:\n Procedure {} is not defined", lineNum.clone(), procName.clone());
                                                        return false;
                                                        
                                                    }
                                                }
                                            }
                                        }
                                    
                                        //Checks procedure type compatability with int
                                        match procType{
                                            VarType::Bool =>{
                                                return true;
                                            }
                                            VarType::Int =>{
                                                return true;
                                            }
                                            VarType::Float =>{
                                                return true;
                                            }
                                            _ => {
                                                println!("Error on line {}:\n Cannot assign {} to variable {} of type {}", lineNum.clone(), procType.clone(), targName.clone(), targType.clone());
                                                return false;
                                            }
                                        }}
                                }   
                                Expr::VarRef(assignName) => {
                                    let mut assignType: VarType;
                                    //Checks if variable is defined
                                    let checkLocVar = self.localTable.getType(&assignName.clone());
                                    match checkLocVar{
                                        Some(var) => {
                                            assignType = var;
                                        }
                                        None => {
                                            let checkGlobVar = self.globalTable.getType(&assignName.clone());
                                            match checkGlobVar{
                                                Some(var) => {
                                                    assignType = var
                                                }
                                                None => {
                                                    println!("Error on line {}:\n Variable {} is not defined", lineNum.clone(), assignName.clone());
                                                    return false;
                                                    
                                                }
                                            }
                                        }
                                    }
                                
                                    //Checks variable type compatability with int
                                    match assignType{
                                        VarType::Bool =>{
                                            return true;
                                        }
                                        VarType::Int =>{
                                            return true;
                                        }
                                        VarType::Float =>{
                                            return true;
                                        }
                                        VarType::IntArray(size) => {
                                            return true;
                                        }
                                        _ => {
                                            println!("Error on line {}:\n Cannot assign {} to variable {} of type {}", lineNum.clone(), assignType.clone(), targName.clone(), targType.clone());
                                            return false;
                                        }
                                    }
                                }
                            }
                        }
                        VarType::Bool => {
                            match newValue.clone(){
                                //Literals
                                Expr::IntLiteral(val) => {
                                    return true;
                                }
                                Expr::FloatLiteral(val) => {
                                    println!("Error on line {}:\n Cannot assign float to variable of type bool", lineNum.clone());
                                    return false;
                                }
                                Expr::ArrayRef(name, index) => {
                                    println!("Assinging: Int Array ref");
                                    let checked = self.checkExpr(newValue.clone());
                                    if checked {
                                        return true;
                                    } else {
                                        println!("Error with array reference on line {}", lineNum.clone());
                                        return false;
                                    }
                                }Expr::BoolLiteral(val) => {
                                    return true;
                                }
                                Expr::StringLiteral(val) => {
                                    println!("Error on line {}:\n Cannot assign string to variable of type bool", lineNum.clone());
                                    return false;
                                }
                                Expr::IntArrayLiteral(size, array) => {
                                    println!("Cannot assign array to variable of type {}", targType.clone());
                                    return false;
                                }
                                
                                //Operations
                                Expr::ArthOp(op1, op, op2) => {
                                    let checked = self.checkExpr(newValue.clone());
                                    if(checked){
                                        return true;
                                    } else {
                                        println!("Error in arithmetic operation on line {}", lineNum.clone());
                                        return false;
                                    }
                                }
                                Expr::LogOp(op1, op, op2) => {
                                    let checked = self.checkExpr(newValue.clone());
                                    if(checked){
                                        return true;
                                    } else {
                                        println!("Error in logical operation on line {}", lineNum.clone());
                                        return false;
                                    }
                                }
                                Expr::RelOp(op1, op, op2) => {
                                    let checked = self.checkExpr(newValue.clone());
                                    if(checked){
                                        return true;
                                    } else {
                                        println!("Error in relational operation on line {}", lineNum.clone());
                                        return false;
                                    }
                                }
                                
                                //Calls/references
                                Expr::ProcRef(procName, params) => {
                                    let mut procType: VarType;
                                    //Checks if procedure is defined
                                    let checkLocProc = self.localTable.getType(&procName.clone());
                                    match checkLocProc{
                                        Some(proc) => {
                                            procType = proc;
                                        }
                                        None => {
                                            println!("Procedure does not exist locally, checking global");
                                            let checkGlobProc = self.localTable.getType(&procName.clone());
                                            match checkGlobProc{
                                                Some(proc) => {
                                                    procType = proc
                                                }
                                                None => {
                                                    println!("Error on line {}:\n Procedure {} is not defined", lineNum.clone(), procName.clone());
                                                    return false;
                                                    
                                                }
                                            }
                                        }
                                    }
                                
                                    //Checks procedure type compatability with int
                                    match procType{
                                        VarType::Bool =>{
                                            return true;
                                        }
                                        VarType::Int =>{
                                            return true;
                                        }
                                        VarType::Float =>{
                                            return true;
                                        }
                                        _ => {
                                            return false;
                                        }
                                    }
                                }   
                                Expr::VarRef(assignName) => {
                                    let mut assignType: VarType;
                                    //Checks if variable is defined
                                    let checkLocVar = self.localTable.getType(&assignName.clone());
                                    match checkLocVar{
                                        Some(var) => {
                                            assignType = var;
                                        }
                                        None => {
                                            let checkGlobVar = self.localTable.getType(&assignName.clone());
                                            match checkGlobVar{
                                                Some(var) => {
                                                    assignType = var
                                                }
                                                None => {
                                                    println!("Error on line {}:\n Variable {} is not defined", lineNum.clone(), assignName.clone());
                                                    return false;
                                                    
                                                }
                                            }
                                        }
                                    }
                                
                                    //Checks variable type compatability with int
                                    match assignType{
                                        VarType::Bool =>{
                                            return true;
                                        }
                                        VarType::Int =>{
                                            return true;
                                        }
                                        VarType::Float =>{
                                            return true;
                                        }
                                        _ => {
                                            println!("Error on line {}:\n Cannot assign {} to variable {} of type {}", lineNum.clone(), assignType.clone(), targName.clone(), targType.clone());
                                            return false;
                                        }
                                    }
                                }
                            }
                        }
                        VarType::Float => {
                            match newValue.clone(){
                                //Literals
                                Expr::IntLiteral(val) => {
                                    return true;
                                }
                                Expr::FloatLiteral(val) => {
                                    return true;
                                }
                                Expr::ArrayRef(name, index) => {
                                    let checked = self.checkExpr(newValue.clone());
                                    if checked {
                                        return true;
                                    } else {
                                        println!("Error with array reference on line {}", lineNum.clone());
                                        return false;
                                    }
                                }
                                Expr::BoolLiteral(val) => {
                                    println!("Error on line {}:\n Cannot assign bool to variable of type float", lineNum.clone());
                                    return false;
                                }
                                Expr::StringLiteral(val) => {
                                    println!("Error on line {}:\n Cannot assign string to variable of type float", lineNum.clone());
                                    return false;
                                }
                                Expr::IntArrayLiteral(size, array) => {
                                    println!("Cannot assign array to variable of type {}", targType.clone());
                                    return false;
                                }

                                //Operations
                                Expr::ArthOp(op1, op, op2) => {
                                    let checked = self.checkExpr(newValue.clone());
                                    if(checked){
                                        return true;
                                    } else {
                                        println!("Error in arithmetic operation on line {}", lineNum.clone());
                                        return false;
                                    }
                                }          
                                Expr::LogOp(op1, op, op2) => {
                                    println!("Error on line {}:\n Cannot assign output of logical operation to variable of type float", lineNum.clone());
                                    return false;
                                }
                                Expr::RelOp(op1, op, op2) => {
                                    println!("Error on line {}:\n Cannot assign output of relational operation to variable of type float", lineNum.clone());
                                    return false;
                                }
                                
                                //Calls/references
                                Expr::ProcRef(procName, params) => {
                                    println!("Assigning: procedure {}", procName.clone());
                                    let mut procType: VarType;
                                    //Checks if procedure is defined
                                    let checkLocProc = self.localTable.getType(&procName.clone());
                                    match checkLocProc{
                                        Some(proc) => {
                                            procType = proc;
                                        }
                                        None => {
                                            println!("Procedure does not exist locally, checking global");
                                            let checkGlobProc = self.localTable.getType(&procName.clone());
                                            match checkGlobProc{
                                                Some(proc) => {
                                                    println!("procedure exists globally");
                                                    procType = proc
                                                }
                                                None => {
                                                    println!("Error on line {}:\n Procedure {} is not defined", lineNum.clone(), procName.clone());
                                                    return false;
                                                    
                                                }
                                            }
                                        }
                                    }
                                
                                    //Checks procedure type compatability with int
                                    match procType{
                                        VarType::Bool =>{
                                            println!("Error on line {}:\n Cannot assign output of procedure of type bool to variable of type float", lineNum.clone());
                                            return false;
                                        }
                                        VarType::Int =>{
                                            return true;
                                        }
                                        VarType::Float =>{
                                            return true;
                                        }
                                        _ => {
                                            println!("Error on line {}:\n Cannot assign {} to variable {} of type {}", lineNum.clone(), procType.clone(), targName.clone(), targType.clone());
                                            return false;
                                        }
                                    }
                                }   
                                Expr::VarRef(assignName) => {
                                    println!("Assigning: variable {}", assignName.clone());
                                    let mut assignType: VarType;
                                    //Checks if variable is defined
                                    let checkLocVar = self.localTable.getType(&assignName.clone());
                                    match checkLocVar{
                                        Some(var) => {
                                            println!("variable exists locally");
                                            assignType = var;
                                        }
                                        None => {
                                            println!("Variable does not exist locally, checking global");
                                            let checkGlobVar = self.localTable.getType(&assignName.clone());
                                            match checkGlobVar{
                                                Some(var) => {
                                                    println!("Variable exists globally");
                                                    assignType = var
                                                }
                                                None => {
                                                    println!("Error on line {}:\n Variable {} is not defined", lineNum.clone(), assignName.clone());
                                                    return false;
                                                    
                                                }
                                            }
                                        }
                                    }
                                
                                    //Checks variable type compatability with int
                                    match assignType{
                                        VarType::Bool =>{
                                            println!("Error on line {}:\n Cannot assign value of variable of type bool to variable of type float", lineNum.clone());
                                            return false;
                                        }
                                        VarType::Int =>{
                                            return true;
                                        }
                                        VarType::Float =>{
                                            return true;
                                        }
                                        _ => {
                                            println!("Error on line {}:\n Cannot assign {} to variable {} of type {}", lineNum.clone(), assignType.clone(), targName.clone(), targType.clone());
                                            return false;
                                        }
                                    }
                                }
                            }
                        }
                        VarType::Str => {
                            match newValue.clone(){
                                //Literals
                                Expr::IntLiteral(val) => {
                                    println!("Error on line {}:\n Cannot assign int to variable of type string", lineNum.clone());
                                    return false;
                                }
                                Expr::FloatLiteral(val) => {
                                    println!("Error on line {}:\n Cannot assign float to variable of type string", lineNum.clone());
                                    return false;
                                }
                                Expr::ArrayRef(name, index) => {
                                    println!("Error on line {}:\n Cannot assign int array value to variable of type string", lineNum.clone());
                                    return false;
                                }
                                Expr::BoolLiteral(val) => {
                                    println!("Error on line {}:\n Cannot assign bool to variable of type string", lineNum.clone());
                                    return false;
                                }
                                Expr::StringLiteral(val) => {
                                    return true;
                                }
                                Expr::IntArrayLiteral(size, array) => {
                                    println!("Cannot assign array to variable of type {}", targType.clone());
                                    return false;
                                }

                                //Operations
                                Expr::ArthOp(op1, op, op2) => {
                                    println!("Error on line {}:\n Cannot assign output of arithmetic operation to variable of type string", lineNum.clone());
                                    return false;
                                }          
                                Expr::LogOp(op1, op, op2) => {
                                    println!("Error on line {}:\n Cannot assign output of logical operation to variable of type string", lineNum.clone());
                                    return false;
                                }
                                Expr::RelOp(op1, op, op2) => {
                                    println!("Error on line {}:\n Cannot assign output of relational operation to variable of type string", lineNum.clone());
                                    return false;
                                }
                                
                                //Calls/references
                                Expr::ProcRef(procName, params) => {
                                    println!("Assigning: procedure {}", procName.clone());
                                    let mut procType: VarType;
                                    //Checks if procedure is defined
                                    let checkLocProc = self.localTable.getType(&procName.clone());
                                    match checkLocProc{
                                        Some(proc) => {
                                            procType = proc;
                                        }
                                        None => {
                                            println!("Procedure does not exist locally, checking global");
                                            let checkGlobProc = self.localTable.getType(&procName.clone());
                                            match checkGlobProc{
                                                Some(proc) => {
                                                    println!("procedure exists globally");
                                                    procType = proc
                                                }
                                                None => {
                                                    println!("Error on line {}:\n Procedure {} is not defined", lineNum.clone(), procName.clone());
                                                    return false;
                                                    
                                                }
                                            }
                                        }
                                    }
                                
                                    //Checks procedure type compatability with int
                                    match procType{
                                        VarType::Bool =>{
                                            println!("Error on line {}:\n Cannot assign output of procedure of type bool to variable of type float", lineNum.clone());
                                            return false;
                                        }
                                        VarType::Int =>{
                                            println!("Error on line {}:\n Cannot assign output of procedure of type integer to variable of type float", lineNum.clone());
                                            return false;
                                        }
                                        VarType::Float =>{
                                            println!("Error on line {}:\n Cannot assign output of procedure of float bool to variable of type float", lineNum.clone());
                                            return false;
                                        }
                                        VarType::Str => {
                                            return true;
                                        }
                                        _ => {
                                            println!("Error on line {}:\n Cannot assign {} to variable {} of type {}", lineNum.clone(), procType.clone(), targName.clone(), targType.clone());
                                            return false;
                                        }
                                    }
                                }   
                                Expr::VarRef(assignName) => {
                                    println!("Assigning: variable {}", assignName.clone());
                                    let mut assignType: VarType;
                                    //Checks if variable is defined
                                    let checkLocVar = self.localTable.getType(&assignName.clone());
                                    match checkLocVar{
                                        Some(var) => {
                                            println!("variable exists locally");
                                            assignType = var;
                                        }
                                        None => {
                                            println!("Variable does not exist locally, checking global");
                                            let checkGlobVar = self.localTable.getType(&assignName.clone());
                                            match checkGlobVar{
                                                Some(var) => {
                                                    println!("Variable exists globally");
                                                    assignType = var
                                                }
                                                None => {
                                                    println!("Error on line {}:\n Variable {} is not defined", lineNum.clone(), assignName.clone());
                                                    return false;
                                                    
                                                }
                                            }
                                        }
                                    }
                                
                                    //Checks variable type compatability with int
                                    match assignType{
                                        VarType::Bool =>{
                                            println!("Error on line {}:\n Cannot assign value of variable of type bool to variable of type float", lineNum.clone());
                                            return false;
                                        }
                                        VarType::Int =>{
                                            println!("Error on line {}:\n Cannot assign value of variable of type integer to variable of type float", lineNum.clone());
                                            return false;
                                        }
                                        VarType::Float =>{
                                            println!("Error on line {}:\n Cannot assign value of variable of type float to variable of type float", lineNum.clone());
                                            return false;
                                        }
                                        VarType::Str => {
                                            return true;
                                        }
                                        _ => {
                                            println!("Error on line {}:\n Cannot assign {} to variable {} of type {}", lineNum.clone(), assignType.clone(), targName.clone(), targType.clone());
                                            return false;
                                        }
                                    }
                                }
                            }
                        }
                        VarType::IntArray(targSize) => {
                            match newValue.clone(){
                                Expr::IntArrayLiteral(newSize, array) => {
                                    if(targSize == newSize) {
                                        return true;
                                    } else {
                                        println!("Error on line {}:\n When copying integers, sizes must be equivalent", lineNum.clone());
                                        return false;
                                    }
                                }
                                _ => {
                                    println!("Error on line {}:\n Cannot assign {} to integer array", lineNum.clone(), targType.clone());
                                    return true;
                                }
                            }
                        }
                    }

                } 
                
                //For index value references
                else if let Expr::ArrayRef(ref targName, targIndexExpr) = valueToAssign {
                    //Check if variable assignment is in the local table
                    let mut targValue: HashItem; 
                    //Looks for the value in the local then global table, retrieves it if so
                    if !(self.localTable.checkItem(targName)){
                        if !(self.globalTable.checkItem(targName)){
                            println!("Attempting to assign value to undeclared variable: {} on line: {}", targName.clone(), lineNum.clone());
                            return false;
                        } else {
                            let gotValue = self.globalTable.get(targName);
                            match gotValue{
                                Some(val) => {
                                    targValue = val.clone();
                                }
                                None => {
                                    println!("Error with value {} on line: {}", targName.clone(), lineNum.clone());
                                    return false;
                                }
                            }
                        }
                    } else {
                        let gotValue = self.localTable.get(targName);
                        match gotValue{
                            Some(val) => {
                                targValue = val.clone();
                            }
                            None => {
                                println!("Error with value {} on line: {}", targName.clone(), lineNum.clone());
                                return false;
                            }
                        }
                    }
                    
                    //Checks if value being assigned to is a variable
                    if targValue.hashType != HashItemType::Variable {
                        println!("On line: {}, cannot assign value to procedure", lineNum.clone());
                        return false;
                    }
                    
                    //Checks to ensure that new value matches target value
                    let targType = targValue.getType();
                    match targType{
                        //The only correct one
                        VarType::IntArray(targSize) => {
                            
                            //Checks if the expression making up the index is valid
                            let checked = self.checkExpr(*targIndexExpr.clone());
                            if (checked){
                            } else {
                                println!("Error with index expression on line {}", lineNum.clone());
                                return false;
                            }
                            
                            //Reacts based on the type of expression the index expression is
                            match *targIndexExpr{
                                //Literals
                                Expr::IntLiteral(val) => {
                                    if (val > targSize.into()){
                                        println!("Error on line {}:\n Index {} is out of bounds", lineNum.clone(), val.clone())
                                    } else {
                                    }
                                    
                                }
                                Expr::FloatLiteral(val) => {
                                    println!("Error on line {}:\n Cannot use float as index value", lineNum.clone());
                                    return false;
                                }
                                Expr::ArrayRef(name, index) => {
                                    let checked = self.checkExpr(newValue.clone());
                                    if checked {
                                        return true;
                                    } else {
                                        println!("Error with array reference on line {}", lineNum.clone());
                                        return false;
                                    }
                                }
                                Expr::BoolLiteral(val) => {
                                    println!("Error on line {}:\n Cannot use bool as index value", lineNum.clone());
                                    return false;
                                }
                                Expr::StringLiteral(val) => {
                                    println!("Error on line {}:\n Cannot use string as index value", lineNum.clone());
                                    return false;
                                }
                                Expr::IntArrayLiteral(size, array) => {
                                    println!("Error on line {}:\n Cannot use array as index value", lineNum.clone());
                                    return false;
                                }

                                //Operations
                                Expr::ArthOp(op1, op, op2) => {
                                    let checked = self.checkExpr(newValue.clone());
                                    if(checked){
                                        return true
                                    } else {
                                        println!("Error in arithmetic operation on line {}", lineNum.clone());
                                        return false;
                                    }
                                }
                                Expr::LogOp(op1, op, op2) => {
                                    println!("Error on line {}:\n Cannot use logical operation as index value", lineNum.clone());
                                    return false;
                                }
                                Expr::RelOp(op1, op, op2) => {
                                    println!("Error on line {}:\n Cannot use relational operation as index value", lineNum.clone());
                                    return false;
                                }
                                
                                //Calls/references
                                Expr::ProcRef(procName, params) => {
                                    println!("Indexing with procedure {}", procName.clone());
                                    let mut procType: VarType;
                                    //Checks if procedure is defined
                                    let checkLocProc = self.localTable.getType(&procName.clone());
                                    match checkLocProc{
                                        Some(proc) => {
                                            procType = proc;
                                        }
                                        None => {
                                            let checkGlobProc = self.localTable.getType(&procName.clone());
                                            match checkGlobProc{
                                                Some(proc) => {
                                                    procType = proc
                                                }
                                                None => {
                                                    println!("Error on line {}:\n Procedure {} is not defined", lineNum.clone(), procName.clone());
                                                    return false;
                                                    
                                                }
                                            }
                                        }
                                    }
                                
                                    //Checks procedure type compatability with int
                                    match procType{
                                        VarType::Bool =>{
                                            println!("Error on line {}:\n Cannot use procedure of type bool as index value", lineNum.clone());
                                            return false;
                                        }
                                        VarType::Int =>{
                                            println!("Procedure type int");
                                            
                                        }
                                        VarType::Float =>{
                                            println!("Error on line {}:\n Cannot use procedure of type float as index value", lineNum.clone());
                                            return false;
                                        }
                                        _ => {
                                            println!("Error on line {}:\n Cannot use procedure {} to index integer array", lineNum.clone(), procName.clone());
                                            return false;
                                        }
                                    }
                                }   
                                Expr::VarRef(indexVarName) => {
                                    println!("indexing with variable {}", indexVarName.clone());
                                    let mut assignType: VarType;
                                    //Checks if variable is defined
                                    let checkLocVar = self.localTable.getType(&indexVarName.clone());
                                    match checkLocVar{
                                        Some(var) => {
                                            println!("variable exists locally");
                                            assignType = var;
                                        }
                                        None => {
                                            println!("Variable does not exist locally, checking global");
                                            let checkGlobVar = self.localTable.getType(&indexVarName.clone());
                                            match checkGlobVar{
                                                Some(var) => {
                                                    println!("Variable exists globally");
                                                    assignType = var
                                                }
                                                None => {
                                                    println!("Error on line {}:\n Variable {} is not defined", lineNum.clone(), indexVarName.clone());
                                                    return false;
                                                    
                                                }
                                            }
                                        }
                                    }
                                
                                    //Checks variable type compatability with int
                                    match assignType{
                                        VarType::Bool =>{
                                            println!("Error on line {}:\n Cannot use variable of type bool as index value", lineNum.clone());
                                            return false;
                                        }
                                        VarType::Int =>{
                                            println!("variable type int");
                                            
                                        }
                                        VarType::Float =>{
                                            println!("Error on line {}:\n Cannot use variable of type float as index value", lineNum.clone());
                                            return false;
                                        }
                                        _ => {
                                            println!("Error on line {}:\n Cannot use variable {} to index integer array", lineNum.clone(), indexVarName.clone());
                                            return false;
                                        }
                                    }
                                }
                            }
                            

                            //Now that we know the index is good, need to check the target assignment
                            match newValue.clone(){
                                //Literals
                                Expr::IntLiteral(val) => {
                                    return true;
                                }
                                Expr::FloatLiteral(val) => {
                                    return true;
                                }
                                Expr::ArrayRef(name, index) => {
                                    let checked = self.checkExpr(newValue.clone());
                                    if checked {
                                        return true;
                                    } else {
                                        println!("Error with array reference on line {}", lineNum.clone());
                                        return false;
                                    }
                                }
                                Expr::BoolLiteral(val) => {
                                    return true;
                                }
                                Expr::StringLiteral(val) => {
                                    println!("Error on line {}:\n Cannot assign string to variable of type int", lineNum.clone());
                                    return false;
                                }
                                Expr::IntArrayLiteral(size, array) => {
                                    println!("Cannot assign array to variable of type {}", targType.clone());
                                    return false;
                                }

                                //Operations
                                Expr::ArthOp(op1, op, op2) => {
                                    let checked = self.checkExpr(newValue.clone());
                                    if(checked){
                                        return true;
                                    } else {
                                        println!("Error in arithmetic operation on line {}", lineNum.clone());
                                        return false;
                                    }
                                }
                                Expr::LogOp(op1, op, op2) => {
                                    println!("Assigning LogOp");
                                    println!("Checking expression");
                                    let checked = self.checkExpr(newValue.clone());
                                    if(checked){
                                        return true;
                                    } else {
                                        println!("Error in logical operation on line {}", lineNum.clone());
                                        return false;
                                    }
                                }
                                Expr::RelOp(op1, op, op2) => {
                                    println!("Assigning RelOp");
                                    println!("Checking expression");
                                    let checked = self.checkExpr(newValue.clone());
                                    if(checked){
                                        return true;
                                    } else {
                                        println!("Error in relational operation on line {}", lineNum.clone());
                                        return false;
                                    }
                                }
                                
                                //Calls/references
                                Expr::ProcRef(procName, params) => {
                                    println!("Assigning: procedure {}", procName.clone());
                                    let mut procType: VarType;
                                    //Checks if procedure is defined
                                    let checkLocProc = self.localTable.getType(&procName.clone());
                                    match checkLocProc{
                                        Some(proc) => {
                                            procType = proc;
                                        }
                                        None => {
                                            let checkGlobProc = self.localTable.getType(&procName.clone());
                                            match checkGlobProc{
                                                Some(proc) => {
                                                    procType = proc
                                                }
                                                None => {
                                                    println!("Error on line {}:\n Procedure {} is not defined", lineNum.clone(), procName.clone());
                                                    return false;
                                                    
                                                }
                                            }
                                        }
                                    }
                                
                                    //Checks procedure type compatability with int
                                    match procType{
                                        VarType::Bool =>{
                                            return true;
                                        }
                                        VarType::Int =>{
                                            return true;
                                        }
                                        VarType::Float =>{
                                            return true;
                                        }
                                        _ => {
                                            println!("Error on line {}:\n Cannot assign {} to variable {} of type {}", lineNum.clone(), procType.clone(), targName.clone(), targType.clone());
                                            return false;
                                        }
                                    }
                                }   
                                Expr::VarRef(assignName) => {
                                    println!("Assigning: variable {}", assignName.clone());
                                    let mut assignType: VarType;
                                    //Checks if variable is defined
                                    let checkLocVar = self.localTable.getType(&assignName.clone());
                                    match checkLocVar{
                                        Some(var) => {
                                            println!("variable exists locally");
                                            assignType = var;
                                        }
                                        None => {
                                            println!("Variable does not exist locally, checking global");
                                            let checkGlobVar = self.localTable.getType(&assignName.clone());
                                            match checkGlobVar{
                                                Some(var) => {
                                                    println!("Variable exists globally");
                                                    assignType = var
                                                }
                                                None => {
                                                    println!("Error on line {}:\n Variable {} is not defined", lineNum.clone(), assignName.clone());
                                                    return false;
                                                    
                                                }
                                            }
                                        }
                                    }
                                
                                    //Checks variable type compatability with int
                                    match assignType{
                                        VarType::Bool =>{
                                            return true;
                                        }
                                        VarType::Int =>{
                                            return true;
                                        }
                                        VarType::Float =>{
                                            return true;
                                        }
                                        _ => {
                                            println!("Error on line {}:\n Cannot assign {} to variable {} of type {}", lineNum.clone(), assignType.clone(), targName.clone(), targType.clone());
                                            return false;
                                        }
                                    }
                                }
                            }                        
                        }
                        _ => {
                            println!("Error on line {}:\n Variable {} is not an array", lineNum.clone(), targName.clone());
                            return false;
                        }
                    }

                } else {
                    println!("On line {}: cannot assign to non-variable", lineNum.clone());
                    return false;
                }


            }
            //For Stmts that are just Exprs
            Stmt::Expr(expr, lineNum) => {
                match (expr){
                    _ => {
                        let checked = self.checkExpr(expr.clone());
                        if checked {
                            return true;
                        } else {
                            println!("Error with expression statement on line {}", lineNum.clone());
                            return false;
                        }
                    }
                }
            }
            //For checking if statements
            Stmt::If(condition, body, elseBody, lineNum) => {
                //Checks the condition
                match condition.clone() {
                    Expr::IntArrayLiteral(size, array) => {
                        println!("Error with if condition on line {}:\n Cannot use array as condition", lineNum.clone());
                        return false;
                    }
                    Expr::FloatLiteral(val) => {
                        println!("Error with if condition on line {}:\n Cannot use float as condition", lineNum.clone());
                        return false;
                    }
                    Expr::StringLiteral(val) => {
                        println!("Error with if condition on line {}:\n Cannot use string as condition", lineNum.clone());
                        return false;
                    }
                    
                    
                    Expr::ProcRef(procName, params) => {
                        let mut procType: VarType;
                        //Checks if procedure is defined
                        let checkLocProc = self.localTable.getType(&procName.clone());
                        match checkLocProc{
                            Some(proc) => {
                                procType = proc;
                            }
                            None => {
                                let checkGlobProc = self.localTable.getType(&procName.clone());
                                match checkGlobProc{
                                    Some(proc) => {
                                        procType = proc
                                    }
                                    None => {
                                        println!("Error on line {}:\n Procedure {} is not defined", lineNum.clone(), procName.clone());
                                        return false;
                                        
                                    }
                                }
                            }
                        }
                    
                        //Checks procedure type compatability with int
                        match procType{
                            VarType::Bool =>{
                                println!("Procedure type bool");
                            }
                            VarType::Int =>{
                                println!("Procedure type int");
                            }
                            VarType::Float =>{
                                println!("Error with if condition on line {}:\n Cannot use float procedure as condition", lineNum.clone());
                        return false;
                            }
                            _ => {
                                println!("Error on line {}:\n Cannot use procedure of type {} as if condition", lineNum.clone(), procType.clone());
                                return false;
                            }
                        }

                        //Checks if the condition is good
                        let goodCond = self.checkExpr(condition.clone());
                        //If the condition is bad, fails here
                        if (!goodCond){
                            println!("Error in if condition on line {}", lineNum.clone());
                            return false;
                        //If the condition is good, checks the rest of the if statement
                        } else {
                            //Checks the if body
                            let goodIfBody = self.checkStmt(*body);
                            //If the body if good
                            if(goodIfBody){
                                //Checks if there is an else
                                match elseBody{
                                    //Checks the else
                                    Some(elseStmt) => {
                                        let goodElse = self.checkStmt(*elseStmt.clone());
                                        if(!goodElse){
                                            println!("Error with else in if statement on line {}", lineNum.clone());
                                            return false;
                                        } else {
                                            return true;
                                        }
                                    }
                                    //If statement is good here if no else
                                    None => {
                                        return true;
                                    }

                                }
                            } else {
                                println!("Error with body of if statement on line: {}", lineNum.clone());
                                return false;
                            }
                        }


                    }   
                    
                    Expr::VarRef(varCondName) => {
                        println!("Assigning: variable {}", varCondName.clone());
                        let mut ifCondType: VarType;
                        //Checks if variable is defined
                        let checkLocVar = self.localTable.getType(&varCondName.clone());
                        match checkLocVar{
                            Some(var) => {
                                println!("variable exists locally");
                                ifCondType = var;
                            }
                            None => {
                                println!("Variable does not exist locally, checking global");
                                let checkGlobVar = self.localTable.getType(&varCondName.clone());
                                match checkGlobVar{
                                    Some(var) => {
                                        println!("Variable exists globally");
                                        ifCondType = var
                                    }
                                    None => {
                                        println!("Error on line {}:\n Variable {} is not defined", lineNum.clone(), varCondName.clone());
                                        return false;
                                        
                                    }
                                }
                            }
                        }
                    
                        //Checks variable type compatability with int
                        match ifCondType{
                            VarType::Bool =>{
                                println!("Variable type bool");
                            }
                            VarType::Int =>{
                                println!("Variable type int");
                            }
                            VarType::Float =>{
                                println!("Error on line {}:\n Cannot use variable of type float as if condition", lineNum.clone());
                                return false;
                            }
                            _ => {
                                println!("Error on line {}:\n Cannot use variable of type {} for if condition", lineNum.clone(), ifCondType.clone());
                                return false;
                            }
                        }

                        //Checks if the condition is good
                        let goodCond = self.checkExpr(condition.clone());
                        //If the condition is bad, fails here
                        if (!goodCond){
                            println!("Error in if condition on line {}", lineNum.clone());
                            return false;
                        //If the condition is good, checks the rest of the if statement
                        } else {
                            //Checks the if body
                            let goodIfBody = self.checkStmt(*body);
                            //If the body if good
                            if(goodIfBody){
                                //Checks if there is an else
                                match elseBody{
                                    //Checks the else
                                    Some(elseStmt) => {
                                        let goodElse = self.checkStmt(*elseStmt.clone());
                                        if(!goodElse){
                                            println!("Error with else in if statement on line {}", lineNum.clone());
                                            return false;
                                        } else {
                                            return true;
                                        }
                                    }
                                    //If statement is good here if no else
                                    None => {
                                        return true;
                                    }

                                }
                            } else {
                                println!("Error with body of if statement on line: {}", lineNum.clone());
                                return false;
                            }
                        }
                    }
                

                    
                    //All of the good conditions
                    _ => {
                        //Checks if the condition is good
                        let goodCond = self.checkExpr(condition.clone());
                        //If the condition is bad, fails here
                        if (!goodCond){
                            println!("Error in if condition on line {}", lineNum.clone());
                            return false;
                        //If the condition is good, checks the rest of the if statement
                        } else {
                            //Checks the if body
                            let goodIfBody = self.checkStmt(*body);
                            //If the body if good
                            if(goodIfBody){
                                //Checks if there is an else
                                match elseBody{
                                    //Checks the else
                                    Some(elseStmt) => {
                                        let goodElse = self.checkStmt(*elseStmt.clone());
                                        if(!goodElse){
                                            println!("Error with else in if statement on line {}", lineNum.clone());
                                            return false;
                                        } else {
                                            return true;
                                        }
                                    }
                                    //If statement is good here if no else
                                    None => {
                                        return true;
                                    }

                                }
                            } else {
                                println!("Error with body of if statement on line: {}", lineNum.clone());
                                return false;
                            }
                        }
                    }
                }
            }    
            Stmt::For(assignment, condition, body, lineNum) => {

                //Checks if the condition is valid
                let checked = self.checkExpr(condition.clone());
                if checked {
                    //Continue
                } else {
                    println!("Error with for condition on line {}", lineNum.clone());
                    return false;
                }

                //Ensures for condition is the correct type
                match condition.clone() {
                    Expr::IntArrayLiteral(size, array) => {
                        println!("Error with if condition on line {}:\n Cannot use array as condition", lineNum.clone());
                        return false;
                    }
                    Expr::FloatLiteral(val) => {
                        println!("Error with if condition on line {}:\n Cannot use float as condition", lineNum.clone());
                        return false;
                    }
                    Expr::StringLiteral(val) => {
                        println!("Error with if condition on line {}:\n Cannot use string as condition", lineNum.clone());
                        return false;
                    }
                    
                    
                    Expr::ProcRef(procName, params) => {
                        println!("If condition procedure {}", procName.clone());
                        let mut procType: VarType;
                        //Checks if procedure is defined
                        let checkLocProc = self.localTable.getType(&procName.clone());
                        match checkLocProc{
                            Some(proc) => {
                                procType = proc;
                            }
                            None => {
                                let checkGlobProc = self.localTable.getType(&procName.clone());
                                match checkGlobProc{
                                    Some(proc) => {
                                        procType = proc
                                    }
                                    None => {
                                        println!("Error on line {}:\n Procedure {} is not defined", lineNum.clone(), procName.clone());
                                        return false;
                                        
                                    }
                                }
                            }
                        }
                    
                        //Checks procedure type compatability with int
                        match procType{
                            VarType::Bool =>{
                                println!("Procedure type bool");
                            }
                            VarType::Int =>{
                                println!("Procedure type int");
                            }
                            VarType::Float =>{
                                println!("Error with for condition on line {}:\n Cannot use float procedure as condition", lineNum.clone());
                        return false;
                            }
                            _ => {
                                println!("Error on line {}:\n Cannot use procedure of type {} as for condition", lineNum.clone(), procType.clone());
                                return false;
                            }
                        }
                    }   
                    
                    Expr::VarRef(varCondName) => {
                        println!("Assigning: variable {}", varCondName.clone());
                        let mut forCondType: VarType;
                        //Checks if variable is defined
                        let checkLocVar = self.localTable.getType(&varCondName.clone());
                        match checkLocVar{
                            Some(var) => {
                                println!("variable exists locally");
                                forCondType = var;
                            }
                            None => {
                                println!("Variable does not exist locally, checking global");
                                let checkGlobVar = self.localTable.getType(&varCondName.clone());
                                match checkGlobVar{
                                    Some(var) => {
                                        println!("Variable exists globally");
                                        forCondType = var
                                    }
                                    None => {
                                        println!("Error on line {}:\n Variable {} is not defined", lineNum.clone(), varCondName.clone());
                                        return false;
                                        
                                    }
                                }
                            }
                        }
                    
                        //Checks variable type compatability with int
                        match forCondType{
                            VarType::Bool =>{
                                println!("Variable type bool");
                            }
                            VarType::Int =>{
                                println!("Variable type int");
                            }
                            VarType::Float =>{
                                println!("Error on line {}:\n Cannot use variable of type float as for condition", lineNum.clone());
                                return false;
                            }
                            _ => {
                                println!("Error on line {}:\n Cannot use variable of type {} as for condition", lineNum.clone(), forCondType.clone());
                                return false;
                            }
                        }
                    }
                

                    
                    //All of the good conditions
                    _ => {
                        //Checks if the condition is good
                        let goodCond = self.checkExpr(condition.clone());
                        //If the condition is bad, fails here
                        if (!goodCond){
                            println!("Error in if condition on line {}", lineNum.clone());
                            return false;
                        //If the condition is good, checks the rest of the if statement
                        } else {
                            //continue
                        }
                    }
                }

                //Checks the for body
                let forBodyCheck = self.checkStmt(*body);
                //If the body for good
                if(forBodyCheck){
                    //Checks for there is an else
                    return true;
                } else {
                    println!("Error with body of for statement on line: {}", lineNum.clone());
                    return false;
                }
            }  
            Stmt::Block(stmts, lineNum) => {
                for instr in stmts {
                    let good = self.checkStmt(instr.clone());
                    if (!good){
                        println!("Error in header:");
                        instr.display(0);
                        return false;
                    } else {
                        //continue
                    }
                }
                return true;
            }
            Stmt::Error(report, errMsg) => {
                println!("Error found in AST: {}", errMsg);
                return false;
            }
            Stmt::Program(name, header, body, lineNum) => {
                return true;
            }
            Stmt::Return(retVal, lineNum) => {
                let checked = self.checkExpr(retVal.clone());
                if checked {
                    return true;
                } else {
                    println!("Error with return statement on line {}", lineNum.clone());
                    return false;
                }
            }
            Stmt::StringLiteral(val, lineNum) => {
                return true;
            }
        }
    }
}

//Used for storing the values of a hashed item
#[derive(Debug, Clone, PartialEq)]
pub enum HashItemType{
    Procedure(Box<Stmt>, Vec<String>, SymbolTable),   //For storing procedures (The procedure AST, a list of the parameter names in order, the SymbolTable populated with the parameters)
    Variable,
}
impl HashItemType{
    pub fn newProcItem(procAst: Box<Stmt>, params: Vec<String>, procST: SymbolTable) -> HashItemType{
        return HashItemType::Procedure(procAst, params, procST);
    }
}

//An enum used for storing objects in the main hash map
#[derive(Debug, Clone, PartialEq)]
pub struct HashItem {
    itemType: VarType,      //The type of the variable/the return type of the proc
    name: String,           //The name of the item
    hashType: HashItemType, //Which type of hash item it is (Procedure or variable)
}
//Assistive functions for hashItem
impl HashItem {
    
    //Used for creating a variable entry in the symbol table
    pub fn newVar(varName: String, variableType: VarType) -> HashItem {
        HashItem{
            itemType: variableType,
            name: varName,
            hashType: HashItemType::Variable,
        }
    }

    //Used for creating a process entry for the symbol table
    pub fn newProc(procName: String, procType: VarType, procItem: HashItemType) -> HashItem {
        HashItem{
            itemType: procType,
            name: procName,
            hashType: procItem,
        }
    }

    //The getter for type
    pub fn getType(&mut self) -> VarType {
        return self.itemType.clone();
    }

    //Checking an items type against another
    pub fn checkType(&mut self, typeCheck: VarType) -> bool {
        if(typeCheck == self.itemType){
            return true;
        } else {
            return false;
        }
    }



}

//The structure for the SymbolTable. This holds all of the IDENTIFIERS of the program as well as their scope and information
#[derive(Debug, Clone, PartialEq)]
pub struct SymbolTable{
    symTab: HashMap<String, HashItem>,
}
impl SymbolTable{
    // The symbol table hashLook function should automatically create a new entry and mark it as an
    // IDENTIFIER Token for any IDENTIFIER string that is not already in the symbol table. In some languages
    // case does not matter to the uniqueness of the symbol. In this case, an easy place to solve this is to simply
    // upper case or lower case all strings in the symbol table API functions (and storage)
    pub fn new() -> SymbolTable {
        //Creates the empty hash map
        let mut symHash: HashMap<String, HashItem> = HashMap::new();

        //Seeding the symbol table with the built in functions
        let builtIns = vec![
            ("getbool", HashItem::newProc("getbool".to_string(), VarType::Bool, HashItemType::Procedure(Box::new(Stmt::StringLiteral("NONE".to_string(), ("0".to_string()))), Vec::new(), SymbolTable::newBuiltIn()))),
            ("getinteger", HashItem::newProc("getinteger".to_string(), VarType::Int, HashItemType::Procedure(Box::new(Stmt::StringLiteral("NONE".to_string(), ("0".to_string()))), Vec::new(), SymbolTable::newBuiltIn()))),
            ("getfloat", HashItem::newProc("getfloat".to_string(), VarType::Float, HashItemType::Procedure(Box::new(Stmt::StringLiteral("NONE".to_string(), ("0".to_string()))), Vec::new(), SymbolTable::newBuiltIn()))),
            ("getstring", HashItem::newProc("getstring".to_string(), VarType::Str, HashItemType::Procedure(Box::new(Stmt::StringLiteral("NONE".to_string(), ("0".to_string()))), Vec::new(), SymbolTable::newBuiltIn()))),
            ("getbool", HashItem::newProc("getBool".to_string(), VarType::Bool, HashItemType::Procedure(Box::new(Stmt::StringLiteral("NONE".to_string(), ("0".to_string()))), Vec::new(), SymbolTable::newBuiltIn()))),
            
            (
                "putbool",
                HashItem::newProc(
                    "putbool".to_string(),
                    VarType::Bool,
                    HashItemType::Procedure(
                        Box::new(Stmt::StringLiteral("NONE".to_string(), "0".to_string())),
                        vec!["boolparam".to_string()], 
                        SymbolTable::newBuiltIn(),
                    ),
                ),
            ),

            (
                "putinteger",
                HashItem::newProc(
                    "putinteger".to_string(),
                    VarType::Bool,
                    HashItemType::Procedure(
                        Box::new(Stmt::StringLiteral("NONE".to_string(), "0".to_string())),
                        vec!["intparam".to_string()], 
                        SymbolTable::newBuiltIn(),
                    ),
                ),
            ),

            (
                "putfloat",
                HashItem::newProc(
                    "putfloat".to_string(),
                    VarType::Bool,
                    HashItemType::Procedure(
                        Box::new(Stmt::StringLiteral("NONE".to_string(), "0".to_string())),
                        vec!["floatparam".to_string()],  
                        SymbolTable::newBuiltIn(),
                    ),
                ),
            ),

            (
                "putstring",
                HashItem::newProc(
                    "putstring".to_string(),
                    VarType::Bool,
                    HashItemType::Procedure(
                        Box::new(Stmt::StringLiteral("NONE".to_string(), "0".to_string())),
                        vec!["stringparam".to_string()],
                        SymbolTable::newBuiltIn(),
                    ),
                ),
            ),

            (
                "sqrt",
                HashItem::newProc(
                    "sqrt".to_string(),
                    VarType::Float,
                    HashItemType::Procedure(
                        Box::new(Stmt::StringLiteral("NONE".to_string(), "0".to_string())),
                        vec!["intparam".to_string()],
                        SymbolTable::newBuiltIn(),
                    ),
                ),
            ),
        ];
        //Inserted seed values into hash table
        for (key, value) in builtIns {
            symHash.insert(key.to_string(), value);
        }

        // println!("symbol table created");
        // for (key, token) in &mut symHash {
        //     println!("Key: {}, Token: {:?}", key, token.printToken());
        // }


        SymbolTable{
            symTab: symHash,
        }
    }
    
    pub fn newBuiltIn() -> SymbolTable {
        //Creates the empty hash map
        let mut symHash: HashMap<String, HashItem> = HashMap::new();

        let mut builtInHash: HashMap<String, HashItem> = HashMap::new();
        let builtInStmt = Stmt::StringLiteral(("NULL".to_string()), "0".to_string());
        //Seeding the symbol table with the built in functions
        let builtIns = vec![
            ("boolparam", HashItem::newVar("boolparam".to_string(), VarType::Bool)),
            ("intparam", HashItem::newVar("intparam".to_string(), VarType::Int)),
            ("floatparam", HashItem::newVar("floatparam".to_string(), VarType::Float)),
            ("stringparam", HashItem::newVar("stringparam".to_string(), VarType::Str)),


        ];
        //Inserted seed values into hash table
        for (key, value) in builtIns {
            symHash.insert(key.to_string(), value);
        }

        


        SymbolTable{
            symTab: symHash,
        }
    }

    pub fn newEmpty() -> SymbolTable {
        //Creates the empty hash map
        let mut symHash: HashMap<String, HashItem> = HashMap::new();

        

        SymbolTable{
            symTab: symHash,
        }
    }



    //Returns an option of if the item exists or not
    pub fn get(&mut self, itemName: &String) -> Option<&HashItem> {
        if self.symTab.contains_key(itemName) {
            return(self.symTab.get(itemName));
            // println!("Key '{}' exists in the map.", key);
        } else {
            return None;
        }
    }

    //Returns an option for the type of an item given a name
    pub fn getType(&mut self, itemName: &String) -> Option<VarType> {
        let value = self.symTab.get(itemName);
        match value{
            Some(v) =>{
                let itemType = v.clone().getType();
                return Some(itemType);

            },
            None => {
                return None;
            }

        }
    }

    //Checks if a variable/procedure is in the table, returns a bool
    pub fn checkItem(&mut self, itemName: &String) -> bool {
        let value = self.symTab.get(itemName);
        match value{
            Some(v) =>{
                return true;

            },
            None => {
                return false;
            }

        }
    }
}

fn combine_blocks(block1: Box<Stmt>, block2: Box<Stmt>) -> Option<Vec<Stmt>> {
    if let (Stmt::Block(mut stmts1, _), Stmt::Block(stmts2, _)) = (*block1, *block2) {
        stmts1.extend(stmts2);
        Some(stmts1)
    } else {
        None
    }
}

///////////////////////// /TYPE CHECKING SECTION /////////////////////////
    