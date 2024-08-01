; ModuleID = 'my_module'
source_filename = "my_module"
target datalayout = "e-m:e-p270:32:32-p271:32:32-p272:64:64-i64:64-f80:128-n8:16:32:64-S128"

@value = global i32 0
@tmp2 = global float 0.000000e+00
@out = global i1 false
@floatval = global float 0.000000e+00
@i = global i32 0

declare i32 @getinteger()

declare i1 @putinteger(i32)

declare float @getfloat()

declare i1 @putbool(i1)

declare i1 @putfloat(float)

define i32 @main() {
mainEntry:
  store i32 1, i32* @i, align 4
  %i = load i32, i32* @i, align 4
  %callProc = call i1 @putinteger(i32 %i)
  br label %forCond

forCond:                                          ; preds = %forBody, %mainEntry
  %i1 = load i32, i32* @i, align 4
  %forLoopCondition = icmp slt i32 %i1, 10
  br i1 %forLoopCondition, label %forBody, label %mergeFor

forBody:                                          ; preds = %forCond
  %i2 = load i32, i32* @i, align 4
  %callProc3 = call i1 @putinteger(i32 %i2)
  %i4 = load i32, i32* @i, align 4
  %addInt = add i32 %i4, 1
  store i32 %addInt, i32* @i, align 4
  br label %forCond

mergeFor:                                         ; preds = %forCond
  store i1 true, i1* @out, align 1
  %i5 = load i32, i32* @i, align 4
  %callProc6 = call i1 @putinteger(i32 %i5)
  ret i32 0
}
