//! Hover documentation for Classic ASP built-in objects and common ADO members.

/// Docs for `Object.Member` where the qualifier is one of the five intrinsic objects.
pub fn member_doc(object: &str, member: &str) -> Option<String> {
    let object = object.to_ascii_lowercase();
    let member = member.to_ascii_lowercase();
    let (title, doc) = match (object.as_str(), member.as_str()) {
        ("response", "write") => ("Response.Write(value)", "Writes a string to the HTTP response body."),
        ("response", "redirect") => ("Response.Redirect(url)", "Sends a 302 redirect to the client and ends processing of the page output."),
        ("response", "end") => ("Response.End", "Stops processing the page and returns the buffered output."),
        ("response", "buffer") => ("Response.Buffer = True|False", "Whether output is buffered until the page finishes (must be set before any output)."),
        ("response", "flush") => ("Response.Flush", "Sends buffered output to the client immediately."),
        ("response", "clear") => ("Response.Clear", "Erases the buffered output without sending it."),
        ("response", "contenttype") => ("Response.ContentType = \"text/html\"", "Sets the MIME type of the response."),
        ("response", "charset") => ("Response.Charset = \"utf-8\"", "Appends the charset name to the Content-Type header."),
        ("response", "cookies") => ("Response.Cookies(name) = value", "Sets a cookie to send to the client."),
        ("response", "expires") => ("Response.Expires = minutes", "Minutes before a cached page expires."),
        ("response", "addheader") => ("Response.AddHeader name, value", "Adds a custom HTTP response header."),
        ("response", "isclientconnected") => ("Response.IsClientConnected", "True while the client is still connected."),
        ("request", "querystring") => ("Request.QueryString(name)", "Reads a value from the URL query string."),
        ("request", "form") => ("Request.Form(name)", "Reads a value from a POSTed form body."),
        ("request", "cookies") => ("Request.Cookies(name)", "Reads a cookie sent by the client."),
        ("request", "servervariables") => ("Request.ServerVariables(name)", "Reads a server/CGI variable, e.g. \"REMOTE_ADDR\", \"HTTP_USER_AGENT\"."),
        ("request", "totalbytes") => ("Request.TotalBytes", "Number of bytes in the request body."),
        ("request", "binaryread") => ("Request.BinaryRead(count)", "Reads raw bytes from the request body."),
        ("server", "createobject") => ("Server.CreateObject(progId)", "Creates a COM object, e.g. Server.CreateObject(\"ADODB.Connection\")."),
        ("server", "mappath") => ("Server.MapPath(virtualPath)", "Maps a virtual path to a physical filesystem path."),
        ("server", "htmlencode") => ("Server.HTMLEncode(text)", "HTML-escapes a string (&, <, >, quotes)."),
        ("server", "urlencode") => ("Server.URLEncode(text)", "URL-encodes a string for use in a query string."),
        ("server", "execute") => ("Server.Execute(path)", "Executes another ASP page, then returns to this one."),
        ("server", "transfer") => ("Server.Transfer(path)", "Transfers control to another ASP page without returning."),
        ("server", "scripttimeout") => ("Server.ScriptTimeout = seconds", "Maximum time a script may run before being terminated."),
        ("session", "sessionid") => ("Session.SessionID", "Unique identifier of the current session."),
        ("session", "timeout") => ("Session.Timeout = minutes", "Idle minutes before the session is abandoned."),
        ("session", "abandon") => ("Session.Abandon", "Destroys the session and releases its resources."),
        ("session", "contents") => ("Session.Contents(name)", "Collection of all values stored in the session."),
        ("session", "codepage") => ("Session.CodePage = codepage", "Code page used for string conversions, e.g. 65001 for UTF-8."),
        ("session", "lcid") => ("Session.LCID = localeId", "Locale identifier used for date/number formatting."),
        ("application", "lock") => ("Application.Lock", "Blocks other clients from modifying Application values."),
        ("application", "unlock") => ("Application.Unlock", "Releases a previous Application.Lock."),
        ("application", "contents") => ("Application.Contents(name)", "Collection of all values stored application-wide."),
        _ => return None,
    };
    Some(format!("**`{title}`**\n\n{doc}"))
}

/// Docs for the intrinsic objects themselves.
pub fn object_doc(name: &str) -> Option<String> {
    let (title, doc) = match name.to_ascii_lowercase().as_str() {
        "response" => ("Response", "Intrinsic ASP object for the outgoing HTTP response. Common members: `Write`, `Redirect`, `End`, `Buffer`, `ContentType`, `Cookies`."),
        "request" => ("Request", "Intrinsic ASP object for the incoming HTTP request. Common members: `QueryString`, `Form`, `Cookies`, `ServerVariables`."),
        "server" => ("Server", "Intrinsic ASP utility object. Common members: `CreateObject`, `MapPath`, `HTMLEncode`, `URLEncode`, `Execute`, `Transfer`."),
        "session" => ("Session", "Per-user state that persists across requests. Common members: `SessionID`, `Timeout`, `Abandon`, `Contents`."),
        "application" => ("Application", "Application-wide shared state. Common members: `Lock`, `Unlock`, `Contents`."),
        _ => return None,
    };
    Some(format!("**`{title}`**\n\n{doc}"))
}

/// Heuristic docs for distinctive ADO members on arbitrary qualifiers
/// (`rs.MoveNext`, `conn.BeginTrans`, ...). Only members that are unlikely
/// to collide with user code are listed.
pub fn ado_member_doc(member: &str) -> Option<String> {
    let (title, doc) = match member.to_ascii_lowercase().as_str() {
        "movenext" => ("Recordset.MoveNext", "Moves to the next record; check `EOF` afterwards."),
        "moveprevious" => ("Recordset.MovePrevious", "Moves to the previous record; check `BOF` afterwards."),
        "movefirst" => ("Recordset.MoveFirst", "Moves to the first record."),
        "movelast" => ("Recordset.MoveLast", "Moves to the last record."),
        "eof" => ("Recordset.EOF", "True when positioned past the last record (also true for an empty recordset)."),
        "bof" => ("Recordset.BOF", "True when positioned before the first record."),
        "recordcount" => ("Recordset.RecordCount", "Number of records; -1 with a forward-only cursor."),
        "addnew" => ("Recordset.AddNew", "Starts a new record; call `Update` to save it."),
        "connectionstring" => ("Connection.ConnectionString", "OLE DB connection string used by `Open`."),
        "begintrans" => ("Connection.BeginTrans", "Begins a transaction."),
        "committrans" => ("Connection.CommitTrans", "Commits the current transaction."),
        "rollbacktrans" => ("Connection.RollbackTrans", "Rolls back the current transaction."),
        "absolutepage" => ("Recordset.AbsolutePage", "1-based page number of the current record (requires `PageSize`)."),
        "pagesize" => ("Recordset.PageSize", "Records per page, used for paging with `AbsolutePage`."),
        _ => return None,
    };
    Some(format!("**`ADODB.{title}`**\n\n{doc}"))
}
