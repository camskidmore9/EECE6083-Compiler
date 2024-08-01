; ModuleID = 'my_module'
source_filename = "my_module"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128"

@y = global i32 0
@tmp = global i1 false

declare i32 @getinteger()

declare i1 @putinteger(i32)

declare float @getfloat()

declare i1 @putbool(i1)

declare i1 @putfloat(float)

define i32 @main() {
mainEntry:
  %y = load i32, i32* @y, align 4
  %callProc = call i32 @"0proc1"(i32 %y)
  store i32 %callProc, i32* @y, align 4
  %y1 = load i32, i32* @y, align 4
  %callProc2 = call i1 @putinteger(i32 %y1)
  store i1 %callProc2, i1* @tmp, align 1
  ret i32 0
}

define i32 @"0proc1"(i32 %0) {
procEntry:
  %val = alloca i32, align 4
  store i32 %0, i32* %val, align 4
  br label %procBody

procBody:                                         ; preds = %procEntry
  %val1 = load i32, i32* %val, align 4
  %addInt = add i32 %val1, 1
  store i32 %addInt, i32* %val, align 4
  %val2 = load i32, i32* %val, align 4
  %callProc = call i32 @"1proc2"(i32 %val2)
  store i32 %callProc, i32* %val, align 4
  %val3 = load i32, i32* %val, align 4
  ret i32 %val3
}

define i32 @"1proc2"(i32 %0) {
procEntry:
  %val = alloca i32, align 4
  store i32 %0, i32* %val, align 4
  br label %procBody

procBody:                                         ; preds = %procEntry
  %val1 = load i32, i32* %val, align 4
  %addInt = add i32 %val1, 1
  store i32 %addInt, i32* %val, align 4
  %val2 = load i32, i32* %val, align 4
  %callProc = call i32 @"2proc1"(i32 %val2)
  store i32 %callProc, i32* %val, align 4
  %val3 = load i32, i32* %val, align 4
  ret i32 %val3
}

define i32 @"2proc1"(i32 %0) {
procEntry:
  %val = alloca i32, align 4
  store i32 %0, i32* %val, align 4
  br label %procBody

procBody:                                         ; preds = %procEntry
  %val1 = load i32, i32* %val, align 4
  %addInt = add i32 %val1, 1
  store i32 %addInt, i32* %val, align 4
  %val2 = load i32, i32* %val, align 4
  ret i32 %val2
}
