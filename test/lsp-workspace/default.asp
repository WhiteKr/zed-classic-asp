<!--#include file="includes/header.asp"-->
<!--#include virtual="/lib/db.asp"-->
<!--#include file="missing.asp"-->
<p>한글 텍스트 뒤 include 위치 검증</p> <!--#include file="없는파일.asp"--> <!--#include file="includes/header.asp"-->
<%
Dim msg
msg = GetGreeting("world")
Response.Write msg
Call RenderHeader
Set rs = OpenConn("select 1")
rs.MoveNext
%>
<!--#include file="/lib/db.asp"-->
