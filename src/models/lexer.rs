///////////////////////// Setup /////////////////////////

//Rules
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(unused_imports)]
#![allow(dead_code)]
#![allow(unused_parens)]
#![allow(unused_mut)]
#![allow(unused_variables)]

//External library imports
extern crate anyhow;
extern crate parse_display;
extern crate utf8_chars;
extern crate unicode_segmentation;

//Imports necessary packages
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
    crate::models::reporting::*,
    std::io::prelude::*,

};

///////////////////////// /Setup /////////////////////////



///////////////////////// LEXER SECTION /////////////////////////
//This section contains all of the necessary code for the Lexical analysis section of the code.
//This includes all of the structs and functions that make up token definitions and such.

//This is the master struct for the lexer
//Defines all of the variables contained within the lexer struct
pub struct Lexer {
    pub inputFile: inFile,      //The file that is imported
    pub symTab: tokenTable,     //The table of tokens, seeded with keywords
    pub tokenList: Vec<Token>,  //The list of the tokens that the lexer processes. This is the output of the lexer
    pub reports: Reporting,     //This is a reporting structure, used to report errors and stuff
}

//This is where all of the methods of the lexer struct are defined
impl Lexer{
    //The default constructor for the lexer
    pub fn new(fileName: &str) -> Lexer {
        // println!("Beginning creation of Lexer");
        //Creates the inFile structure
        let newFile = inFile::new(fileName);
        // println!("Lexer created successfully");
        //Creates the token table
        let mut symTable = tokenTable::new();
        //Creates the reporting class
        let mut report: Reporting = Reporting::new();

        //This is the lexer object that is returned
        Lexer { 
            inputFile: newFile,
            symTab: symTable,
            tokenList: Vec::new(),
            reports: report,
        }
    }
    
    //The main function of the lexer
    //Returns one Token
    fn scan(&mut self) -> Token{
        //Gets the next character
        let mut currChar = self.inputFile.getChar();

        //Looks for the filler characters and removes them
        while let Some(c) = currChar {            
            //Defines the filler characters, finds them, passes them or marks whatever is necessary
            if c == '\n' || c == '\t' || c == '\r' || c == ' ' || c == '\u{0009}' {
                // println!("Filler character found: '{}'", c);
                
                if c == '\n' {
                    self.inputFile.incLineCnt();
                }
                currChar = self.inputFile.getChar();
            } else {
                break;
            }
        }

        //This section parses and ignores comments by looking for the comment identifiers and then skipping until finding the end
        if let Some('/') = currChar {
            currChar = self.inputFile.getChar();
            let Some(c) = currChar else { todo!() };
            //Two /s in a row, single line comment
            if c == '/' {
                // println!("Comment line found");
                while let Some(c) = currChar {
                    if c == '\n' {
                        self.inputFile.incLineCnt();
                        break;
                    } else {
                        currChar = self.inputFile.getChar();
                    }
                }
            } 
            //This identifies a multiline comment
            else if c == '*' {
                // println!("multiline comment");
                let mut nested: usize = 1;
                // println!("Comment line found");
                //Finds the end of the multiline comment
                while let Some(c) = currChar {
                    //If a nested multiline comment is found, increases scope
                    if c == '/' {
                        // println!("scope +1 nested");
                        currChar = self.inputFile.getChar();
                        let Some(ch) = currChar else { todo!() };
                        if ch == '*' {
                            nested += 1;
                            currChar = self.inputFile.getChar();
                        }
                    } else if c == '*' {
                        currChar = self.inputFile.getChar();
                        let Some(ch) = currChar else { todo!() };
                        if ch == '/' {
                            nested -= 1;
                            if nested == 0 {
                                currChar = self.inputFile.getChar();
                                // println!("End of nested comment");
                                break;
                            } else {
                                currChar = self.inputFile.getChar();
                                // println!("Not end of internal nested comment");
                            }
                        }
                    } else if c == '\n' {
                        self.inputFile.incLineCnt();
                        currChar = self.inputFile.getChar();

                          
                    } else {
                        currChar = self.inputFile.getChar();
                    }
                }
            } 
            //This is for if the / has a space after it meaning its a divide not a comment
            else if c == ' ' {
                let tokenString = '/';
                let newToken = Token::new(crate::tokenTypeEnum::DIVIDE,tokenString.to_string(), self.inputFile.lineCnt.to_string(), tokenGroup::OPERATOR);
                return newToken;
            }
        }

        //A switch case to handle all of the different characters the lexer could find
        let mut tokenString: String = "".to_string();
        match currChar {
            //If the character is a lette, iterates through until no more letters or numbers are found, turns all of that into a word then checks if it is a predefined keyword token, creating a new token if not
            Some(ch) if ch.is_ascii_alphabetic() => {
                let mut tokType: tokenTypeEnum = tokenTypeEnum::WORD;
                //Iterates through until it stops finding numbers
                while let Some(numC) = currChar {
                    if (numC.is_ascii_alphabetic() || numC.is_ascii_digit() || numC == '_')  {
                        tokenString.push(numC);
                        currChar = self.inputFile.getChar();
                    } else {
                        break;
                    }
                }
                self.inputFile.unGetChar();
                tokenString = tokenString.to_ascii_lowercase();
                let mut newToken = self.symTab.hashLook(tokenString, self.inputFile.lineCnt.to_string());
                newToken.lineNum = self.inputFile.lineCnt.to_string();
                return newToken;
            }

            //If the character is a number, iterates through until end of number, if a . is found, it creates a float token
            Some(ch) if ch.is_ascii_digit() => {
                let mut tokType: tokenTypeEnum = tokenTypeEnum::INT;
                //Iterates through until it stops finding numbers
                while let Some(numC) = currChar {
                    if numC.is_ascii_digit() {
                        tokenString.push(numC);
                        currChar = self.inputFile.getChar();
                    //If the number has a decimal, meaning its a float
                    } else if numC == '.' {
                        tokenString.push('.');
                        tokType = tokenTypeEnum::FLOAT;
                        currChar = self.inputFile.getChar();
                    } else {
                        break;
                    }
                }
                self.inputFile.unGetChar();
                let newToken: Token = Token::new(tokType,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::CONSTANT);
                return newToken;
            }

            //If the character is a <, looks if it is a less or less equals
            Some('<') => {
                let mut nextNextChar = self.inputFile.getChar();
                tokenString.push('<');
                let Some(nextC) = nextNextChar else { todo!() };
                if nextC == '=' {
                    tokenString.push('=');
                    let newToken = Token::new(crate::tokenTypeEnum::LESS_EQUALS,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::OPERATOR);
                    return newToken;
                } else {
                    self.inputFile.unGetChar();
                    let newToken = Token::new(crate::tokenTypeEnum::LESS,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::OPERATOR);
                    return newToken;
                }
            }

            //If the character is a >, checks if > or >=
            Some('>') => {
                tokenString.push('>');
                let mut nextNextChar = self.inputFile.getChar();
                let Some(nextC) = nextNextChar else { todo!() };
                if nextC == '=' {
                    tokenString.push('=');
                    let newToken = Token::new(crate::tokenTypeEnum::GREATER_EQUALS,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::OPERATOR);
                    return newToken;
                } else {
                    self.inputFile.unGetChar();
                    let newToken = Token::new(crate::tokenTypeEnum::GREATER,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::OPERATOR);
                    return newToken;
                }
            }

            //If the character is a =, checks if a = or a ==
            Some('=') => {
                tokenString.push('=');
                let mut nextNextChar = self.inputFile.getChar();
                let Some(nextC) = nextNextChar else { todo!() };
                if nextC == '=' {
                    tokenString.push('=');
                    let newToken = Token::new(crate::tokenTypeEnum::CHECK_EQUALS,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::OPERATOR);
                    return newToken;
                } else if nextC == ' ' {
                    self.inputFile.unGetChar();
                    let newToken = Token::new(crate::tokenTypeEnum::SET_EQUALS,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::OPERATOR);
                    return newToken;
                } else {
                    //If there is an unknown next character, creates an error token, this will be turned into an error given to the user in the second pass
                    println!("ERROR");
                    self.inputFile.unGetChar();
                    let newToken = Token::new(crate::tokenTypeEnum::ERROR,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::OTHER);
                    return newToken;
                }
            }

            //If the character is a !, checks if != or just !, throws error if a !
            Some('!') => {
                tokenString.push('!');
                let mut nextNextChar = self.inputFile.getChar();
                let Some(nextC) = nextNextChar else { todo!() };
                if nextC == '=' {
                    tokenString.push('=');
                    let newToken = Token::new(crate::tokenTypeEnum::NOT_EQUALS,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::OPERATOR);
                    return newToken;
                } else {
                    self.inputFile.unGetChar();
                    let newToken = Token::new(crate::tokenTypeEnum::ERROR,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::OTHER);
                    return newToken;
                }
            }

            //If the character is a ;
            Some(';') => {
                tokenString.push(';');
                let newToken = Token::new(crate::tokenTypeEnum::SEMICOLON,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::SYMBOL);
                return newToken;
            }

            //If the character is a :, checks if a := or just a :
            Some(':') => {
                tokenString.push(':');
                let mut nextNextChar = self.inputFile.getChar();
                let Some(nextC) = nextNextChar else { todo!() };
                if nextC == '=' {
                    tokenString.push('=');
                    let newToken = Token::new(crate::tokenTypeEnum::SET_EQUALS,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::OPERATOR);
                    return newToken;
                } else {
                    self.inputFile.unGetChar();
                    let newToken = Token::new(crate::tokenTypeEnum::COLON,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::OPERATOR);
                    return newToken;
                }
            }

            //If the character is a [
            Some('[') => {
                tokenString.push('[');
                let newToken = Token::new(crate::tokenTypeEnum::L_BRACKET,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::SYMBOL);
                return newToken;
            }

            //If the character is a ]
            Some(']') => {
                tokenString.push(']');
                let newToken = Token::new(crate::tokenTypeEnum::R_BRACKET,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::SYMBOL);
                return newToken;
            }

            //If the character is a (
            Some('(') => {
                tokenString.push('(');
                let newToken = Token::new(crate::tokenTypeEnum::L_PAREN,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::SYMBOL);
                return newToken;
            }

            //If the character is a )
            Some(')') => {
                tokenString.push(')');
                let newToken = Token::new(crate::tokenTypeEnum::R_PAREN,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::SYMBOL);
                return newToken;
            }

            //If the character is a +
            Some('+') => {
                tokenString.push('+');
                let newToken = Token::new(crate::tokenTypeEnum::PLUS,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::OPERATOR);
                return newToken;
            }

            //If the character is a -
            Some('-') => {
                tokenString.push('-');
                let mut nextNextChar = self.inputFile.getChar();
                let Some(nextC) = nextNextChar else { todo!() };
                self.inputFile.unGetChar();
                let newToken = Token::new(crate::tokenTypeEnum::MINUS,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::OPERATOR);
                return newToken;
            }


            Some('*') => {
                tokenString.push('*');
                let newToken = Token::new(crate::tokenTypeEnum::MULTIPLY,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::OPERATOR);
                return newToken;
            }

            Some(',') => {
                tokenString.push(',');
                let newToken = Token::new(crate::tokenTypeEnum::COMMA,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::SYMBOL);
                return newToken;
            }

            Some('/') => {
                tokenString.push('/');
                let newToken = Token::new(crate::tokenTypeEnum::DIVIDE,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::OPERATOR);
                return newToken;
            }

            //If the character is a .
            Some('.') => {
                tokenString.push('.');
                let newToken = Token::new(crate::tokenTypeEnum::PERIOD,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::SYMBOL);
                return newToken;
            }

            //If the character is a &
            Some('&') => {
                tokenString.push('&');
                let newToken = Token::new(crate::tokenTypeEnum::AND,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::OPERATOR);
                return newToken;
            }

            //If the character is a |
            Some('|') => {
                tokenString.push('|');
                let newToken = Token::new(crate::tokenTypeEnum::OR,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::OPERATOR);
                return newToken;
            }

            //If the character is a ", finds the end of the string, creates a string character
            Some('"') => {
                currChar = self.inputFile.getChar();
                let mut tokType: tokenTypeEnum = tokenTypeEnum::WORD;
                while let Some(numC) = currChar {
                    if numC == '"' {
                        break;
                    } else {
                        tokenString.push(numC);
                    }
                    currChar = self.inputFile.getChar();

                }
                while tokenString.len() < 64 {
                    tokenString.push(' ');
                }
                tokenString.push('\0');
                let mut newToken = self.symTab.hashLook(tokenString, self.inputFile.lineCnt.to_string());
                newToken.lineNum = self.inputFile.lineCnt.to_string();
                if newToken.tt != tokenTypeEnum::STRING {
                    newToken.tt = tokenTypeEnum::STRING;
                }
                return newToken;
            }
            
            //Somehow a \n makes it here, just runs it through another scan to get the next thing
            Some('\n') => {
                let newToken = self.scan();
                return newToken;
            }
            
            //Unaccounted character, this will create the unaccounted token which throws an error on the second pass
            Some(c) => {
                // println!("This character is unaccounted for '{}'", c);
                tokenString.push(c);
                let newToken = Token::new(crate::tokenTypeEnum::UNACCOUNTED,tokenString, self.inputFile.lineCnt.to_string(), tokenGroup::OTHER);
                return newToken;
            }
            
            //This is if there is no character, meaning we have found the end of the file
            None => {
                // println!("This character is a None aka EOF");
                let newToken = Token::new(crate::tokenTypeEnum::EOF, "EOF".to_string(), self.inputFile.lineCnt.to_string(), tokenGroup::SYMBOL);
                return newToken;
            }
        }
    }
    
    //Prints all of the tokens, used for debugging
    fn printTokenList(&mut self){
        for token in &self.tokenList {
            println!("< \"{}\" , {}, {} >", token.tokenString, token.tt.to_string(), token.lineNum);
        }
    }

    //A second pass through the tokenString created by the lexer, used to find errors and combine certain groups of tokens into one token
    fn secondPass(&mut self) -> Vec<Token>{
        let mut newTokList = Vec::new(); 
        let mut i: usize = 0;
        while i < self.tokenList.len() {
            let token = &self.tokenList[i];
            match token.tt {
                //Turns an end token into an end program, procedure, or other structure
                tokenTypeEnum::END => {
                    // println!("End found");
                    let nextToken = &self.tokenList[i+1];
                    if nextToken.tt == tokenTypeEnum::PROGRAM {
                        // println!("Combining end and program");
                        let newToken = Token::new(crate::tokenTypeEnum::END_PROGRAM,"END_PROGRAM".to_string(), nextToken.lineNum.to_string(), tokenGroup::OTHER);
                        newTokList.push(newToken.clone());
                        i = i + 1;
                    } else if nextToken.tt == tokenTypeEnum::PROCEDURE {
                        // println!("Combining end and procedure");
                        let newToken = Token::new(crate::tokenTypeEnum::END_PROCEDURE,"END_PROCEDURE".to_string(), nextToken.lineNum.to_string(), tokenGroup::OTHER);
                        newTokList.push(newToken.clone());
                        i = i + 1;
                    } else if nextToken.tt == tokenTypeEnum::IF {
                        // println!("Combining end and if");
                        let newToken = Token::new(crate::tokenTypeEnum::END_IF,"END_IF".to_string(), nextToken.lineNum.to_string(), tokenGroup::OTHER);
                        newTokList.push(newToken.clone());
                        i = i + 1;
                    } else if nextToken.tt == tokenTypeEnum::FOR {
                        // println!("Combining end and if");
                        let newToken = Token::new(crate::tokenTypeEnum::END_FOR,"END_FOR".to_string(), nextToken.lineNum.to_string(), tokenGroup::OTHER);
                        newTokList.push(newToken.clone());
                        i = i + 1;
                    } else {
                        // println!("other end with type: {}", nextToken.tt);
                        newTokList.push(token.clone());

                    }
                }
                //Turns identifiers into procedure calls if that's what it is
                tokenTypeEnum::IDENTIFIER => {
                    let nextToken = &self.tokenList[i+1];
                    if nextToken.tt == tokenTypeEnum::L_PAREN {
                        // println!("Combining end and if");
                        let newToken = Token::new(crate::tokenTypeEnum::PROCEDURE_CALL, token.tokenString.clone(), nextToken.lineNum.to_string(), tokenGroup::SYMBOL);
                        newTokList.push(newToken.clone());
                        i = i + 1;
                    } else {
                        // println!("other end with type: {}", nextToken.tt);
                        newTokList.push(token.clone());

                    }
                }
                //The unaccounted token, throws an error
                tokenTypeEnum::UNACCOUNTED => {
                    if(token.tokenString == "\r"){
                        // println!("Skipping unaccounted");
                        // println!("Unaccounted: {}", token.tokenString);
                        let nextToken = &self.tokenList[i+1];
                        // println!("Next token: {}", nextToken.tokenString);
                        newTokList.push(nextToken.clone());
                        i = i + 1;
                    } else {
                        let errMsg = format!("Unaccounted token '{}' found on line {}", token.tokenString.clone(), token.lineNum.clone());
                        self.reports.reportError(errMsg.clone());
                        // println!("Skipping unaccounted");
                        // println!("Unaccounted: {}", token.tokenString);
                        let nextToken = &self.tokenList[i+1];
                        // println!("Next token: {}", nextToken.tokenString);
                        newTokList.push(nextToken.clone());
                        i = i + 1;
                    }
                    
                }
                //Checks - tokens for if they are neg numbers or minus operators
                tokenTypeEnum::MINUS => {
                    let nextToken = &self.tokenList[i+1];
                    let prevToken = &self.tokenList[i-1];
                    //Defines a negative number
                    if ((nextToken.tg == tokenGroup::VARIABLE) || (nextToken.tg == tokenGroup::CONSTANT)) && ((prevToken.tg == tokenGroup::OPERATOR) || (prevToken.tt == tokenTypeEnum::SET_EQUALS)) {
                        // println!("Found a neg number");
                        let newString = format!("-{}", nextToken.tokenString.clone());
                        let newToken = Token::new(nextToken.tt.clone(), newString, nextToken.lineNum.to_string(), tokenGroup::CONSTANT);
                        newTokList.push(newToken.clone());
                        i = i + 1;
                    //This is just a minus operator
                    } else {
                        newTokList.push(token.clone());
                    }
                }
                //All the other tokens, nothing done just passed through
                _ => {
                    // Handle other token types
                    newTokList.push(token.clone());
                }
            }
            //Increments the i
            i = i + 1;
        }
        //Returns the second pass of the token string
        return newTokList;
    }

    //A function to scan through entire file, this is the external function of the lexer that scans a file
    pub fn scanThrough(&mut self){
        // println!("\nBeginning scan:");

        //Scans the first token and initializes the newToken variable
        let mut newToken: Token = self.scan();
        self.tokenList.push(newToken.clone());

        //Goes through the inputfile and calls scan() which returns each token until the EOF is reached
        while newToken.tokenString != "EOF".to_string(){
            newToken = self.scan();
            self.tokenList.push(newToken.clone());
        };

        //Runs the tokenString through a second pass, this will conclude the lexer
        let newTokList = self.secondPass();
        self.tokenList = newTokList;
        // println!("Second pass finished");
    }

}

//inFile Class, this is where the file to be compiled is loaded
pub struct inFile{
    attatchFile: bool,
    pub fileName: String,
    fileContents: String,
    lineCnt: usize,
    pub numChars: usize,
    pub totalLines: usize,
    pub file : BufReader <File>,
    pub currentCharIndex: usize,
}
impl inFile {
    //Constructor, imports and opens the file
    fn new(fileName: &str) -> inFile {
        let mut newFile = BufReader::new(File::open(fileName).unwrap());
        let fileContentsString = std::fs::read_to_string(fileName).expect("Unable to read file");
        let numChars = fileContentsString.len();
        // println!("Creating the inFile structure");
        
        inFile {
            fileName: fileName.to_string(),
            attatchFile: false,
            lineCnt: 1,
            currentCharIndex: 0,
            totalLines: 0,
            file: newFile,
            fileContents: fileContentsString,
            numChars: numChars,
        }

    }

    //Prints the stats of the file (for debugging)
    fn printInfo(&self){
        println!("File Name: {}", self.fileName);
        println!("Lines: {}", self.lineCnt);
    }

    //Gets the next character in the file string
    fn getChar(&mut self) -> Option<char> {
        if let Some(current_char) = self.fileContents.chars().nth(self.currentCharIndex) {
            self.currentCharIndex += 1;
            Some(current_char)
        } else {
            None
        }
    }
    
    //"ungets" the next character by decrementing the current index. Used for looking ahead then going back
    fn unGetChar(&mut self) {
        self.currentCharIndex -= 1;
    }

    //A function to increment the current line
    fn incLineCnt(&mut self){
        self.lineCnt += 1;
    }

}

//Token class, this is where tokens are defined and setup
#[derive(Clone, PartialEq)]
pub struct Token{
    pub tt: tokenTypeEnum,
    pub tokenString: String,
    pub tg: tokenGroup,
    pub lineNum: String,
    //To be completed later when I understand
    //tm: tokenMark,
}
impl Token{
    //Init for the Token
    pub fn new(iden: tokenTypeEnum, tokenString: String, line: String, group: tokenGroup) -> Token{
        Token {
            tt: iden,
            tokenString: tokenString,
            lineNum: line,
            tg: group,
        }
    }
    
    //Used for setting the Token type
    fn setTokenType(&mut self, newType: tokenTypeEnum){
        self.tt = newType;
    }

    //Prints a given token (used for debuggin)
    fn printToken(&mut self){
        println!("< \"{}\" , {} >", self.tokenString, self.tt.to_string());
    }
}

//The structure for the tokenTable, which defines keywords and such, contains a hashmap that uses the string as key and the token as a value
pub struct tokenTable{
    tokTab: HashMap<String, Token>,
}

//The methods of the tokentable
impl tokenTable{
    //The constructor for creating a tokenTable
    fn new() -> tokenTable {
        //Creates the empty hash map
        let mut tokHash: HashMap<String, Token> = HashMap::new();

        //List of all of the tokens that should be in the symbol table when initializes
        //This defines all of the keywords in the program and creates the tokens and organizes them for their intended purpose
        let keywords = vec![
            ("if", Token::new(tokenTypeEnum::IF, "if".to_string(), "0".to_string(), tokenGroup::KEYWORD)),
            ("else", Token::new(tokenTypeEnum::ELSE, "else".to_string(), "0".to_string(), tokenGroup::KEYWORD)),
            ("procedure", Token::new(tokenTypeEnum::PROCEDURE, "procedure".to_string(), "0".to_string(), tokenGroup::KEYWORD)),
            ("is", Token::new(tokenTypeEnum::IS, "is".to_string(), "0".to_string(), tokenGroup::KEYWORD)),
            ("global", Token::new(tokenTypeEnum::GLOBAL, "global".to_string(), "0".to_string(), tokenGroup::KEYWORD)),
            ("variable", Token::new(tokenTypeEnum::VARIABLE, "variable".to_string(), "0".to_string(), tokenGroup::KEYWORD)),
            ("begin", Token::new(tokenTypeEnum::BEGIN, "begin".to_string(), "0".to_string(), tokenGroup::KEYWORD)),
            ("then", Token::new(tokenTypeEnum::THEN, "then".to_string(), "0".to_string(), tokenGroup::KEYWORD)),
            ("end", Token::new(tokenTypeEnum::END, "end".to_string(), "0".to_string(), tokenGroup::KEYWORD)),
            ("program", Token::new(tokenTypeEnum::PROGRAM, "program".to_string(), "0".to_string(), tokenGroup::KEYWORD)),
            ("return", Token::new(tokenTypeEnum::RETURN, "return".to_string(), "0".to_string(), tokenGroup::KEYWORD)),
            ("for", Token::new(tokenTypeEnum::FOR, "for".to_string(), "0".to_string(), tokenGroup::KEYWORD)),
            ("not", Token::new(tokenTypeEnum::NOT, "not".to_string(), "0".to_string(), tokenGroup::OPERATOR)),
            ("true", Token::new(tokenTypeEnum::TRUE, "true".to_string(), "0".to_string(), tokenGroup::CONSTANT)),
            ("false", Token::new(tokenTypeEnum::FALSE, "false".to_string(), "0".to_string(), tokenGroup::CONSTANT)),
        ];

        //Inserts all of the keywords into the table
        for (key, value) in keywords {
            tokHash.insert(key.to_string(), value);
        }

        //Prints the seeded tokenTable, used for debugging
        // println!("token table created and seeded");
        // for (key, token) in &mut symHash {
        //     println!("Key: {}, Token: {:?}", key, token.printToken());
        // }

        //Returns the tokentable
        tokenTable{
            tokTab: tokHash,
        }
    }
    
    //Checks if a given word is in the hashtable. Used to check if a word is a keyword
    //If not found, returns a new token with that string
    fn hashLook(&mut self, mut lookupString: String, line: String) -> Token{
        if let Some(tokenResp) = self.tokTab.get(&lookupString){
            return tokenResp.clone();
        } else {
            let newToken = Token::new(tokenTypeEnum::IDENTIFIER, lookupString, line.to_string(), tokenGroup::VARIABLE);
            self.tokTab.insert(newToken.tokenString.clone(), newToken.clone());
            return newToken;
        }
    }

}

//An enum used in conjunction with tokenType for parsing purposes
//Defines the tokens into groups like constants, operators, etc.
#[derive(Clone, PartialEq)]
pub enum tokenGroup{
    OPERATOR,
    KEYWORD,
    VARIABLE,
    OTHER,
    SYMBOL,
    CONSTANT,
}

//Display for tokenGroup, defines how the tokenGroup enum instances are printed (mostly for debugging purposes)
impl fmt::Display for tokenGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let variant_str = match self {
            &tokenGroup::OPERATOR => "OPERATOR",
            &tokenGroup::KEYWORD => "KEYWORD",
            &tokenGroup::VARIABLE => "VARIABLE",
            &tokenGroup::OTHER => "OTHER",
            &tokenGroup::SYMBOL => "SYMBOL",
            &tokenGroup::CONSTANT => "NUMBER",

        };
        write!(f, "{}", variant_str)
    }
}

//Used to print an entire list of tokens (This is for debugging)
pub fn printTokList(tokList: &Vec<Token>){
    for token in tokList {
        println!("< \"{}\" , {}, {} >", token.tokenString, token.tt.to_string(), token.lineNum);
    }
}

///////////////////////// /LEXER SECTION /////////////////////////


