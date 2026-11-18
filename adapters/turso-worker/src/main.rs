//! Experimental Turso 0.8.3 process adapter. Build separately; not part of default workspace.
//! SQL comes from a local trusted CLI in V1. No public RPC endpoint or parameter binding yet.
use polydb::wire::{self, Message};
use std::io::{BufReader, BufWriter, stdin, stdout};

fn error_response(e: impl std::fmt::Display) -> Message {
    Message { opcode: wire::ERROR, args: vec![e.to_string().into_bytes()] }
}
fn json_escape(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < '\u{20}' => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

async fn handle(conn: &turso::Connection, m: Message) -> Message {
    let answer: Result<Vec<Vec<u8>>, String> = async {
        match (m.opcode, m.args.as_slice()) {
            (wire::PING, []) => Ok(vec![]),
            (wire::SQL_EXEC, [sql]) => {
                let sql = wire::str_arg(sql).map_err(|e| e.to_string())?;
                conn.execute(&sql, ()).await.map_err(|e| e.to_string())?;
                Ok(vec![])
            }
            (wire::SQL_QUERY, [sql]) => {
                let sql = wire::str_arg(sql).map_err(|e| e.to_string())?;
                let mut rows = conn.query(&sql, ()).await.map_err(|e| e.to_string())?;
                let column_count = rows.column_count();
                let headers = rows.column_names();
                let mut out = String::from("{\"columns\":[");
                for (i, name) in headers.iter().enumerate() {
                    if i > 0 { out.push(','); }
                    out.push_str(&json_escape(name));
                }
                out.push_str("],\"rows\":[");
                let mut first_row = true;
                while let Some(row) = rows.next().await.map_err(|e| e.to_string())? {
                    if !first_row { out.push(','); }
                    first_row = false;
                    out.push('[');
                    for idx in 0..column_count {
                        if idx > 0 { out.push(','); }
                        // V1 exposes SQL cell values as DEBUG TEXT, not typed JSON.
                        let value = row.get_value(idx).map_err(|e| e.to_string())?;
                        out.push_str(&json_escape(&format!("{value:?}")));
                    }
                    out.push(']');
                    if out.len() > 6 * 1024 * 1024 { return Err("query result exceeds 6 MiB".into()); }
                }
                out.push_str("]}");
                Ok(vec![out.into_bytes()])
            }
            _ => Err("unsupported SQL adapter opcode/arity".into()),
        }
    }.await;
    match answer {
        Ok(args) => Message { opcode: wire::OK, args },
        Err(e) => error_response(e),
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: polydb-turso-worker <database-file>");
        std::process::exit(2);
    }
    let database = turso::Builder::new_local(&args[1]).build().await?;
    let connection = database.connect()?;
    let mut reader = BufReader::new(stdin().lock());
    let mut writer = BufWriter::new(stdout().lock());
    while let Some(request) = wire::read_message(&mut reader)? {
        wire::write_message(&mut writer, &handle(&connection, request).await)?;
    }
    Ok(())
}
