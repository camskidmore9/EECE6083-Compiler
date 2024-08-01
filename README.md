# EECE6083-Compiler
A compiler for a custom programming language written in Rust

PREREQUESITES:
RustC version 1.75.0 or similar
Cargo version 1.75.0 or similar
LLVM version 18 installed and added to path

COMPILING THE COMPILER:
To build the compiler, run cargo build

RUNNING THE COMPILER:
In the main directory, run "cargo run /path/to/file/" where /path/to/file/ is the .src file to be compiled.

This will generate 2 files called output.o and a.out.

-output.o is the object file of the compiled code, not linked
-a.out is the linux executable file of the compiled program

RUNNING PROGRAM:
./a.out

PROJECT STRUCTURE:
The main project code is located in ./src/

main.rs is the main file that bring all components together, takes input for the source program, and compiles the code

Each stage of the compiler (excluding linking) has its own file and Rust struct. These are all located in /src/models/ and are named according to their function

The built in functions are defined in a library located in /funcLib/src/main.rs
The compiled version of this library is located at /funcLib.a. This file is what main.rs for the compiler calls. **If this is removed, the linker will not run.**

To recompile the library, run "cargo build --release" this will generate the file /target/relrease/libFuncLib.a. Move that file to the main directory of the project and rename it "funcLib.a" and the code will run.

Reach out with any questions.
