//! Standalone persistent KV/Document plugin executable (stdio binary IPC v1).
use polydb::{Command, DataModel, Engine, Response, WalEngine};
use polydb::wire::{self, Message};
use std::io::{stdin, stdout, BufReader, BufWriter};

fn as_command(msg: &Message) -> polydb::Result<Command> {
    use wire::*;
    let a = &msg.args;
    let arg = |i: usize| -> polydb::Result<String> {
        str_arg(a.get(i).ok_or_else(|| polydb::DbError::Invalid(format!("missing argument {i}")))?)
    };
    match (msg.opcode, a.len()) {
        (PING, 0) => Ok(Command::Ping),
        (PUT, 3) => Ok(Command::Put { namespace: arg(0)?, key: arg(1)?, value: a[2].clone() }),
        (GET, 2) => Ok(Command::Get { namespace: arg(0)?, key: arg(1)? }),
        (DELETE, 2) => Ok(Command::Delete { namespace: arg(0)?, key: arg(1)? }),
        (SCAN, 2) => Ok(Command::Scan { namespace: arg(0)?, prefix: arg(1)? }),
        _ => Err(polydb::DbError::Unsupported("unsupported opcode/arity".into())),
    }
}

fn handle(engine: &WalEngine, message: Message) -> Message {
    use wire::*;
    let result = as_command(&message).and_then(|c| engine.execute(c));
    match result {
        Ok(Response::Ack) => Message { opcode: OK, args: vec![] },
        Ok(Response::Value(Some(value))) => Message { opcode: OK, args: vec![value] },
        Ok(Response::Value(None)) => Message { opcode: NONE, args: vec![] },
        Ok(Response::Items(items)) => {
            let args = items.into_iter().flat_map(|(k, v)| [k.into_bytes(), v]).collect();
            Message { opcode: OK, args }
        }
        Ok(_) => Message { opcode: ERROR, args: vec![b"unsupported response".to_vec()] },
        Err(error) => Message { opcode: ERROR, args: vec![error.to_string().into_bytes()] },
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: polydb-worker <kv|document> <wal-path>");
        std::process::exit(2);
    }
    let model = DataModel::parse(&args[1]).ok_or("unsupported model")?;
    let engine = WalEngine::open(&args[2], model)?;
    let mut input = BufReader::new(stdin().lock());
    let mut output = BufWriter::new(stdout().lock());
    while let Some(msg) = wire::read_message(&mut input)? {
        wire::write_message(&mut output, &handle(&engine, msg))?;
    }
    Ok(())
}
