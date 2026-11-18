//! Length-prefixed binary IPC; no tabs/newlines or delimiter injection.
//! Frame: u32 payload size (LE), u16 protocol, u8 opcode, u8 argument_count,
//! repeated (u32 byte_length, bytes). Max frame = 8 MiB.
use crate::{DbError, Result};
use crate::model::PROTOCOL_VERSION;
use std::io::{Read, Write};

pub const MAX_FRAME: usize = 8 * 1024 * 1024;
pub const PING: u8 = 1;
pub const PUT: u8 = 2;
pub const GET: u8 = 3;
pub const DELETE: u8 = 4;
pub const SCAN: u8 = 5;
pub const SQL_EXEC: u8 = 6;
pub const SQL_QUERY: u8 = 7;
pub const OK: u8 = 100;
pub const ERROR: u8 = 101;
pub const NONE: u8 = 102;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Message { pub opcode: u8, pub args: Vec<Vec<u8>> }

pub fn write_message(mut out: impl Write, msg: &Message) -> Result<()> {
    if msg.args.len() > u8::MAX as usize { return Err(DbError::Invalid("too many IPC arguments".into())); }
    let size: usize = 4 + msg.args.iter().map(|a| 4 + a.len()).sum::<usize>();
    if size > MAX_FRAME { return Err(DbError::Invalid("IPC frame exceeds 8 MiB".into())); }
    out.write_all(&(size as u32).to_le_bytes())?;
    out.write_all(&PROTOCOL_VERSION.to_le_bytes())?;
    out.write_all(&[msg.opcode, msg.args.len() as u8])?;
    for arg in &msg.args {
        out.write_all(&(arg.len() as u32).to_le_bytes())?;
        out.write_all(arg)?;
    }
    out.flush()?;
    Ok(())
}

pub fn read_message(mut input: impl Read) -> Result<Option<Message>> {
    let mut size = [0u8; 4];
    let first = input.read(&mut size[..1])?;
    if first == 0 { return Ok(None); }
    input.read_exact(&mut size[1..])?;
    let size = u32::from_le_bytes(size) as usize;
    if !(4..=MAX_FRAME).contains(&size) { return Err(DbError::Invalid("invalid IPC frame size".into())); }
    let mut buf = vec![0u8; size];
    input.read_exact(&mut buf)?;
    let ver = u16::from_le_bytes(buf[0..2].try_into().unwrap());
    if ver != PROTOCOL_VERSION { return Err(DbError::Incompatible("IPC protocol mismatch".into())); }
    let opcode = buf[2];
    let count = buf[3] as usize;
    let mut args = Vec::with_capacity(count);
    let mut cursor = 4usize;
    for _ in 0..count {
        if size - cursor < 4 { return Err(DbError::Invalid("truncated IPC argument header".into())); }
        let len = u32::from_le_bytes(buf[cursor..cursor + 4].try_into().unwrap()) as usize;
        cursor += 4;
        if len > size - cursor { return Err(DbError::Invalid("truncated IPC argument".into())); }
        args.push(buf[cursor..cursor + len].to_vec());
        cursor += len;
    }
    if cursor != size { return Err(DbError::Invalid("IPC trailing bytes".into())); }
    Ok(Some(Message { opcode, args }))
}

pub fn str_arg(bytes: &[u8]) -> Result<String> {
    String::from_utf8(bytes.to_vec()).map_err(|_| DbError::Invalid("invalid UTF-8 IPC argument".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn wire_round_trip() {
        let msg = Message { opcode: PUT, args: vec![b"a\0b\n".to_vec(), vec![0,1,2,3]] };
        let mut bytes = Vec::new();
        write_message(&mut bytes, &msg).unwrap();
        assert_eq!(read_message(bytes.as_slice()).unwrap(), Some(msg));
    }
    #[test]
    fn oversized_frame_is_rejected() {
        let bytes = ((MAX_FRAME as u32) + 1).to_le_bytes();
        assert!(read_message(bytes.as_slice()).is_err());
    }
}
