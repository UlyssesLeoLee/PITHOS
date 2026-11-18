use crate::{Command, DataModel, DbError, Descriptor, Engine, Response, Result};
use crate::model::PROTOCOL_VERSION;
use crate::wire::{self, Message};
use std::io::{BufReader, BufWriter};
use std::process::{Child, ChildStdin, ChildStdout, Command as OsCommand, Stdio};
use std::sync::Mutex;

struct Pipes {
    child: Child,
    input: BufWriter<ChildStdin>,
    output: BufReader<ChildStdout>,
}
impl Drop for Pipes {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Untrusted plugins can be operated out of process, with one serialized IPC channel.
/// Limitation: V1 IPC has no request deadline; a hanging worker must be supervised externally.
pub struct ProcessEngine {
    descriptor: Descriptor,
    pipes: Mutex<Pipes>,
}

impl ProcessEngine {
    pub fn spawn(program: &str, args: &[&str], name: &str, model: DataModel, storage_id: &str) -> Result<Self> {
        let mut child = OsCommand::new(program).args(args)
            .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit())
            .spawn()?;
        let stdin = child.stdin.take().ok_or_else(|| DbError::Plugin("stdin unavailable".into()))?;
        let stdout = child.stdout.take().ok_or_else(|| DbError::Plugin("stdout unavailable".into()))?;
        let engine = Self {
            descriptor: Descriptor { name: name.into(), model, protocol: PROTOCOL_VERSION,
                storage_id: storage_id.into() },
            pipes: Mutex::new(Pipes { child, input: BufWriter::new(stdin), output: BufReader::new(stdout) }),
        };
        engine.health()?;
        Ok(engine)
    }

    fn send(&self, msg: Message) -> Result<Message> {
        let mut pipes = self.pipes.lock().map_err(|_| DbError::Plugin("worker lock poisoned".into()))?;
        let Pipes { input, output, .. } = &mut *pipes;
        wire::write_message(input, &msg)?;
        wire::read_message(output)?.ok_or_else(|| DbError::Plugin("worker closed stdout".into()))
    }
}

impl Engine for ProcessEngine {
    fn descriptor(&self) -> Descriptor { self.descriptor.clone() }
    fn execute(&self, command: Command) -> Result<Response> {
        use wire::*;
        let (op, args) = match &command {
            Command::Ping => (PING, vec![]),
            Command::Put { namespace, key, value } => (PUT, vec![namespace.as_bytes().to_vec(), key.as_bytes().to_vec(), value.clone()]),
            Command::Get { namespace, key } => (GET, vec![namespace.as_bytes().to_vec(), key.as_bytes().to_vec()]),
            Command::Delete { namespace, key } => (DELETE, vec![namespace.as_bytes().to_vec(), key.as_bytes().to_vec()]),
            Command::Scan { namespace, prefix } => (SCAN, vec![namespace.as_bytes().to_vec(), prefix.as_bytes().to_vec()]),
            Command::SqlExec { sql } => (SQL_EXEC, vec![sql.as_bytes().to_vec()]),
            Command::SqlQuery { sql } => (SQL_QUERY, vec![sql.as_bytes().to_vec()]),
        };
        let response = self.send(Message { opcode: op, args })?;
        if response.opcode == ERROR {
            let details = response.args.first().map(|v| str_arg(v)).transpose()?.unwrap_or_default();
            return Err(DbError::Plugin(details));
        }
        if response.opcode == NONE {
            return match command { Command::Get {..} => Ok(Response::Value(None)),
                _ => Err(DbError::Plugin("unexpected NONE response".into())) };
        }
        if response.opcode != OK { return Err(DbError::Plugin("unknown worker response".into())); }
        match command {
            Command::Ping | Command::Put {..} | Command::Delete {..} | Command::SqlExec {..} => {
                if !response.args.is_empty() { return Err(DbError::Plugin("expected empty ACK".into())); }
                Ok(Response::Ack)
            }
            Command::Get {..} => {
                if response.args.len() != 1 { return Err(DbError::Plugin("expected one value".into())); }
                Ok(Response::Value(Some(response.args[0].clone())))
            }
            Command::Scan {..} => {
                if response.args.len() % 2 != 0 { return Err(DbError::Plugin("invalid scan item count".into())); }
                let mut items = Vec::new();
                for pair in response.args.chunks_exact(2) {
                    items.push((str_arg(&pair[0])?, pair[1].clone()));
                }
                Ok(Response::Items(items))
            }
            Command::SqlQuery {..} => {
                if response.args.len() != 1 { return Err(DbError::Plugin("expected serialized SQL query result".into())); }
                Ok(Response::Text(str_arg(&response.args[0])?))
            }
        }
    }
}
