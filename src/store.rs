use crate::{Command, DataModel, DbError, Descriptor, Engine, Response, Result};
use crate::model::PROTOCOL_VERSION;
use std::collections::BTreeMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const MAGIC: &[u8; 4] = b"PDB1";
const MAX_PAYLOAD: usize = 4 * 1024 * 1024;
type Map = BTreeMap<(String, String), Vec<u8>>;

struct StoreState { file: File, values: Map }

/// Educational append-only log. All writes sync before acknowledgment.
/// It intentionally fails closed on a torn tail or CRC failure.
pub struct WalEngine {
    model: DataModel,
    path: PathBuf,
    state: Mutex<StoreState>,
}

impl WalEngine {
    pub fn open(path: impl AsRef<Path>, model: DataModel) -> Result<Self> {
        if !matches!(model, DataModel::KeyValue | DataModel::Document) {
            return Err(DbError::Unsupported("WAL adapter only supports kv/document".into()));
        }
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() { std::fs::create_dir_all(parent)?; }
        }
        let mut file = OpenOptions::new().create(true).read(true).append(true).open(&path)?;
        let mut bytes = Vec::new();
        file.read_to_end(&mut bytes)?;
        let values = replay(&bytes)?;
        Ok(Self { model, path, state: Mutex::new(StoreState { file, values }) })
    }

    pub fn storage_identity(&self) -> String {
        std::fs::canonicalize(&self.path)
            .unwrap_or_else(|_| self.path.clone()).to_string_lossy().into_owned()
    }

    fn record(&self, op: u8, namespace: &str, key: &str, value: &[u8]) -> Result<()> {
        if namespace.is_empty() || key.is_empty() {
            return Err(DbError::Invalid("namespace and key must not be empty".into()));
        }
        let payload = encode_record(op, namespace, key, value)?;
        let mut state = self.state.lock().map_err(|_| DbError::Plugin("poisoned store lock".into()))?;
        state.file.write_all(MAGIC)?;
        state.file.write_all(&(payload.len() as u32).to_le_bytes())?;
        state.file.write_all(&payload)?;
        state.file.write_all(&crc32(&payload).to_le_bytes())?;
        state.file.sync_all()?;
        let id = (namespace.to_owned(), key.to_owned());
        if op == 1 { state.values.insert(id, value.to_vec()); }
        else { state.values.remove(&id); }
        Ok(())
    }
}

impl Engine for WalEngine {
    fn descriptor(&self) -> Descriptor {
        Descriptor { name: format!("wal-{}", self.model.token()), model: self.model,
            protocol: PROTOCOL_VERSION, storage_id: self.storage_identity() }
    }
    fn execute(&self, command: Command) -> Result<Response> {
        match command {
            Command::Ping => Ok(Response::Ack),
            Command::Put { namespace, key, value } => {
                self.record(1, &namespace, &key, &value)?;
                Ok(Response::Ack)
            }
            Command::Delete { namespace, key } => {
                self.record(2, &namespace, &key, &[])?;
                Ok(Response::Ack)
            }
            Command::Get { namespace, key } => {
                let state = self.state.lock().map_err(|_| DbError::Plugin("poisoned store lock".into()))?;
                Ok(Response::Value(state.values.get(&(namespace, key)).cloned()))
            }
            Command::Scan { namespace, prefix } => {
                let state = self.state.lock().map_err(|_| DbError::Plugin("poisoned store lock".into()))?;
                let rows = state.values.iter()
                    .filter(|((ns, k), _)| ns == &namespace && k.starts_with(&prefix))
                    .map(|((_, k), v)| (k.clone(), v.clone())).collect();
                Ok(Response::Items(rows))
            }
            _ => Err(DbError::Unsupported("SQL is not supported by the WAL adapter".into())),
        }
    }
}

fn encode_record(op: u8, namespace: &str, key: &str, value: &[u8]) -> Result<Vec<u8>> {
    if op != 1 && op != 2 { return Err(DbError::Invalid("unknown WAL operation".into())); }
    let n = namespace.as_bytes();
    let k = key.as_bytes();
    if n.len() > u16::MAX as usize || k.len() > u32::MAX as usize || value.len() > u32::MAX as usize {
        return Err(DbError::Invalid("field too long".into()));
    }
    let total = 1 + 2 + 4 + 4 + n.len() + k.len() + value.len();
    if total > MAX_PAYLOAD { return Err(DbError::Invalid("record exceeds 4 MiB".into())); }
    let mut out = Vec::with_capacity(total);
    out.push(op);
    out.extend_from_slice(&(n.len() as u16).to_le_bytes());
    out.extend_from_slice(&(k.len() as u32).to_le_bytes());
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(n);
    out.extend_from_slice(k);
    out.extend_from_slice(value);
    Ok(out)
}

fn replay(bytes: &[u8]) -> Result<Map> {
    let mut cursor = 0;
    let mut map = Map::new();
    while cursor < bytes.len() {
        if bytes.len() - cursor < 12 { return Err(DbError::Corrupt("truncated WAL header/trailer".into())); }
        if &bytes[cursor..cursor + 4] != MAGIC { return Err(DbError::Corrupt("invalid WAL magic".into())); }
        let n = u32::from_le_bytes(bytes[cursor + 4..cursor + 8].try_into().unwrap()) as usize;
        cursor += 8;
        if n > MAX_PAYLOAD || bytes.len() - cursor < n + 4 {
            return Err(DbError::Corrupt("truncated or oversized WAL entry".into()));
        }
        let data = &bytes[cursor..cursor + n];
        cursor += n;
        let checksum = u32::from_le_bytes(bytes[cursor..cursor + 4].try_into().unwrap());
        cursor += 4;
        if checksum != crc32(data) { return Err(DbError::Corrupt("WAL checksum mismatch".into())); }
        if data.len() < 11 { return Err(DbError::Corrupt("record too short".into())); }
        let op = data[0];
        let nl = u16::from_le_bytes(data[1..3].try_into().unwrap()) as usize;
        let kl = u32::from_le_bytes(data[3..7].try_into().unwrap()) as usize;
        let vl = u32::from_le_bytes(data[7..11].try_into().unwrap()) as usize;
        if 11usize.checked_add(nl).and_then(|v| v.checked_add(kl))
            .and_then(|v| v.checked_add(vl)) != Some(data.len()) {
            return Err(DbError::Corrupt("record length mismatch".into()));
        }
        let ns = std::str::from_utf8(&data[11..11 + nl]).map_err(|_| DbError::Corrupt("namespace UTF-8".into()))?;
        let k = std::str::from_utf8(&data[11 + nl..11 + nl + kl]).map_err(|_| DbError::Corrupt("key UTF-8".into()))?;
        if ns.is_empty() || k.is_empty() { return Err(DbError::Corrupt("empty key/namespace".into())); }
        let val = &data[11 + nl + kl..];
        match op {
            1 => { map.insert((ns.into(), k.into()), val.to_vec()); },
            2 if vl == 0 => { map.remove(&(ns.into(), k.into())); },
            _ => return Err(DbError::Corrupt("unknown operation".into())),
        }
    }
    Ok(map)
}

fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB88320 & mask);
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crc32_reference_vector() { assert_eq!(crc32(b"123456789"), 0xcbf43926); }
    #[test]
    fn reject_bad_length() {
        let mut record = Vec::new();
        record.extend_from_slice(MAGIC);
        record.extend_from_slice(&(1u32).to_le_bytes());
        record.push(0);
        record.extend_from_slice(&(0u32).to_le_bytes());
        assert!(replay(&record).is_err());
    }
}
