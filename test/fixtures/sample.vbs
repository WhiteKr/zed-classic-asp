' standalone VBScript file — highlighted without the embedded-template wrapper
Option Explicit

Dim total, i
total = 0

For i = 1 To 10
    total = total + i
Next

Function Describe(value)
    If IsNumeric(value) Then
        Describe = "number: " & CStr(value)
    Else
        Describe = "other: ""'" & value & "'"""
    End If
End Function

MsgBox Describe(total) & vbCrLf & Now
