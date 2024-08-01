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
