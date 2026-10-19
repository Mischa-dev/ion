//! Password files other browsers and managers export: Chrome, Edge, Brave,
//! Firefox, Safari, Bitwarden and 1Password all write a CSV with a header row
//! naming the URL, username and password columns.

/// One login read from a CSV export.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub url: String,
    pub username: String,
    pub password: String,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CsvError {
    /// No header row naming a URL and a password column.
    UnknownFormat,
}

impl std::fmt::Display for CsvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("not a password export: no url and password columns")
    }
}

impl std::error::Error for CsvError {}

/// The logins in a CSV export, skipping rows without a URL or password.
pub fn parse(text: &str) -> Result<Vec<Row>, CsvError> {
    let mut records = records(text.trim_start_matches('\u{feff}')).into_iter();
    let header: Vec<String> = records
        .next()
        .ok_or(CsvError::UnknownFormat)?
        .iter()
        .map(|h| h.trim().to_ascii_lowercase())
        .collect();
    let column = |names: &[&str]| header.iter().position(|h| names.contains(&h.as_str()));
    let url = column(&[
        "url",
        "login_uri",
        "website",
        "web site",
        "origin",
        "hostname",
    ])
    .ok_or(CsvError::UnknownFormat)?;
    let password = column(&["password", "login_password"]).ok_or(CsvError::UnknownFormat)?;
    let username = column(&[
        "username",
        "login_username",
        "user",
        "login",
        "email",
        "user name",
    ]);
    let note = column(&["note", "notes", "comment"]);
    let field = |record: &[String], i: Option<usize>| {
        i.and_then(|i| record.get(i)).cloned().unwrap_or_default()
    };
    Ok(records
        .map(|record| Row {
            url: field(&record, Some(url)).trim().to_owned(),
            username: field(&record, username),
            password: field(&record, Some(password)),
            note: field(&record, note),
        })
        .filter(|row| !row.url.is_empty() && !row.password.is_empty())
        .collect())
}

/// Logins as a CSV in Chrome's export format (`name,url,username,password,note`),
/// which every browser and password manager imports.
pub fn write(rows: &[(String, Row)]) -> String {
    let mut out = String::from("name,url,username,password,note\n");
    for (name, row) in rows {
        let fields = [name, &row.url, &row.username, &row.password, &row.note];
        let line: Vec<String> = fields.iter().map(|f| quote(f)).collect();
        out.push_str(&line.join(","));
        out.push('\n');
    }
    out
}

fn quote(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_owned()
    }
}

/// RFC 4180 records: quoted fields may hold commas, quotes ("") and newlines.
fn records(text: &str) -> Vec<Vec<String>> {
    let mut records = Vec::new();
    let mut record = Vec::new();
    let mut field = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                chars.next();
                field.push('"');
            }
            '"' if quoted => quoted = false,
            '"' if field.is_empty() => quoted = true,
            ',' if !quoted => record.push(std::mem::take(&mut field)),
            '\r' if !quoted => {}
            '\n' if !quoted => {
                record.push(std::mem::take(&mut field));
                if record.iter().any(|f| !f.is_empty()) {
                    records.push(std::mem::take(&mut record));
                }
                record.clear();
            }
            c => field.push(c),
        }
    }
    record.push(field);
    if record.iter().any(|f| !f.is_empty()) {
        records.push(record);
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_chrome_exports() {
        let text = "name,url,username,password,note\n\
                    example.com,https://example.com/login,alice,\"p,ss\"\"word\",\"two\nlines\"\r\n\
                    empty,https://empty.com,bob,,\n";
        let rows = parse(text).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].url, "https://example.com/login");
        assert_eq!(rows[0].username, "alice");
        assert_eq!(rows[0].password, "p,ss\"word");
        assert_eq!(rows[0].note, "two\nlines");
    }

    #[test]
    fn reads_firefox_safari_and_bitwarden_exports() {
        let firefox = "\u{feff}\"url\",\"username\",\"password\",\"httpRealm\",\"formActionOrigin\",\"guid\"\n\
                       \"https://a.com\",\"me\",\"pw\",,\"https://a.com\",\"{1}\"\n";
        assert_eq!(parse(firefox).unwrap()[0].username, "me");
        let safari = "Title,URL,Username,Password,Notes,OTPAuth\nA,https://b.com/,you,secret,hi,\n";
        let row = &parse(safari).unwrap()[0];
        assert_eq!(
            (row.url.as_str(), row.note.as_str()),
            ("https://b.com/", "hi")
        );
        let bitwarden = "folder,favorite,type,name,notes,fields,reminder_prompt,login_uri,login_username,login_password,login_totp\n\
                         ,,login,C,,,,https://c.com,them,pw3,\n";
        assert_eq!(parse(bitwarden).unwrap()[0].password, "pw3");
        assert_eq!(parse("a,b\n1,2\n"), Err(CsvError::UnknownFormat));
        assert_eq!(parse(""), Err(CsvError::UnknownFormat));
    }

    #[test]
    fn writes_what_it_reads() {
        let row = Row {
            url: "https://a.com".into(),
            username: "=cmd".into(),
            password: "a\"b,c".into(),
            note: String::new(),
        };
        let text = write(&[("a.com".into(), row.clone())]);
        assert_eq!(parse(&text).unwrap(), vec![row]);
    }
}
