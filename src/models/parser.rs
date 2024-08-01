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
    crate::models::lexer::*,
    crate::models::reporting::Reporting,
    std::io::prelude::*,
};

///////////////////////// /Setup /////////////////////////



///////////////////////// PARSER SECTION /////////////////////////

//This is the master struct for the parser
pub struct Parser {
    pub tokenList: Vec<Token>,  //The list of tokens that is passed into it. This comes from the Lexer
    pub reports: Reporting,         //The reporting object, used to report warnings and errors
    pub scope: i32,                 //the scope
}

impl Parser{
    //The constructor. Takes in a Lexer and extracts its tokenList, cloning it and creating the parser
    pub fn new(lexer: &mut Lexer) -> Parser {
        let tokenList = lexer.tokenList.clone();
        let mut report: Reporting = Reporting::new();
        Parser { 
            tokenList,
            reports: report,
            scope: 0,
        }
    }  

    //The public function that is used to parse the entire program, returns the AST if successful
    //Returns the reporting structure if not
    pub fn startParse(&mut self) -> Result<(Reporting, Option<Stmt>), Reporting> {
        // println!("Starting master parse");

        //Starts the master parse which runs recursively until all tokens have been parsed
        let mut tokList = self.tokenList.clone();
        let parsed = self.parse(&mut tokList);

        //Checks that parsing was completed successfully
        match parsed {
            Ok((Some(stmt))) => {
                return Ok((self.reports.clone(), Some(stmt)));
            },
            Ok((None)) => {
                return Ok((self.reports.clone(), None));
            },
            Err(reporting) => {
                return Err(self.reports.clone());
            },
        }

    }

    //Parses a expressions and returns an Expr which is used within program AST Stmt
    fn parseExpr(&mut self, tokenList: &mut Vec<Token>) -> Result<Expr, String> {
        //Initializes the variable that is being referenced first
        let mut firstOp:Expr = Expr::StringLiteral(("NONE".to_string()));

        //Initializes values for finding the end of the expression
        let mut k = 0;
        let mut nextTok = &tokenList[k];
        let mut curStmt: Vec<Token> = vec![];
        
        //Finds the end of the expression
        while k < tokenList.len() {
            let nextTok = &tokenList[k];
            curStmt.push(nextTok.clone());
        
            if (nextTok.tt == tokenTypeEnum::SEMICOLON){
                break;
            }
        
            k += 1;
        }

        //Checks if the first value is an array reference or not
        if(curStmt[1].tt == tokenTypeEnum::L_BRACKET) {
            let varName = curStmt[0].tokenString.clone();
            // let varRef = Expr::VarRef((varName));

            //Finds the end of the brackets
            let mut brackInd = 0;
            let mut nextIndToken = &curStmt[brackInd];
            let mut indexList: Vec<Token> = vec![];
            
            //Finds the end of the index statement
            while brackInd < curStmt.len() {
                let nextTok = &curStmt[brackInd];
                indexList.push(nextTok.clone());
            
                if (nextTok.tt == tokenTypeEnum::R_BRACKET) {
                    break;
                }
            
                brackInd += 1;
            }

            //If the end of the index was not found, error
            if brackInd == curStmt.len() {
                tokenList.drain(..k+1);
                let errMsg = format!("Error finding the end of the array index on line {}", curStmt[1].lineNum);
                return(Err(errMsg));
            }

            //Removes the index assignment so there is just the ] left for proper parsing of index expression
            indexList.drain(0..2);

            //Recursively parses the expr that makes up the index of the arrayRef call
            let parsedExpr = self.parseExpr(&mut indexList);
            let mut indexExpr: Expr;

            //Extracts the resulting expression
            match parsedExpr {
                Ok(expr) => {
                    indexExpr = expr;
                }
                Err(err) => {
                    let errMsg = format!("Error on line {}: {}", tokenList[0].lineNum, err);
                    self.reports.reportError(errMsg);
                    return Err("Error with expression".to_string());
                }
            }

            //Creates the necessary structure for the array reference Stmt
            let indexBox = Box::new(indexExpr);
            firstOp = Expr::ArrayRef((varName), (indexBox));
            //Removes the array reference so there is just the ] left
            let modifier: usize;
            println!("Next string after array ref {}", curStmt[brackInd + 1].tokenString.clone());
            if(curStmt[brackInd + 1].tt.clone() == tokenTypeEnum::SEMICOLON){
                modifier = 1;
            } else {
                modifier = 0;
            }
            
            curStmt.drain(0..brackInd + modifier);
            println!("Next string after array ref {}", curStmt[0].tokenString.clone());

        } 
        //If the first token in the expr list is a variable
        else if (curStmt[0].tg == tokenGroup::VARIABLE){
            //If not an array
            firstOp = Expr::VarRef(curStmt[0].tokenString.clone());
        } 
        //If the expression contains a (, finds the end of it and parses the interior expression
        else if (curStmt[0].tt == tokenTypeEnum::L_PAREN) { 
            let mut scope = 0;

            //Initializes values for finding the end of the expression
            let mut parenInd = 1;
            // let mut nextParenTok = &curStmt[1];
            let mut parStmt: Vec<Token> = vec![];
            
            //Finds the end of the expression
            while parenInd < curStmt.len() {
                let nextParenTok = &curStmt[parenInd];
                parStmt.push(nextParenTok.clone());
                
                if (nextParenTok.tt == tokenTypeEnum::L_PAREN){
                    scope += 1;
                }

                //Finds the end of the parentheses, including nested parentheses
                if (nextParenTok.tt == tokenTypeEnum::R_PAREN){
                    if(scope != 0){
                        scope -= 1;
                    } else {
                        break;
                    }
                }
            
                //Increments
                parenInd += 1;
            }

            //modifies the end of the parentheses so that the parser can respond correctly
            parStmt[parenInd - 1].tokenString = ";".to_string();
            parStmt[parenInd - 1].tt = tokenTypeEnum::SEMICOLON;
            parStmt[parenInd - 1].tg = tokenGroup::SYMBOL;


            //Parses the internal part of the parentheses
            let scanned = self.parseExpr(&mut parStmt);                            
            match scanned {
                Ok(expr) => {
                    firstOp = expr;
                },
                Err(reporting) => {
                    println!("Error parsing paren expression: {:?}", reporting);
                    let errMsg = format!("Error parsing paren expr: {:?}", self.reports);
                    return Err(errMsg);
                },
            }
            
            //Prints the extracted and parsed Expr from the parenthese (used for debugging)
            // println!("extracted paren statement:");
            // printTokList(&parStmt);
            // println!("Parsed paren expression: {}", varRef);

            //Drains the expression list so it can continue properly
            curStmt.drain(..parStmt.len());

        } 
        //If the expression is a procedure call, parses all of the parameteres and creates the proc call Expr
        else if (curStmt[0].tt == tokenTypeEnum::PROCEDURE_CALL) {
            //Gets the procedure name
            let procName = curStmt[0].tokenString.clone();
            //If there are parameters being passed
            if (curStmt[1].tt != tokenTypeEnum::R_PAREN) {
                let mut paramInd = 0;
                let mut params: Vec<Expr> = Vec::new();
                let mut paramTokens: Vec<Token> = Vec::new();
                let mut p = 1;
                let mut pToken;
                
                //Finds the end of the parameters
                while p < curStmt.len() {
                    pToken = curStmt[p].clone();
                    if(pToken.tt == tokenTypeEnum::R_PAREN) {
                        break;
                    }

                    paramTokens.push(pToken.clone());
                    p += 1;
                    
                }

                //Adds the parameter parsed to the vector of parameters
                paramTokens.push(curStmt[p].clone());

                //Parses out all of the parameters
                let paramScan = self.parseExpr(&mut paramTokens);
                match paramScan {
                    Ok(expr)=> {
                        params.push(expr);
                    } Err(err) => {
                        return(Err(err));
                    }
                }

                //Prints the extracted parameters (used for debugging)
                // println!("Extracted parameters:");
                // for param in params.clone(){
                //     println!("{}", param);
                // }
                
                //Creates the process reference Expr
                let procCall = Expr::ProcRef((procName), (Some(params)));
                
                //Sets the call properly so it parseExpr can handle the rest of the expression
                firstOp = procCall;
                curStmt.drain(0..p+1);

            } 
            //If there are no parameters in the parameter call
            else {
                //Creates the procedure call Expr and drains the tokenList
                let procCall = Expr::ProcRef((procName), (None));
                firstOp = procCall;
                curStmt.drain(0..1);
            }
        } 
        //If the expression contains a constant
        else if (curStmt[0].tg ==tokenGroup::CONSTANT) {
            //Checks if it is a proper constant and parses it into the proper constant Expr
            let constCheck = Expr::newCon(curStmt[0].clone());
            match constCheck{
                Ok(constExpr) => {
                    firstOp = constExpr;
                }
                Err(msg) => {
                    return Err(msg);
                }
            }
        } 
        //Other types of thing in an expression
        //Attempts to just turn it into an expression but sends an error if not.
        else {
            match &firstOp {
                //This means that the firstOp has not been changes, attempts to create a new Expr with it
                Expr::StringLiteral(s) if s == "NONE" => {
                    let empty:Expr = Expr::StringLiteral(("NONE".to_string())); 
                    let valRef = Expr::new(curStmt[0].tt.clone(), Some(curStmt[0].tokenString.clone()));
                    match valRef {
                        Ok(expr) => {
                            firstOp = expr;
                        } Err(err) => {
                            let errMsg = format!("Error parsing expression on line on line {}: {}", curStmt[0].lineNum.to_string(), err);
                            return(Err(errMsg));
                        }
                    }
                }
                //This means that it was intialized and we can proceed to process the rest of the Expr
                _ => {
                    // println!("Initialized");
                    //continue
                }
            }
        }


        //Print statements for debugging
        // println!("We are doing something with this value: {}", varRef);
        // // // println!("First after variable: {}", curStmt[1].tokenString);
        // println!("Remaining items: {}", curStmt.len().to_string());
        // printTokList(&curStmt);

        //Checks how much is left after the first one is parsed
        //If it is greater than 2, that means there is more to parse, a complex expression
        if(curStmt.len() > 2){
            //Parses out the sections of what is left, attempts to create new operators
            let operand1 = firstOp;
            let operatorRes = Operator::new(curStmt[1].tt.clone());
            let mut operator: Operator;
            //Checks if the operator was parsed properly
            match operatorRes {
                Ok(op) => {
                    operator = op;
                },
                Err(reporting) => {
                    // println!("Error parsing op on line {}: {:?}",curStmt[1].lineNum, reporting);
                    println!("BAD OP {}", curStmt[1].tokenString.clone());
                    let errMsg = format!("Error parsing operator on line {}: {:?}", curStmt[1].lineNum.to_string(), self.reports);
                    return Err(errMsg);
                },
            }

            //Drains some to parse what is left
            let mut subList = curStmt.clone();
            subList.drain(0..2); 
            let mut parsedExpr: Expr;
            //parses the remainder of the expression
            let scanned = self.parseExpr(&mut subList);                            
            let mut headerStmt:Expr;
            //Checks if it was parsed successfully
            match scanned {
                Ok(stmt) => {
                    parsedExpr = stmt;
                },
                Err(reporting) => {
                    // println!("Error parsing expression : {:?}", reporting);
                    let errMsg = format!("Error parsing expression on line {}: {:?}", subList[0].lineNum.clone(), self.reports);
                    return Err(errMsg);
                },
            }

            //Turns the seperate expressions into Boxes for proper expression creation
            let op1Box = Box::new(operand1);
            let op2Box = Box::new(parsedExpr);
            let retExpr = Expr::newOp((op1Box), (operator), (op2Box));

            //Prints the full parsed expression, used for debugging
            // println!("Expression parsed: {}", retExpr);
            // parsedStmt.display(0);

            //Returns the aprsed expression
            return Ok(retExpr);            
        } 
        //If the expression is 2 long, it is a simple expression and can return
        else if (curStmt.len() == 2) {
            // println!("Simple expressions");
            
            if(curStmt[0].tt == tokenTypeEnum::R_PAREN){
                return(Ok(firstOp));
            }

            //Creates the new expression to reutn
            let valueRes = Expr::new(tokenList[0].tt.clone(), Some(tokenList[0].tokenString.clone()));
            let mut valueExpr:Expr; 
            //Checks if it was properly parsed
            match valueRes {
                Ok(expr) => {
                    valueExpr = expr;
                }
                Err(err) => {
                    // println!("Error parsing expression");
                    let errMsg = format!("Error parsing expression on line {}: {}", tokenList[0].lineNum, err);
                    self.reports.reportError(errMsg);
                    return Err("Error with expression".to_string());
                }
            }

            //Drains the token list and returns the parsed expression
            tokenList.drain(0..k);
            return Ok(valueExpr);
        } 
        //This means there is nothing other than the first expression to parse, returns that
        else {
            return(Ok(firstOp));
        }

        
    }

    //This is the outer parse function. It parses sections of the tokenList
    //This will return either a Stmt section of the programAST or an error, 
    //Runs recursively
    pub fn parse(&mut self, tokenList: &mut Vec<Token>) -> Result<Option<Stmt>, String> {
        // println!("Beginning individual parse");

        //Sets up the things that will be used here            
        let tokLen: usize = tokenList.len();
        let mut token = &tokenList[0];
        
        //The main match case, used to determine what should be done with the beginning of each Stmt
        match token.tt {
            //Indicates the beginning of the program
            tokenTypeEnum::PROGRAM => {
                
                //If program is just starting, check it.
                //Checks the first line
                let firstToken = &tokenList[0];
                if let tokenTypeEnum::PROGRAM = firstToken.tt {
                    let thirdToken = &tokenList[2];
                    if let tokenTypeEnum::IS = thirdToken.tt {
                        //Gets the program name
                        let programName: String = tokenList[1].tokenString.clone();                                
                        
                        //Removes the program statement
                        tokenList.drain(0..3);

                        //Finds where the header ends and the body begins
                        let mut beginInt = 0;
                        let mut beginScope = 0;
                        let iterTokList = tokenList.clone();
                        
                        //Finds where the header ends and the body begin
                        for token in iterTokList{
                            if (token.tt == tokenTypeEnum::BEGIN) && (beginScope == 0){
                                break;
                            } else if (token.tt == tokenTypeEnum::PROCEDURE) {
                                beginScope = beginScope + 1;
                                beginInt = beginInt + 1;
                            } else if (token.tt == tokenTypeEnum::END_PROCEDURE){
                                beginInt = beginInt + 1;
                                beginScope = beginScope - 1;
                            } else {
                                beginInt = beginInt + 1;
                            }
                        }

                        //Splits into two lists to parse seperately, the header and the body
                        let mut bodyList = tokenList.split_off(beginInt);

                        //Parses the header
                        let mut newHeader: Vec<Token> = tokenList.iter().cloned().map(|t| t.clone()).collect();
                        let mut headerBlock = Stmt::Block(Vec::new(), tokenList[0].lineNum.clone());
                        let mut headerI = 0;
                        let headerLen = newHeader.len();
                        //Runs through the header and scans it
                        while(!newHeader.is_empty()){
                            //Ensures that this list does not overflow
                            if(headerI > headerLen){
                                self.reports.reportError("Infinite loop in header".to_string());
                                return Err("infinite loop in header".to_string());
                            }
                            
                            //Parses the next statement out of the header
                            let scanned = self.parse(&mut newHeader);                            
                            let mut headerStmt:Stmt;
                            //Checks the result of the scanned Stmt
                            match scanned {
                                //For a properly returned stmt
                                Ok((Some(stmt))) => {
                                    //Prints the parsed header (used for debugging)
                                    // println!("Header statement parsed successfully");
                                    // stmt.display(0);
                                    //Adds the stmt to the vector of header statements
                                    let _ = headerBlock.push_to_block(stmt.clone());
                                    headerI += 1;
                                },
                                //For a stmt that returns properly but is not properly parsed
                                Ok((None)) => {
                                    let errMsg = format!("Error parsing header statement on line {}", newHeader[0].lineNum.clone());
                                    self.reports.reportError(errMsg);
                                    headerI += 1;
                                },
                                //If there is an error parsing the header
                                Err(reporting) => {
                                    let errMsg = format!("Error parsing header statment on line {}: {:?}", newHeader[0].lineNum.clone(), self.reports);
                                    return Err(errMsg);
                                },
                            }
                        }

                        //Creates the body tokenList
                        let mut newBody: Vec<Token> = bodyList.iter().cloned().map(|t| t.clone()).collect();
                        newBody.drain(0..1);
                        

                        let mut bodyBlock = Stmt::Block(Vec::new(), "".to_string());
                        let mut bodyI = 0;
                        let bodyLen = newBody.len();
                        
                        //Goes through the body block and parses the whole guy
                        while(newBody[0].tt != tokenTypeEnum::END_PROGRAM){
                            //Avoids infinite loops
                            if(bodyI > bodyLen){
                                
                                self.reports.reportError("No End Program phrase detected. Program must end with 'End Program' ".to_string());
                                return Err("No end program".to_string());
                            }

                            //Parses the body list one stmt at a time
                            let scanned = self.parse(&mut newBody);                            
                            let mut bodyStmt:Stmt;
                            //Checks that the stmt returned ok
                            match scanned {
                                Ok((Some(stmt))) => {
                                    let _ = bodyBlock.push_to_block(stmt.clone());
                                    bodyI = bodyI + 1;
                                },
                                Ok((None)) => {
                                    let errMsg = format!("Error parsing body statement");
                                    self.reports.reportError(errMsg);
                                    bodyI = bodyI + 1;
                                },
                                Err(reporting) => {
                                    let errMsg = format!("Error parsing body statment on line {}: {:?}", newBody[0].lineNum.clone(), self.reports);
                                    return Err(errMsg);
                                },
                            }
                            
                        }

                        //Prints the finished parsed body (debugging)
                        // println!("Finished parsing body: ");
                        // bodyBlock.display(0);

                        // Turns the statements into boxes
                        let boxHeader: Box<Stmt> = Box::new(headerBlock);
                        let boxBody: Box<Stmt> = Box::new(bodyBlock);

                        //Creates the programAst Stmt
                        let programAst = Stmt::Program(programName.clone(), boxHeader, boxBody, "0".to_string());
                        // programAst.display(0);

                        //Returns the parsed program
                        return Ok((Some(programAst)));

                    } 
                    //If the program declaration is incorrect
                    else {
                        self.reports.reportError("Program declaration incorrect. \n Program must start with: 'program [Program name] is'".to_string());
                        // println!("Error with program delcaration");
                        return Err("Error with program declaration".to_string());
                    }
                } 
                //If there is an error in the program delcaration
                else {
                    self.reports.reportError("Program declaration incorrect. \n Program must start with: 'program [Program name] is'".to_string());
                    // println!("Error with program delcaration");
                    return Err("Error with program declaration".to_string());
                }
                
            }
            //Indicates the "variable" keyword has been found
            tokenTypeEnum::VARIABLE => {
                if(self.scope != 0) {
                        //Initializes the return stmt
                    let mut retStmt:Stmt;

                    //Sets up the variables for iteratins 
                    let mut k = 1;
                    let mut nextTok = &tokenList[k];
                    let mut curStmt: Vec<&Token> = vec![];
                    curStmt.push(token);
                    //Finds the end of the statement
                    while nextTok.tt != tokenTypeEnum::SEMICOLON {
                        curStmt.push(nextTok);
                        k = k + 1;
                        nextTok = &tokenList[k];
                    }
                    curStmt.push(nextTok);

                    //Extracts the name of the variable being referenced
                    let varName = &curStmt[1].tokenString;

                    //Prints the list of the variable statment (debugging)
                    // println!("\nCurrent variable declaration name: {}", varName);
                    // for token in &curStmt {
                    //     println!("< \"{}\" , {}, {} >", token.tokenString, token.tt.to_string(), token.lineNum);
                    // }

                    //Checks the validity of the statement, sending errors if it is incorrect
                    if curStmt[2].tt != tokenTypeEnum::COLON {
                        let errMsg = format!("In line: {}, Array variable declaration incorrect. \n Must be in this format: 'variable [Variable name] : [variable type]'", curStmt[3].lineNum,);
                        self.reports.reportError(errMsg);
                        return Err("Error with variable declaration".to_string());
                    } 
                    //Checks more of the statement
                    else {
                        if (curStmt[4].tt != tokenTypeEnum::SEMICOLON) {
                            if curStmt[4].tt != tokenTypeEnum::L_BRACKET {
                                let errMsg = format!("In line: {}, Array variable declaration incorrect. \n Must be in this format: 'variable [Variable name] : integer[arraySize]'", curStmt[3].lineNum.clone());
                                self.reports.reportError(errMsg);
                                return Err("Error with variable declaration".to_string());
                            } else {
                                if curStmt[3].tokenString == "integer" {
                                    if curStmt[5].tt == tokenTypeEnum::INT {
                                        let arSizeStr = curStmt[5].tokenString.clone();
                                        if let Ok(arSize) = arSizeStr.parse::<usize>() {
                                            let newVar = Stmt::VarDecl(varName.clone(), VarType::IntArray(arSize.try_into().unwrap()), curStmt[0].lineNum.clone());
                                            retStmt = newVar;
                                        } else {
                                            self.reports.reportError(format!(
                                                "In line: {}, Invlaid array size", 
                                                curStmt[3].lineNum, 
                                            ));
                                            return Err("Error with variable declaration".to_string());
                                        }
                                    } else {
                                        self.reports.reportError(format!(
                                            "In line: {}, Array variable declaration incorrect. \n Must be in this format: 'variable [Variable name] : integer[arraySize]'", 
                                            curStmt[3].lineNum, 
                                        ));
                                        return Err("Error with variable declaration".to_string());
                                    }
                                } else {
                                    self.reports.reportError(format!(
                                        "In line: {}, '{}' is not a valid variable type", 
                                        curStmt[3].lineNum, 
                                        curStmt[3].tokenString
                                    ));
                                    return Err("Error with variable declaration".to_string());
                                }
                            }
                        } else if curStmt[3].tokenString == "string" {
                            let newVar = Stmt::VarDecl(varName.clone(), VarType::Str, curStmt[3].lineNum.clone());
                            retStmt = newVar;
                        } else if curStmt[3].tokenString == "integer" {
                            let newVar = Stmt::VarDecl(varName.clone(), VarType::Int, curStmt[3].lineNum.clone());
                            retStmt = newVar;

                        }  else if curStmt[3].tokenString == "bool" {
                            let newVar = Stmt::VarDecl(varName.clone(), VarType::Bool, curStmt[3].lineNum.clone());
                            retStmt = newVar;

                        }  else if curStmt[3].tokenString == "float" {
                            let newVar = Stmt::VarDecl(varName.clone(), VarType::Float, curStmt[3].lineNum.clone());
                            retStmt = newVar;
                        } else {
                            self.reports.reportError(format!(
                                "In line: {}, '{}' is not a valid variable type", 
                                curStmt[3].lineNum, 
                                curStmt[3].tokenString
                            ));
                            return Err("Error with variable declaration".to_string());
                        }
                    }

                    // let newVar = Stmt::VarDecl(varName, );
                    
                    k = k + 1;
                    // i = k;

                    tokenList.drain(0..k);
                    return Ok(Some(retStmt));
                    // println!("Variable initialized");}
                
                }
                else {
                    let mut retStmt:Stmt;
                    
                    let mut k = 1;
                    let mut nextTok = &tokenList[k];
                    let mut curStmt: Vec<&Token> = vec![];
                    while nextTok.tt != tokenTypeEnum::SEMICOLON {
                        curStmt.push(nextTok);
                        k = k + 1;
                        nextTok = &tokenList[k];

                    }
                    curStmt.push(nextTok);
                    let varName = &curStmt[0].tokenString;

                    
                    let globalToken = Token::new(tokenTypeEnum::GLOBAL, "global".to_string(), curStmt[0].lineNum.clone(), tokenGroup::KEYWORD);
                    curStmt.insert(0, &globalToken);

                    //Error checking
                    if curStmt[2].tt != tokenTypeEnum::COLON {
                        self.reports.reportError(format!(
                            "In line: {}, Array variable declaration incorrect. \n Must be in this format: 'variable [Variable name] : [variable type]'", 
                            curStmt[3].lineNum, 
                        ));
                        return Err("Error with global variable declaration".to_string());
                    } else {
                        if (curStmt[4].tt != tokenTypeEnum::SEMICOLON) {
                            if curStmt[4].tt != tokenTypeEnum::L_BRACKET {
                                self.reports.reportError(format!(
                                    "In line: {}, Array variable declaration incorrect. \n Must be in this format: 'variable [Variable name] : integer[arraySize]'", 
                                    curStmt[3].lineNum, 
                                ));
                                return Err("Error with global variable declaration".to_string());
                            } else {
                                if curStmt[3].tokenString == "integer" {
                                    if curStmt[5].tt == tokenTypeEnum::INT {
                                        let arSizeStr = curStmt[5].tokenString.clone();
                                        if let Ok(arSize) = arSizeStr.parse::<usize>() {
                                            let newVar = Stmt::GlobVarDecl(varName.clone(), VarType::IntArray(arSize.try_into().unwrap()), curStmt[3].lineNum.clone());
                                            retStmt = newVar;
                                        } else {
                                            self.reports.reportError(format!(
                                                "In line: {}, Invlaid array size", 
                                                curStmt[3].lineNum, 
                                            ));
                                            return Err("Error with variable declaration".to_string());
                                        }
                                    } else {
                                        self.reports.reportError(format!(
                                            "In line: {}, Array variable declaration incorrect. \n Must be in this format: 'variable [Variable name] : integer[arraySize]'", 
                                            curStmt[3].lineNum, 
                                        ));
                                        return Err("Error with variable declaration".to_string());
                                    }
                                } else {
                                    self.reports.reportError(format!(
                                        "In line: {}, '{}' is not a valid variable type", 
                                        curStmt[3].lineNum, 
                                        curStmt[3].tokenString
                                    ));
                                    return Err("Error with variable declaration".to_string());
                                }
                            }
                        } else if curStmt[3].tokenString == "string" {
                            let newVar = Stmt::GlobVarDecl(varName.clone(), VarType::Str, curStmt[3].lineNum.clone());
                            retStmt = newVar;
                        } else if curStmt[3].tokenString == "integer" {
                            let newVar = Stmt::GlobVarDecl(varName.clone(), VarType::Int, curStmt[3].lineNum.clone());
                            retStmt = newVar;

                        }  else if curStmt[3].tokenString == "bool" {
                            let newVar = Stmt::GlobVarDecl(varName.clone(), VarType::Bool, curStmt[3].lineNum.clone());
                            retStmt = newVar;

                        }  else if curStmt[3].tokenString == "float" {
                            let newVar = Stmt::GlobVarDecl(varName.clone(), VarType::Float, curStmt[3].lineNum.clone());
                            retStmt = newVar;
                        } else {
                            self.reports.reportError(format!(
                                "In line: {}, '{}' is not a valid variable type", 
                                curStmt[3].lineNum, 
                                curStmt[3].tokenString
                            ));
                            return Err("Error with variable declaration".to_string());
                        }
                    }

                    
                    k = k + 1;

                    //Adjusts the tokenList and returns the global variable declaration
                    tokenList.drain(0..k);
                    return Ok(Some(retStmt));
                }
                
            }
            //Indicates that a global variable or procedure is being defined
            tokenTypeEnum::GLOBAL => {
                let mut retStmt:Stmt;
                
                let mut k = 1;
                let mut nextTok = &tokenList[k];
                let mut curStmt: Vec<&Token> = vec![];
                while nextTok.tt != tokenTypeEnum::SEMICOLON {
                    curStmt.push(nextTok);
                    k = k + 1;
                    nextTok = &tokenList[k];

                }
                curStmt.push(nextTok);
                let varName = &curStmt[1].tokenString;



                

                //Error checking
                if curStmt[2].tt != tokenTypeEnum::COLON {
                    self.reports.reportError(format!(
                        "In line: {}, Array variable declaration incorrect. \n Must be in this format: 'variable [Variable name] : [variable type]'", 
                        curStmt[3].lineNum, 
                    ));
                    return Err("Error with global variable declaration".to_string());
                } else {
                    if (curStmt[4].tt != tokenTypeEnum::SEMICOLON) {
                        if curStmt[4].tt != tokenTypeEnum::L_BRACKET {
                            self.reports.reportError(format!(
                                "In line: {}, Array variable declaration incorrect. \n Must be in this format: 'variable [Variable name] : integer[arraySize]'", 
                                curStmt[3].lineNum, 
                            ));
                            return Err("Error with global variable declaration".to_string());
                        } else {
                            if curStmt[3].tokenString == "integer" {
                                if curStmt[5].tt == tokenTypeEnum::INT {
                                    let arSizeStr = curStmt[5].tokenString.clone();
                                    if let Ok(arSize) = arSizeStr.parse::<usize>() {
                                        let newVar = Stmt::GlobVarDecl(varName.clone(), VarType::IntArray(arSize.try_into().unwrap()), curStmt[3].lineNum.clone());
                                        retStmt = newVar;
                                    } else {
                                        self.reports.reportError(format!(
                                            "In line: {}, Invlaid array size", 
                                            curStmt[3].lineNum, 
                                        ));
                                        return Err("Error with variable declaration".to_string());
                                    }
                                } else {
                                    self.reports.reportError(format!(
                                        "In line: {}, Array variable declaration incorrect. \n Must be in this format: 'variable [Variable name] : integer[arraySize]'", 
                                        curStmt[3].lineNum, 
                                    ));
                                    return Err("Error with variable declaration".to_string());
                                }
                            } else {
                                self.reports.reportError(format!(
                                    "In line: {}, '{}' is not a valid variable type", 
                                    curStmt[3].lineNum, 
                                    curStmt[3].tokenString
                                ));
                                return Err("Error with variable declaration".to_string());
                            }
                        }
                    } else if curStmt[3].tokenString == "string" {
                        let newVar = Stmt::GlobVarDecl(varName.clone(), VarType::Str, curStmt[3].lineNum.clone());
                        retStmt = newVar;
                    } else if curStmt[3].tokenString == "integer" {
                        let newVar = Stmt::GlobVarDecl(varName.clone(), VarType::Int, curStmt[3].lineNum.clone());
                        retStmt = newVar;

                    }  else if curStmt[3].tokenString == "bool" {
                        let newVar = Stmt::GlobVarDecl(varName.clone(), VarType::Bool, curStmt[3].lineNum.clone());
                        retStmt = newVar;

                    }  else if curStmt[3].tokenString == "float" {
                        let newVar = Stmt::GlobVarDecl(varName.clone(), VarType::Float, curStmt[3].lineNum.clone());
                        retStmt = newVar;
                    } else {
                        self.reports.reportError(format!(
                            "In line: {}, '{}' is not a valid variable type", 
                            curStmt[3].lineNum, 
                            curStmt[3].tokenString
                        ));
                        return Err("Error with variable declaration".to_string());
                    }
                }

                
                k = k + 1;

                //Adjusts the tokenList and returns the global variable declaration
                tokenList.drain(0..k);
                return Ok(Some(retStmt));
                
            }           
            //This means an identifier (like a variable name) is found, probably a variable assignment
            tokenTypeEnum::IDENTIFIER => {
                //Initializes the variable that is being referenced first
                let mut varRef:Expr;
                //Initializes the return statement (I DONT THINK THIS IS NEEDED)
                let mut retStmt:Stmt;

                //Initializes values for finding the end of the expression
                let mut k = 0;
                let mut nextTok = &tokenList[k];
                let mut curStmt: Vec<&Token> = vec![];
                
                //Finds the end of the expression
                while k < tokenList.len() {
                    let nextTok = &tokenList[k];
                    curStmt.push(nextTok);
                
                    if (nextTok.tt == tokenTypeEnum::SEMICOLON) {
                        break;
                    }
                
                    k += 1;
                }


                //Checks if the first value is an array reference or not
                if(curStmt[1].tt == tokenTypeEnum::L_BRACKET) {
                    let varName = curStmt[0].tokenString.clone();
                    //Finds the end of the brackets
                    let mut brackInd = 0;
                    let mut nextIndToken = &curStmt[brackInd];
                    let mut indexList: Vec<Token> = vec![];
                    
                    //Finds the end of the index statement
                    while brackInd < curStmt.len() {
                        let nextTok = curStmt[brackInd];
                        indexList.push(nextTok.clone());
                    
                        if (nextTok.tt == tokenTypeEnum::R_BRACKET) {
                            break;
                        }
                    
                        brackInd += 1;
                    }

                    //If the end of the index was not found, error
                    if brackInd == curStmt.len() {
                        let errMsg = format!("THIS IS AN ERROR in finding end of array index on line {}", curStmt[0].lineNum.clone());
                        tokenList.drain(..k+1);
                        self.reports.reportError(errMsg.clone());
                        return Err(errMsg);
                    }

                    //Removes the index assignment so there is just the ] left for proper parsing of index expression
                    indexList.drain(0..2);

                    //Parses the expression
                    let parsedExpr = self.parseExpr(&mut indexList);
                    let mut indexExpr: Expr;

                    //Extracts the resulting expression
                    match parsedExpr {
                        Ok(expr) => {
                            indexExpr = expr;
                        }
                        Err(err) => {
                            let errMsg = format!("Error on line {}: {}", tokenList[0].lineNum, err);
                            self.reports.reportError(errMsg);
                            return Err("Error with expression".to_string());
                        }
                    }

                    let indexBox = Box::new(indexExpr);
                    varRef = Expr::ArrayRef((varName), (indexBox));

                    //Removes the array reference so there is just the ] left
                    curStmt.drain(0..brackInd);
                } 
                //If the guy is a variable reference but not an array variable
                else if (curStmt[0].tg == tokenGroup::VARIABLE){
                    //If not an array
                    varRef = Expr::VarRef(curStmt[0].tokenString.clone());

                } 
                else {
                    let errMsg = format!("Invalid identifier on line {}", curStmt[0].lineNum.clone());
                    self.reports.reportError(errMsg.clone());
                    return Err(errMsg.clone());
                }

                //Used for debugging
                // println!("We are doing something with this variable: {}", varRef);
                // println!("First after variable: {}", curStmt[1].tokenString);

                // //Looks ahead to see what comes next and parses accordingly
                match curStmt[1].tg {
                    tokenGroup::OPERATOR => {
                        match curStmt[1].tt {
                            tokenTypeEnum::SET_EQUALS =>{
                                //Gets the var name
                                let varName = curStmt[0].tokenString.clone();

                                //Initializes a list for new value tokens
                                let mut newValueList: Vec<Token> = curStmt.iter().cloned().map(|t| t.clone()).collect();
                                newValueList.drain(..2);

                                //Parses the new value expression
                                let parsedExpr = self.parseExpr(&mut newValueList);
                                let mut newValueExpr: Expr;

                                //Extracts the resulting expression
                                match parsedExpr {
                                    Ok(expr) => {
                                        newValueExpr = expr;
                                    }
                                    Err(err) => {
                                        let errMsg = format!("Error on line {}: {}", tokenList[0].lineNum, err);
                                        self.reports.reportError(errMsg);
                                        return Err("Error with expression".to_string());
                                    }
                                }
                                
                                //Creates the variable assignment statement
                                let varAssignment = Stmt::Assign((varRef), (newValueExpr), curStmt[0].lineNum.clone());
                                tokenList.drain(..k+1);
                                return Ok(Some(varAssignment));

                            }
                            //Something other than an assignment
                            _ => {
                                let mut newValueList: Vec<Token> = curStmt.iter().cloned().map(|t| t.clone()).collect();

                                //Parses the expression
                                let newExpr = self.parseExpr(&mut newValueList);
                                let retVal: Expr;
                                match newExpr {
                                    Ok(expr) => {
                                        retVal = expr;
                                    }
                                    Err(err) => {
                                        let errMsg = format!("Error on line {}: {}", tokenList[0].lineNum, err);
                                        self.reports.reportError(errMsg);
                                        return Err("Error with expression".to_string());
                                    }
                                }

                                let exprStmt = Stmt::Expr((retVal), curStmt[0].lineNum.clone());
                                tokenList.drain(..k+1);
                                return Ok(Some(exprStmt));
                            }
                        }
                    }
                    //If it is not an operator, it is unaccounted, which is an error
                    _ => {
                        let errMsg = format!("Error: Found {}, of group {} on line {} when an operator was expected", curStmt[1].tokenString, curStmt[1].tg, curStmt[1].lineNum);
                        self.reports.reportError(errMsg);
                        return Err("Unexpected token found".to_string());
                    }
                }                
            }
            //The declaration of an if statement
            tokenTypeEnum::IF => {
                //Sets up the if statement
                let mut k = 0;
                let mut nextTok = &tokenList[k];
                let mut curStmt: Vec<Token> = vec![];
                let mut ifInd = 0;
                let ifLen = tokenList.len();
            
                // Finds the end of the if
                while nextTok.tt != tokenTypeEnum::END_IF {
                    if(ifInd > ifLen) {
                        let errMsg = format!("For If on line {}, no end if found", token.lineNum);
                        self.reports.reportError(errMsg);
                        return Err("No end if".to_string());
                    }
                    curStmt.push(nextTok.clone());
                    k = k + 1;
                    ifInd = ifInd + 1;
                    nextTok = &tokenList[k];
                }
                curStmt.push(nextTok.clone());

                //Finds the end of the if condition
                let mut condInt;
                let mut ifCondition: Expr;
                // // Extract the condition if it exists
                if curStmt[1].tt == tokenTypeEnum::L_PAREN {
                    let mut j = 1;
                    let mut nextTok = &curStmt[j];
                    let mut condStmt: Vec<Token> = vec![];
                
                    // Finds the end of the condition by findind the then
                    while nextTok.tt != tokenTypeEnum::THEN {
                        condStmt.push(nextTok.clone());
                        j = j + 1;
                        nextTok = &curStmt[j];
                    }
                    condInt = j;

                    //Parses the if condition into an expression
                    condStmt.drain(0..1);
                    let mut parsedExpr: Expr;
                    let scanned = self.parseExpr(&mut condStmt);                            
                    let mut headerStmt:Expr;
                    //Checks if it was good
                    match scanned {
                        Ok(stmt) => {
                            parsedExpr = stmt;   
                        },
                        Err(err) => {
                            let errMsg = format!("Error parsing if condition: {}", err);
                            self.reports.reportError(errMsg);
                            return Err("Error with if condition".to_string());
                        },
                    }
                    ifCondition = parsedExpr;
                } else {
                    let errMsg = format!("Error in if statement on line: {},\nIf statement declarations must follow this format: if([condition]) then", token.lineNum);
                    self.reports.reportError(errMsg);
                    return Err("Error with if condition".to_string());
                }

                //Checks for an else statement
                let mut elseInd: usize = 0;
                let mut holder = 0;
                curStmt.drain(0..condInt+1);
                for token in &curStmt {
                    if(token.tt == tokenTypeEnum::ELSE){
                        elseInd = holder;
                    }
                    holder = holder + 1;
                }

                //If an else was found
                if elseInd != 0 {
                    let mut ifList = curStmt.clone();
                    ifList.drain(elseInd..);
                    
                    //Parses the if body
                    let mut newIf: Vec<Token> = ifList.iter().cloned().map(|t| t.clone()).collect();
                    let mut ifBlock = Stmt::Block(Vec::new(), curStmt[0].lineNum.clone());
                    let mut ifI = 0;
                    let ifLen = newIf.len();
                    while(!newIf.is_empty()){
                        if(ifI > ifLen){
                            self.reports.reportError("Infinite loop in if statement".to_string());
                            return Err("infinite loop in if".to_string());
                        }
                        ifI = ifI + 1;
                        
                        //Scans each piece of the if body
                        let scanned = self.parse(&mut newIf);                            
                        let mut ifStmt:Stmt;
                        match scanned {
                            Ok((Some(stmt))) => {
                                let _ = ifBlock.push_to_block(stmt.clone());
                            },
                            Ok((None)) => {
                                //continue because this shouldnt happen
                            },
                            Err(reporting) => {
                                let errMsg = format!("Error parsing if: {:?}", self.reports);
                                return Err(errMsg);
                            },
                        }
                    }


                    //Parses the else block
                    let mut elseList = curStmt.split_off(elseInd);                    
                    let mut newElse: Vec<Token> = elseList.iter().cloned().map(|t| t.clone()).collect();
                    newElse.drain(0..1);
                    newElse.drain(newElse.len() - 1..);
                    // println!("First in else: {}", newElse[0].tokenString);
                    let mut elseBlock = Stmt::Block(Vec::new(), curStmt[0].lineNum.clone());
                    let mut elseI = 0;
                    let elseLen = newElse.len();
                    while(!newElse.is_empty()){
                        if(elseI > elseLen){
                            self.reports.reportError("Infinite loop in else".to_string());
                            return Err("infinite loop in else".to_string());
                        }
                        let scanned = self.parse(&mut newElse);                            
                        let mut elseStmt:Stmt;
                        match scanned {
                            Ok((Some(stmt))) => {
                                let _ = elseBlock.push_to_block(stmt.clone());
                                elseI = elseI + 1;
                            },
                            Ok((None)) => {
                                //continue because this shouldnt happen
                                elseI = elseI + 1;

                            },
                            Err(reporting) => {
                                let errMsg = format!("Error parsing else: {:?}", self.reports);
                                return Err(errMsg);
                            },
                        }
                        
                    }

                    //Converts the blocks to boxes
                    let ifBox = Box::new(ifBlock);
                    let elseBox = Box::new(elseBlock);

                    //Finishes up and returns
                    let retStmt = Stmt::If(ifCondition, ifBox, Some(elseBox), curStmt[0].lineNum.clone());
                    tokenList.drain(0..k+2);
                    return Ok(Some(retStmt));
                } 
                //If there is no else
                else {
                    //Sets up stuff
                    let mut ifList = curStmt.clone();
                    ifList.drain(ifList.len() - 1..);                    

                    //Parses the header
                    let mut newIf: Vec<Token> = ifList.iter().cloned().map(|t| t.clone()).collect();
                    let mut ifBlock = Stmt::Block(Vec::new(), curStmt[0].lineNum.clone());
                    let mut ifI = 0;
                    let ifLen = newIf.len();
                    //parses the if body
                    while(!newIf.is_empty()){
                        if(ifI > ifLen){
                            self.reports.reportError("Infinite loop in if statement".to_string());
                            return Err("infinite loop in if".to_string());
                        }
                        ifI = ifI + 1;
                        let scanned = self.parse(&mut newIf);                            
                        let mut ifStmt:Stmt;
                        match scanned {
                            Ok((Some(stmt))) => {
                                let _ = ifBlock.push_to_block(stmt.clone());
                            },
                            Ok((None)) => {
                                //continue because this shouldnt happen
                            },
                            Err(reporting) => {
                                let errMsg = format!("Error parsing if: {:?}", self.reports);
                                return Err(errMsg);
                            },
                        }
                    }

                    //Converts the blocks to boxes
                    let ifBox = Box::new(ifBlock);

                    //Finishes up and returns
                    let retStmt = Stmt::If(ifCondition, ifBox, None, curStmt[0].lineNum.clone());
                    tokenList.drain(0..k+2);
                    return Ok(Some(retStmt));
                }
            }
            //The declaration of a for loop
            tokenTypeEnum::FOR => {
                //Sets up stuff
                let mut k = 0;
                let mut nextTok = &tokenList[k];
                let mut curStmt: Vec<Token> = vec![];
                let mut forInd = 0;
                let forLen = tokenList.len();
            
                // Finds the end of the for
                while nextTok.tt != tokenTypeEnum::END_FOR {
                    if(forInd > forLen) {
                        let errMsg = format!("For for on line {}, no end for found", token.lineNum);
                        self.reports.reportError(errMsg);
                        return Err("No end for".to_string());
                    }
                    curStmt.push(nextTok.clone());
                    k = k + 1;
                    forInd = forInd + 1;
                    nextTok = &tokenList[k];
                }
                curStmt.push(nextTok.clone());

                let mut condInt;
                let mut forDecl: Stmt;
                let mut forCond: Expr;
                // // Extract the condition if it exists
                if curStmt[1].tt == tokenTypeEnum::L_PAREN {
                    let mut j = 1;
                    let mut nextTok = &curStmt[j];
                    let mut condStmt: Vec<Token> = vec![];
                
                    // Finds the end of the condition by findind the paren
                    while nextTok.tt != tokenTypeEnum::R_PAREN {
                        condStmt.push(nextTok.clone());
                        j = j + 1;
                        nextTok = &curStmt[j];
                    }
                    condInt = j;

                    condStmt.push(nextTok.clone());
                    condStmt.drain(0..1);

                    //Parses the for loop condition
                    let mut parsedStmt: Stmt = Stmt::StringLiteral("NONE".to_string(), "0".to_string());
                    let scanned = self.parse(&mut condStmt);                            
                    match scanned {
                        Ok((Some(stmt))) => {
                            parsedStmt = stmt;
                        },
                        Ok((None)) => {
                            //this shoudlnt happen
                        },
                        Err(err) => {
                            let errMsg = format!("Error parsing for condition: {}", err);
                            self.reports.reportError(errMsg);
                            return Err("Error with for condition".to_string());
                        },
                    }
                    forDecl = parsedStmt;

                    //Parses the for condition
                    let scanned = self.parseExpr(&mut condStmt);                            
                    match scanned {
                        Ok((stmt)) => {
                            forCond = stmt;
                        },
                        Err(err) => {
                            let errMsg = format!("Error parsing for condition: {}", err);
                            self.reports.reportError(errMsg);
                            return Err("Error with for condition".to_string());
                        },
                    }

                } 
                //If there is an error in the for loop
                else {
                    let errMsg = format!("Error in FOR statement on line: {},\nFor statement declarations must follow this format: for([condition]) then", token.lineNum);
                    self.reports.reportError(errMsg);
                    return Err("Error with for condition".to_string());
                }

                //Modifies the forList
                let mut forList = curStmt.clone();
                forList.drain(0..condInt+1);
                let mut newForLen = forList.len() - 1;
                forList.drain(newForLen..);             

                //Parses the for body
                let mut newFor: Vec<Token> = forList.iter().cloned().map(|t| t.clone()).collect();
                let mut forBlock = Stmt::Block(Vec::new(), tokenList[0].lineNum.clone());
                let mut ifI = 0;
                let ifLen = newFor.len();
                while(!newFor.is_empty()){
                    if(ifI > ifLen){
                        self.reports.reportError("Infinite loop in if statement".to_string());
                        return Err("infinite loop in if".to_string());
                    }
                    ifI = ifI + 1;
                    let scanned = self.parse(&mut newFor);                            
                    let mut ifStmt:Stmt;
                    match scanned {
                        Ok((Some(stmt))) => {
                            let _ = forBlock.push_to_block(stmt.clone());
                        },
                        Ok((None)) => {
                            //continue as this shouldnt happen
                        },
                        Err(reporting) => {
                            let errMsg = format!("Error parsing if: {:?}", self.reports);
                            return Err(errMsg);
                        },
                    }
                }

                //Converts the blocks to boxes
                let forBox = Box::new(forBlock);

                //Finishes up and returns
                let retStmt = Stmt::For(forDecl.into(), forCond, forBox, tokenList[0].lineNum.clone());
                tokenList.drain(0..k+2);
                return Ok(Some(retStmt));
            }
            //When a procedure is called but not assigned to something            
            tokenTypeEnum::PROCEDURE => {
                self.scope += 1;
                // println!("Parsing procedure");
                //Finds the end of the procedure
                let mut retStmt:Stmt;
                let mut k = 1;
                let mut nextTok = &tokenList[1];
                let mut scope = 0;
                let mut curStmt: Vec<Token> = vec![];
                curStmt.push(token.clone());
                while (k < tokenList.len()) {
                    if(nextTok.tt == tokenTypeEnum::PROCEDURE){
                        scope = scope + 1;
                    } else if ((nextTok.tt == tokenTypeEnum::END_PROCEDURE)){
                        if(scope != 0){
                            scope = scope - 1;
                        } else {
                            break;
                        }
                        

                    } 
                    curStmt.push(nextTok.clone());
                    k = k + 1;
                    nextTok = &tokenList[k];

                }
                curStmt.push(nextTok.clone());
                
                //Gets the procedure return type
                let procId = &curStmt[1].tokenString.clone();
                let procType = VarType::new(&curStmt[3].tokenString);    
                let mut procedureType:VarType;
                // Gets the procedure type
                match procType {
                    Ok(varType) => {
                        procedureType = varType;
                    }
                    Err(err) => {
                        let errMsg = format!("Error determining procedure type: {}", err);
                        self.reports.reportError(errMsg.clone());
                        return Err("Error with procedure type".to_string());
                    }
                }

                //Initialized param stuff
                let mut paramList = Stmt::Block(Vec::new(), curStmt[0].lineNum.clone());
                let mut j = 4;
                //Finds and extracts the parameters
                if(curStmt[3].tt != tokenTypeEnum::PROCEDURE_CALL){
                    let errMsg = format!("Invalid procedure declaration: {} on line {}", &curStmt[4].tt, &curStmt[4].lineNum);
                    self.reports.reportError(errMsg.clone());
                    return Err("Error with procedure call".to_string());
                } 
                //Finds the end of the procedure call
                else {
                    let mut nextTok = &curStmt[j];
                    let mut paramTokens: Vec<Token> = vec![];
                    let decLine = curStmt[4].lineNum.clone();
                    while nextTok.tt != tokenTypeEnum::R_PAREN  {
                        if(nextTok.lineNum != decLine){
                            let errMsg = format!("Error with procedure reference on line {}, no closing parentheses found", curStmt[0].lineNum.clone());
                            self.reports.reportError(errMsg.clone());
                            return Err("Error with procedure reference".to_string());                            
                        } else {
                            paramTokens.push(nextTok.clone());
                            j = j + 1;
                            nextTok = &curStmt[j];
                        }
                    }

                    //Parses each parameter
                    let mut curParam: Vec<Token> = vec![];
                    //Parses each of the lists of tokens that make up parameters
                    for curToken in &paramTokens {
                        if(curToken.tt == tokenTypeEnum::COMMA) {
                            //Parse the parameters
                            let tokenString: String = ";".to_string();
                            let semicolon = Token::new(crate::tokenTypeEnum::SEMICOLON,tokenString, decLine.to_string(), tokenGroup::SYMBOL);
                            curParam.push(semicolon.clone());
                            let mut newCurParam: Vec<Token> = curParam.iter().cloned().map(|t| t.clone()).collect();
                            let scanParam = self.parse(&mut newCurParam);
                            let mut paramStmt: Stmt;
                            match scanParam {
                                Ok((Some(stmt))) => {
                                    paramStmt = stmt;
                                    let _ = paramList.push_to_block(paramStmt);
                                },
                                Ok((None)) => {
                                    let errMsg = format!("In line: {}, Error with parameter", curStmt[0].lineNum);
                                    self.reports.reportError(errMsg.clone());
                                    let errMsg = format!("Error with procedure statement on line {}", curStmt[0].lineNum.clone());
                                    return Err(errMsg);
                                },
                                Err(reporting) => {
                                    let errMsg = format!("In line: {}, Error with parameter", curStmt[0].lineNum);
                                    self.reports.reportError(errMsg.clone());
                                    let errMsg = format!("Error with procedure statement on line {}", curStmt[0].lineNum.clone());
                                    return Err(errMsg);
                                },
                            }
                            curParam = vec![];
                        } else {
                            let _ = &curParam.push(curToken.clone());
                        }
                    }
                    
                    //Parses each parameter set of tokens into stmts
                    if((paramTokens.len().clone() as i32) != 0){
                        //Parse the parameter
                        let tokenString: String = ";".to_string();
                        let semicolon = Token::new(crate::tokenTypeEnum::SEMICOLON,tokenString, decLine.to_string(), tokenGroup::SYMBOL);
                        curParam.push(semicolon.clone());
                        let mut newCurParam: Vec<Token> = curParam.iter().cloned().map(|t| t.clone()).collect();
                        let scanParam = self.parse(&mut newCurParam);
                        let mut paramStmt: Stmt;
                        match scanParam {
                            Ok((Some(stmt))) => {
                                paramStmt = stmt; 
                                let _ = paramList.push_to_block(paramStmt);
                            },
                            Ok((None)) => {
                                self.reports.reportError(format!(
                                    "In line: {}, Error with parameter", curStmt[0].lineNum
                                ));
                                return Err("Error with parsing parameters".to_string());
                            },
                            Err(reporting) => {
                                self.reports.reportError(format!("In line: {}, Error with condition", curStmt[0].lineNum));
                                return(Err("Error with parameter".to_string()));
                            },
                        }
                    }
                }

                //Displays all the params (for debugging)
                // println!("All Params:");
                // paramList.display(0);

                //Modifies the current list
                curStmt.drain(0..j+1);

                //Finds where the header ends and the body begins
                let mut beginInt = 0;
                let mut beginScope = 0;
                let iterTokList = curStmt.clone();
                for token in iterTokList{
                    if (token.tt == tokenTypeEnum::BEGIN) && (beginScope == 0){
                        break;
                    } else if (token.tt == tokenTypeEnum::PROCEDURE) {
                        beginScope = beginScope + 1;
                        beginInt = beginInt + 1;
                    } else if (token.tt == tokenTypeEnum::END_PROCEDURE){
                        beginInt = beginInt + 1;
                        beginScope = beginScope - 1;
                    } else {
                        beginInt = beginInt + 1;
                    }
                }

                //Splits into two lists to parse seperately
                let mut bodyList = curStmt.split_off(beginInt);

                //Parses the header
                let mut newHeader: Vec<Token> = curStmt.iter().cloned().map(|t| t.clone()).collect();
                let mut headerBlock = Stmt::Block(Vec::new(), tokenList[0].lineNum.clone());
                let mut headerI = 0;

                let headerLen = newHeader.len();
                while(!newHeader.is_empty()){
                    if(headerI > headerLen){
                        self.reports.reportError("Infinite loop in procedure header".to_string());
                        return Err("infinite loop in procedure header".to_string());
                    }
                    let scanned = self.parse(&mut newHeader);                            
                    let mut headerStmt:Stmt;
                    match scanned {
                        Ok((Some(stmt))) => {
                            let _ = headerBlock.push_to_block(stmt.clone());
                        },
                        Ok((None)) => {
                            //continue becuase this shoudlnt happen
                        },
                        Err(reporting) => {
                            let errMsg = format!("Error parsing header: {:?}", self.reports);
                            self.reports.reportError(errMsg.clone());
                            return Err("Error parsing procedure header".to_string());
                        },
                    }
                }     
                
                //For debugging
                // println!("Finished parsing procedure header: ");
                // headerBlock.display(0);

                //Parses the procedure body
                let mut newBody: Vec<Token> = bodyList.iter().cloned().map(|t| t.clone()).collect();
                newBody.drain(0..1);
                let mut bodyBlock = Stmt::Block(Vec::new(), tokenList[0].lineNum.clone());
                let mut bodyI = 0;
                let bodyLen = newBody.len();
                while(!newBody.is_empty()){
                    if(bodyI > bodyLen){
                        self.reports.reportError("Infinite loop in body".to_string());
                        return Err("infinite loop in body".to_string());
                    }
                    let scanned = self.parse(&mut newBody);                            
                    let mut headerStmt:Stmt;
                    match scanned {
                        Ok((Some(stmt))) => {
                            let _ = bodyBlock.push_to_block(stmt.clone());
                            bodyI = bodyI + 1;
                        },
                        Ok((None)) => {
                            //continue because this cant happen
                            bodyI = bodyI + 1;
                        },
                        Err(reporting) => {
                            let errMsg = format!("Error parsing procedure body: {:?}", self.reports);
                            return Err(errMsg);
                        },
                    }
                    
                }

                //Displays the body (debugging)
                // println!("Finished parsing procedure body: ");
                // bodyBlock.display(0);

                // Turns the statements into boxes
                let boxHeader: Box<Stmt> = Box::new(headerBlock);
                let boxBody: Box<Stmt> = Box::new(bodyBlock);
                let boxParams: Box<Stmt> = Box::new(paramList);

                //Creates the procedure stmt, modifies the tokenList, returns
                let procedureAst = Stmt::ProcDecl(procedureType, procId.clone(), boxParams, boxHeader, boxBody, tokenList[0].lineNum.clone());
                
                self.scope -= 1;

                tokenList.drain(0..k + 2);
                return Ok(Some(procedureAst));
            }
            //For return statement
            tokenTypeEnum::RETURN => {
                //Checks if there is a value being returned
                if tokenList[1].tt != tokenTypeEnum::SEMICOLON {
                    //Initializes the variable that is being referenced first
                    let mut varRef:Expr;
                    //Initializes the return statement (I DONT THINK THIS IS NEEDED)
                    let mut retStmt:Stmt;

                    //Initializes values for finding the end of the expression
                    let mut k = 0;
                    let mut nextTok = tokenList[k].clone();
                    let mut curStmt: Vec<Token> = vec![];
                    
                    //Finds the end of the expression
                    while k < tokenList.len() {
                        let nextTok = &tokenList[k];
                        curStmt.push(nextTok.clone());
                        if (nextTok.tt == tokenTypeEnum::SEMICOLON) {
                            break;
                        }
                    
                        k += 1;
                    }

                    curStmt.drain(0..1);

                    if(curStmt[0].tt == tokenTypeEnum::L_PAREN){
                        curStmt.drain(0..1);
                        curStmt.remove(curStmt.len() - 2);
                        
                    }

                    //Parses the return expression
                    let scanExpr = self.parseExpr(&mut curStmt);
                    let retExpr: Expr;
                    match scanExpr {
                        Ok(expr) => {
                            retExpr = expr;
                        }
                        Err(err) => {
                            return Err(err);
                        }
                    }


                    let retVal = Stmt::Return((retExpr), tokenList[0].lineNum.clone());
                    tokenList.drain(..k+1);
                    return Ok(Some(retVal));
                } 
                
                else {
                    let retValue = Expr::VarRef("".to_string());
                    let retStmt = Stmt::Return(retValue, tokenList[0].lineNum.clone());
                    tokenList.drain(0..3);

                    return(Ok(Some(retStmt)));
                }
            }           
            //The end of the program
            tokenTypeEnum::END_PROGRAM => {
                let len = tokenList.len();
                tokenList.drain(0..len);
                return Ok((None));
            }
            //The end of a procedure
            tokenTypeEnum::END_PROCEDURE => {
                let len = tokenList.len();
                tokenList.drain(0..len);
                return Ok((None));
            }
            //An in constant is found
            tokenTypeEnum::INT => {
                let mut retStmt:Stmt;
                println!("integer");
                let mut k = 0;
                let mut nextTok = &tokenList[k];
                let mut curStmt: Vec<&Token> = vec![];
                while k < tokenList.len() {
                    let nextTok = &tokenList[k];
                    curStmt.push(nextTok);
                
                    if (nextTok.tt == tokenTypeEnum::SEMICOLON) || (nextTok.tt == tokenTypeEnum::R_PAREN) {
                        break;
                    }
                
                    k += 1;
                }
                
                if(curStmt.len() == 4) {
                    let operand1 = Expr::new(curStmt[0].tt.clone(), Some(curStmt[0].tokenString.clone()));
                    let mut op1Expr: Expr;
                    match operand1 {
                        Ok(expr) => {
                            op1Expr = expr;
                        }
                        Err(err) => {
                            println!("Error parsing operand 1");
                            let errMsg = format!("Error with operand 1 on line {}: {}", curStmt[0].lineNum, err);
                            self.reports.reportError(errMsg);
                            return Err("Error with operand 1".to_string());
                        }
                    }
                    
                    let operand2 = Expr::new(curStmt[2].tt.clone(), Some(curStmt[2].tokenString.clone()));
                    let mut op2Expr: Expr;
                    match operand2 {
                        Ok(expr) => {
                            op2Expr = expr;
                        }
                        Err(err) => {
                            println!("Error parsing operand 2");
                            let errMsg = format!("Error with operand 2 on line {}: {}", curStmt[0].lineNum, err);
                            self.reports.reportError(errMsg);
                            return Err("Error with operand 2".to_string());
                        }
                    }
                
                    let operator = Operator::new(curStmt[1].tt.clone());
                    let mut opBin:Operator; 
                    match operator {
                        Ok(expr) => {
                            opBin = expr;
                        }
                        Err(err) => {
                            let errMsg = format!("Error with operator on line {}: {}", curStmt[0].lineNum, err);
                            self.reports.reportError(errMsg);
                            let errMsg =  format!("Error with operator on line {}", curStmt[0].lineNum.clone());
                            println!("{}", errMsg);
                            return Err(errMsg);
                        }
                    }
                    
                    let finalExpr = Expr::newOp(Box::new(op1Expr), opBin, Box::new(op2Expr));

                    let retStmt = Stmt::Expr(finalExpr, tokenList[0].lineNum.clone());
                    tokenList.drain(0..k+1);
                    return Ok(Some(retStmt));

                } else if (curStmt.len() > 4) {
                    let operand1 = Expr::new(curStmt[0].tt.clone(), Some(curStmt[0].tokenString.clone()));
                    let mut op1Expr: Expr;
                    match operand1 {
                        Ok(expr) => {
                            op1Expr = expr;
                        }
                        Err(err) => {
                            println!("Error parsing operand 1");
                            let errMsg = format!("Error with operand 1 on line {}: {}", curStmt[0].lineNum, err);
                            self.reports.reportError(errMsg);
                            return Err("Error with operand 1".to_string());
                        }
                    }

                    
                    let mut subList = tokenList.clone();
                    subList.drain(0..2);

                    let mut parsedExpr: Expr;
                    let scanned = self.parse(&mut subList);                            
                        let mut headerStmt:Expr;
                        match scanned {
                            Ok((Some(stmt))) => {
                                let parsed = stmt.extractExpr();
                                match parsed {
                                    Ok(expr) => {
                                        parsedExpr = expr
                                    },
                                    Err(msg) => {
                                        println!("Error parsing expression from statment");
                                        let errMsg = format!("Error parsing body: {:?}", self.reports);
                                        parsedExpr = Expr::IntLiteral(0);
                                    }
                                }
                                            
                                
                            },
                            Ok((None)) => {
                                //continue
                                let errMsg = format!("Error parsing expression on line {}", tokenList[0].lineNum.clone());
                                return Err("Error parsing expression".to_string());
                            },
                            Err(reporting) => {
                                let errMsg = format!("Error parsing expression on line {}: {:?}", tokenList[0].lineNum.clone(), reporting.clone());
                                return Err("Error parsing expression".to_string());
                            },
                        }
                    
                    let op2Expr = parsedExpr;
                    let operator = Operator::new(curStmt[1].tt.clone());
                    let mut opBin:Operator; 
                    match operator {
                        Ok(expr) => {
                            opBin = expr;
                        }
                        Err(err) => {
                            let errMsg = format!("Error with operator on line {}: {}", curStmt[0].lineNum, err);
                            self.reports.reportError(errMsg.clone());
                            return Err(errMsg);
                        }
                    }

                    let finalExpr = Expr::newOp(Box::new(op1Expr), opBin, Box::new(op2Expr));
                    let retStmt = Stmt::Expr(finalExpr, tokenList[0].lineNum.clone());
                    tokenList.drain(0..k+1);
                    return Ok(Some(retStmt));
                } else {
                    let errMsg = format!("In line: {}, expression is too short'", curStmt[1].lineNum);
                    self.reports.reportError(errMsg);
                    return Err("Error with expression".to_string());
                }
            }
            //A constant float is found
            tokenTypeEnum::FLOAT => {
                let mut retStmt:Stmt;
                
                let mut k = 0;
                let mut nextTok = &tokenList[k];
                let mut curStmt: Vec<&Token> = vec![];
                while k < tokenList.len() {
                    let nextTok = &tokenList[k];
                    curStmt.push(nextTok);
                
                    if (nextTok.tt == tokenTypeEnum::SEMICOLON) || (nextTok.tt == tokenTypeEnum::R_PAREN) {
                        break;
                    }
                
                    k += 1;
                }
                if(curStmt.len() == 4) {
                    let operand1 = Expr::new(curStmt[0].tt.clone(), Some(curStmt[0].tokenString.clone()));
                    let mut op1Expr: Expr;
                    match operand1 {
                        Ok(expr) => {
                            op1Expr = expr;
                        }
                        Err(err) => {
                            let errMsg = format!("Error with operand 1 on line {}: {}", curStmt[0].lineNum, err);
                            self.reports.reportError(errMsg);
                            return Err("Error with operand 1".to_string());
                        }
                    }
                    
                    let operand2 = Expr::new(curStmt[2].tt.clone(), Some(curStmt[2].tokenString.clone()));
                    let mut op2Expr: Expr;
                    match operand2 {
                        Ok(expr) => {
                            op2Expr = expr;
                        }
                        Err(err) => {
                            let errMsg = format!("Error with operand 2 on line {}: {}", curStmt[0].lineNum, err);
                            self.reports.reportError(errMsg);
                            return Err("Error with operand 2".to_string());
                        }
                    }
                
                    let operator = Operator::new(curStmt[1].tt.clone());
                    let mut opBin:Operator; 
                    match operator {
                        Ok(expr) => {
                            opBin = expr;
                        }
                        Err(err) => {
                            let errMsg = format!("Error with operator on line {}: {}", curStmt[0].lineNum, err);                            
                            self.reports.reportError(errMsg);
                            return Err("Error with operator".to_string());
                        }
                    }
                    
                    let finalExpr = Expr::newOp(Box::new(op1Expr), opBin, Box::new(op2Expr));

                    let retStmt = Stmt::Expr(finalExpr, tokenList[0].lineNum.clone());
                    tokenList.drain(0..k+1);
                    return Ok(Some(retStmt));

                } else if (curStmt.len() > 4) {
                    //Parses the first operand
                    let operand1 = Expr::new(curStmt[0].tt.clone(), Some(curStmt[0].tokenString.clone()));
                    let mut op1Expr: Expr;
                    match operand1 {
                        Ok(expr) => {
                            op1Expr = expr;
                        }
                        Err(err) => {
                            let errMsg = format!("Error with operand1 on line {}: {}", curStmt[0].lineNum, err);                            
                            self.reports.reportError(errMsg);
                            return Err("Error with operand 1".to_string());
                        }
                    }

                    let mut subList = tokenList.clone();
                    subList.drain(0..2);

                    let mut parsedExpr: Expr;
                    let scanned = self.parse(&mut subList);                            
                    let mut headerStmt:Expr;
                    match scanned {
                        Ok((Some(stmt))) => {
                            let parsed = stmt.extractExpr();
                            match parsed {
                                Ok(expr) => {
                                    parsedExpr = expr
                                },
                                Err(msg) => {
                                    let errMsg = format!("Error parsing expression on line {}: {:?}", tokenList[0].lineNum.clone(), self.reports);                            
                                    parsedExpr = Expr::IntLiteral(0);
                                }
                            }
                                        
                            
                        },
                        Ok((None)) => {
                            println!("Parsed complex expression but no statement returned.");
                            parsedExpr = Expr::IntLiteral(0);
                        },
                        Err(reporting) => {
                            println!("Error parsing expression: {:?}", reporting);
                            let errMsg = format!("Error parsing body: {:?}", self.reports);

                            return Err(errMsg);
                        },
                    }
                    let op2Expr = parsedExpr;


                    let operator = Operator::new(curStmt[1].tt.clone());
                    let mut opBin:Operator; 
                    match operator {
                        Ok(expr) => {
                            opBin = expr;
                        }
                        Err(err) => {
                            let errMsg = format!("Error with operator on line {}: {}", curStmt[0].lineNum, err);
                            self.reports.reportError(errMsg);
                            return Err("Error with operator".to_string());
                        }
                    }

                    let finalExpr = Expr::newOp(Box::new(op1Expr), opBin, Box::new(op2Expr));

                    let retStmt = Stmt::Expr(finalExpr, tokenList[0].lineNum.clone());
                    tokenList.drain(0..k+1);
                    return Ok(Some(retStmt));
                } else {
                    let errMsg = format!("In line: {}, expression is too short'", curStmt[3].lineNum.clone());
                    self.reports.reportError(errMsg);
                    return Err("Error with expression".to_string());
                }
            }
            //A true bool constant has been found
            tokenTypeEnum::TRUE => {
                let trueExpr = Expr::BoolLiteral(true);
                return Ok(Some(Stmt::Expr((trueExpr), (tokenList[0].lineNum.clone()))));
            }
            //A false bool constant has been found
            tokenTypeEnum::FALSE => {
                let falseExpr = Expr::BoolLiteral(false);
                return Ok(Some(Stmt::Expr((falseExpr), (tokenList[0].lineNum.clone()))));
            }
            //A procedure reference has been found
            tokenTypeEnum::PROCEDURE_CALL => {
                let mut k = 1;
                let mut nextTok = &tokenList[k];
                let mut curStmt: Vec<Token> = vec![];
                curStmt.push(token.clone());
                while nextTok.tt != tokenTypeEnum::SEMICOLON {
                    curStmt.push(nextTok.clone());
                    k = k + 1;
                    nextTok = &tokenList[k];
                }
                curStmt.push(nextTok.clone());

                let mut procExpr: Expr;
                let procCallExpr = self.parseExpr(&mut curStmt.clone());
                match procCallExpr{
                    Ok(expr) => {
                        procExpr = expr;
                    }
                    Err(ErrMsg) => {
                        let errMsg = format!("Error with parsing procedure call on line {}", curStmt[0].lineNum.clone());
                        self.reports.reportError(errMsg.clone());
                        return Err("Error parsing procedure call".to_string());
                    }
                }
                tokenList.drain(0..k + 1);
                return Ok(Some(Stmt::Expr((procExpr), (curStmt[0].lineNum.clone()))));
            }
            _ => {
                let errMsg = format!("Unexpected token: '{}' on line: {}", token.tokenString, token.lineNum);
                self.reports.reportError(errMsg.clone());
                tokenList.drain(0..1);
                return Err("Unexpected token found".to_string());
            }
        }
    }

    //Prints all of the tokens in the lexers tokenList
    pub fn printTokenList(&mut self){
        for token in &self.tokenList {
            println!("< \"{}\" , {}, {} >", token.tokenString, token.tt.to_string(), token.lineNum);
        }
    }
    
}

//An enumeration used to define the different operators available
#[derive(Debug, Clone, PartialEq)]
pub enum Operator {
    Add,
    Sub,
    Mul,
    Div,
    Greater,
    Less,
    Greater_Equal,
    Less_Equal,
    Check_Equal,
    And,
    Or,
    Not,
    Not_Equals,
}
//Functions for the operator enumeration
impl Operator {
    pub fn new(op_str: tokenTypeEnum) -> Result<Self, String> {
        match op_str {
            tokenTypeEnum::PLUS => Ok(Operator::Add),
            tokenTypeEnum::MINUS => Ok(Operator::Sub),
            tokenTypeEnum::MULTIPLY => Ok(Operator::Mul),
            tokenTypeEnum::DIVIDE => Ok(Operator::Div),
            tokenTypeEnum::GREATER => Ok(Operator::Greater),
            tokenTypeEnum::LESS => Ok(Operator::Less),
            tokenTypeEnum::GREATER_EQUALS => Ok(Operator::Greater_Equal),
            tokenTypeEnum::LESS_EQUALS => Ok(Operator::Less_Equal),
            tokenTypeEnum::CHECK_EQUALS => Ok(Operator::Check_Equal),
            tokenTypeEnum::AND => Ok(Operator::And),
            tokenTypeEnum::OR => Ok(Operator::Or),
            tokenTypeEnum::NOT => Ok(Operator::Not),
            tokenTypeEnum::NOT_EQUALS => Ok(Operator::Not_Equals),

            _ => Err(format!("Unsupported operator: {}", op_str)),
        }
    }
}
//Tells the enumerators how to display
impl fmt::Display for Operator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operator::Add => write!(f, "+"),
            Operator::Sub => write!(f, "-"),
            Operator::Mul => write!(f, "*"),
            Operator::Div => write!(f, "/"),
            Operator::Greater => write!(f, ">"),
            Operator::Less => write!(f, "<"),
            Operator::Greater_Equal => write!(f, ">="),
            Operator::Less_Equal => write!(f, "<="),
            Operator::Check_Equal => write!(f, "=="),
            Operator::And => write!(f, "&"),
            Operator::Or => write!(f, "|"),
            Operator::Not => write!(f, "not"),
            Operator::Not_Equals => write!(f, "!="),
        }
    }
}

// Define types of expressions
//Expressions are the smallest building blocks of the AST
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    //Literals
    IntLiteral(i64),                            //An integer literal (int value)
    FloatLiteral(f32),                          //A float literal (float value)
    StringLiteral(String),                      //A string literal (the string)
    BoolLiteral(bool),
    IntArrayLiteral(i32, Vec<i64>),             //An integer array literal
    
    //References
    VarRef(String),                             //A reference to a variable (variable name)
    ProcRef(String, Option<Vec<Expr>>),         //Procedure calls: the name of the procedure, an optional box of a Block of Exprs for the parameters 
    ArrayRef(String, Box<Expr>),                //A reference to an array index (array name, Box of the index value)
                                            //                               This is a box because it can be an intliteral or BinOp
    
    //Operations
    ArthOp(Box<Expr>, Operator, Box<Expr>),     //An arthmetic Operation, (Operand 1, an instance of the BinOp enum, Operand 2)
                                            //                      These are boxes because they can contain more BinOps within themselves     
    RelOp(Box<Expr>, Operator, Box<Expr>),      //A relational operation (operand 1, operator (<, >, etc.), operand 2) 
    LogOp(Box<Expr>, Operator, Box<Expr>),      //Operator for logical/bitwise equations (op1, operator (&, |, !), op2)

    
}

//Functions for the expressions
impl Expr {
    //Constructor that can create exprs depending on different situations with parameters
    pub fn new(expr_type: tokenTypeEnum, param1: Option<String>) -> Result<Self, String> {
        match expr_type {
            tokenTypeEnum::INT => {
                let value_str = param1.ok_or("IntLiteral requires an integer parameter".to_string())?;
                let value = value_str.parse::<i64>().map_err(|e| format!("Failed to parse integer: {}", e))?;
                Ok(Expr::IntLiteral(value))
            },
            tokenTypeEnum::FLOAT => {
                let value_str = param1.ok_or("Float requires a float parameter".to_string())?;
                let value = value_str.parse::<f32>().map_err(|e| format!("Failed to parse integer: {}", e))?;
                Ok(Expr::FloatLiteral(value))
            },
            tokenTypeEnum::STRING => {
                let value = param1.ok_or("StringLiteral requires a string parameter".to_string())?.to_string();
                Ok(Expr::StringLiteral(value))
            },
            tokenTypeEnum::FALSE => {
                return Ok(Expr::BoolLiteral(false));
            }
            tokenTypeEnum::TRUE => {
                return Ok(Expr::BoolLiteral(true));
            }
            tokenTypeEnum::IDENTIFIER => {
                let var_name = param1.ok_or("VarRef requires a variable name".to_string())?.to_string();
                Ok(Expr::VarRef(var_name))
            },
            _ => Err("Invalid expression type".to_string()),
        }
    }

    pub fn newOp(op1: Box<Expr>, operand: Operator, op2: Box<Expr>) -> Expr {
        match operand{
            //Relational operators
            Operator::Check_Equal => {
                return  Expr::RelOp(op1, operand, op2);
            }
            Operator::Greater => {
                return  Expr::RelOp(Box::new(*op1), operand, Box::new(*op2));
            }
            Operator::Greater_Equal => {
                return  Expr::RelOp(Box::new(*op1), operand, Box::new(*op2));
            }
            Operator::Less_Equal => {
                return  Expr::RelOp(Box::new(*op1), operand, Box::new(*op2));
            }
            Operator::Less => {
                return  Expr::RelOp(Box::new(*op1), operand, Box::new(*op2));
            }
            Operator::Not_Equals => {
                return  Expr::RelOp(Box::new(*op1), operand, Box::new(*op2));
            }
            
            //Logical Operators
            Operator::And => {
                return  Expr::LogOp(Box::new(*op1), operand, Box::new(*op2));
            }
            Operator::Or => {
                return  Expr::LogOp(Box::new(*op1), operand, Box::new(*op2));
            }
            Operator::Not => {
                return  Expr::LogOp(Box::new(*op1), operand, Box::new(*op2));
            }

            //The remainder (arthmetic operators)
            _ => {
                return  Expr::ArthOp(Box::new(*op1), operand, Box::new(*op2));
            }
        }
    }

    pub fn newCon(constant: Token) -> Result<Expr, String>{
        if constant.tg.clone() != tokenGroup::CONSTANT {
            let errMsg = format!("Error parsing constant {} on line {}", constant.tokenString.clone(), constant.lineNum.clone());
            return Err(errMsg);
        } else {
            match constant.tt.clone(){
                tokenTypeEnum::FALSE => {
                    return Ok(Expr::BoolLiteral(false));
                }
                tokenTypeEnum::TRUE => {
                    return Ok(Expr::BoolLiteral(true));
                }
                tokenTypeEnum::FLOAT => {
                    return Ok(Expr::FloatLiteral(constant.tokenString.clone().parse().unwrap()));
                }
                tokenTypeEnum::INT => {
                    return Ok(Expr::IntLiteral(constant.tokenString.clone().parse().unwrap()));
                }
                tokenTypeEnum::STRING => {
                    return Ok(Expr::StringLiteral(constant.tokenString.clone()));
                }
                _ => {
                    let errMsg = format!("Error parsing constant {} on line {}: Invalid constant type {}", constant.tokenString.clone(), constant.lineNum.clone(), constant.tt.clone());
                    return Err(errMsg);
                }
            }
        }
    }
}
//Tells the expr how to display
impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::IntLiteral(i) => write!(f, "{}", i),
            Expr::StringLiteral(s) => write!(f, "{}", s),
            Expr::FloatLiteral(n) => write!(f, "{}", n),
            Expr::ArthOp(left, op, right) => write!(f, "({} {} {})", left, op, right),
            Expr::VarRef(var) => write!(f, "{}", var),
            Expr::ArrayRef(var, index) => write!(f, "({}[{}])", var, index),
            Expr::ProcRef(name, Some(params)) => {
                let params_str = params.iter().map(|expr| format!("{}", expr)).collect::<Vec<_>>().join(", ");
                write!(f, "{}({})", name, params_str)
            },
            Expr::ProcRef(name, None) => write!(f, "{}()", name),
            Expr::RelOp(left, op, right) => write!(f, "({} {} {})", left, op, right),
            Expr::LogOp(left, op, right) => write!(f, "({} {} {})", left, op, right),
            Expr::BoolLiteral(val) => write!(f, "{}", val),
            Expr::IntArrayLiteral(size, array) => write!(f, "([{}])", size),

        }
    }
}

// Define supported variable types
#[derive(Debug, Clone, PartialEq)]
pub enum VarType {
    Int,
    Bool,
    Float,
    Str,
    IntArray(i32),
}
impl VarType {
    pub fn new(typeStr: &str) -> Result<Self, String> {
        match typeStr {
            "integer" => Ok(VarType::Int),
            "bool" => Ok(VarType::Bool),
            "float" => Ok(VarType::Float),
            "string" => Ok(VarType::Str),
            "int[]" => Ok(VarType::IntArray(0)),
            
            _ => Err(format!("Unsupported var type: {}", typeStr)),
        }
    }
}
impl fmt::Display for VarType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VarType::Int => write!(f, "Int"),
            VarType::Bool => write!(f, "Bool"),
            VarType::Float => write!(f, "Float"),
            VarType::Str => write!(f, "Str"),
            VarType::IntArray(arr) => write!(f, "IntArray[{}]", arr),
        }
    }
}

// These are the types of statements that are available
//Statements make up the nodes of the AST and are made up of expressions
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    StringLiteral(String, String),
    Expr(Expr, String),                     // Expression statement
    Assign(Expr, Expr, String),           // Assignment statement: variable refernce, expression to assign to
    VarDecl(String, VarType, String),       // Variable declaration statement
    GlobVarDecl(String, VarType, String),       // Variable declaration statement
    If(Expr, Box<Stmt>, Option<Box<Stmt>>, String),  // If statement: condition, body, optional else body
    For(Rc<Stmt>, Expr, Box<Stmt>, String),          // For statement: assignment, condition, Box of commands for statement
    Block(Vec<Stmt>, String),               // Block statement: list of statements
    Error(Reporting, String),
    Return(Expr, String),
    Program(String, Box<Stmt>, Box<Stmt>, String), //The program AST: Name, header block, body block, lineNum
    ProcDecl(VarType, String, Box<Stmt>, Box<Stmt>, Box<Stmt>, String), //Procedure AST: type, Name, parameter, Header, body
}
//Functions for Stmt
impl Stmt {
    // Function to push a statement into a Block variant
    pub fn push_to_block(&mut self, stmt: Stmt) -> Result<(), String> {
        let fakeLine = "num".to_string();
        match self {
            Stmt::Block(stmts, fakeLine) => {
                stmts.push(stmt);
                Ok(())
            },
            _ => Err("Cannot push to a non-Block statement".to_string())
        }
    }

    pub fn display(&self, indent: usize) {
        let indentation = " ".repeat(indent);
        match self {
            Stmt::StringLiteral(s, lineNum) => println!("{}StringLiteral({})", indentation, s),
            Stmt::Expr(expr, lineNum) => println!("{}Expr({})", indentation, expr),
            Stmt::Assign(var, expr, lineNum) => println!("{}Assign({}, {})", indentation, var, expr),
            Stmt::VarDecl(var, vartype, lineNum) => println!("{}VarDecl({}, {})", indentation, var, vartype),
            Stmt::GlobVarDecl(var, vartype, lineNum) => println!("{}GlobVarDecl({}, {})", indentation, var, vartype),
            Stmt::If(cond, body, else_body, lineNum) => {
                println!("{}If (", indentation);
                println!("{}  Condition: {}", indentation, cond);
                println!("{}  Body: ", indentation);
                body.display(indent + 2);
                if let Some(else_stmt) = else_body {
                    println!("{}  Else: ", indentation);
                    else_stmt.display(indent + 2);
                }
                println!("{})", indentation);
            }
            Stmt::For(assignment, cond, body, lineNum) => {
                println!("{}For (", indentation);
                println!("{}  Assignment: ", indentation);
                assignment.display(indent + 3);
                println!("{}  Condition: {}", indentation, cond);
                println!("{}  Body: ", indentation);
                body.display(indent + 3);
                println!("{})", indentation);
            }
            Stmt::Block(stmts, lineNum) => {
                println!("{}Block([", indentation);
                for stmt in stmts {
                    stmt.display(indent + 2);
                }
                println!("{}])", indentation);
            },
            Stmt::Error(reporting, lineNum) => println!("{}Error({:?})", indentation, reporting),
            Stmt::Return(expr, lineNum) => println!("{}Return({})", indentation, expr),
            Stmt::Program(name, header, body, lineNum) => {
                println!("{}{}:(", indentation,name);
                println!(" {}Header:",indentation);
                header.display(indent + 1);
                println!(" {}Body:",indentation);
                body.display(indent + 1);
                println!("{})", indentation);
            }
            Stmt::ProcDecl(procType, name, params, header, body, lineNum) => {
                println!("{}{} {}:(", indentation,procType,name);
                println!(" {}Params:",indentation);
                params.display(indent + 1);
                
                println!(" {}Header:",indentation);
                header.display(indent + 1);
                println!(" {}Body:",indentation);
                body.display(indent + 1);
                println!("{})", indentation);
            }
            
        }
    }

    //Used to get an Expr from a returned Stmt if the Stmt is just a Expr
    pub fn extractExpr(&self) -> Result<Expr, String> {
        match self {
            Stmt::Expr(expr, lineNum) => Ok(expr.clone()),
            _ => Err("Provided statement is not an expression.".to_string()),
        }
    }
}

//A function that just prints a given list of tokens (used for debugging)
fn printTokList(tokList: &Vec<Token>){
    for token in tokList {
        println!("< \"{}\" , {}, {} >", token.tokenString, token.tt.to_string(), token.lineNum);
    }
}

///////////////////////// /PARSER SECTION /////////////////////////
