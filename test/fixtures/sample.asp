<%@ Language="VBScript" %>
<!--#include file="includes/header.asp"-->
<!--#include virtual="/lib/db.asp"-->
<%
' page setup
Option Explicit
Dim userName, itemCount
dim lowercase_style
DIM UPPER_STYLE
Set conn = Server.CreateObject("ADODB.Connection")
Const PAGE_SIZE = 20

Function GetGreeting(name)
    If Len(name) > 0 Then
        GetGreeting = "Hello, " & name & "!"
    ElseIf IsNull(name) Then
        GetGreeting = "Hello, ""stranger""!"
    Else
        GetGreeting = "Hello!"
    End If
End Function

Sub RenderRow(label, value)
    Response.Write "<tr><td>" & label & "</td><td>" & value & "</td></tr>"
End Sub

Class Cart
    Private m_total

    Property Get Total
        Total = m_total
    End Property

    Property Let Total(value)
        m_total = value
    End Property
End Class
%>
<!DOCTYPE html>
<html>
<head>
    <title>Fixture Page</title>
</head>
<body>
    <h1><%= GetGreeting(userName) %></h1>

    <table>
        <%
        For Each item In Session("cart")
            RenderRow item, item * 2
        Next

        Do While itemCount < PAGE_SIZE
            itemCount = itemCount + 1
        Loop

        Select Case userName
            Case "admin"
                Response.Write "<p>admin tools</p>"
            Case Else
                Response.Write "<p>guest view</p>"
        End Select
        %>
    </table>

    <script runat="server" language="VBScript">
        ' not highlighted in phase 1 -- must not break the page
        Sub ServerSideThing()
        End Sub
    </script>

    <p>Count: <%= itemCount %></p>
</body>
</html>
