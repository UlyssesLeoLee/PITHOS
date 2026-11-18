use crate::error::Result;

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataModel { KeyValue, Document, Relational, Graph, Vector }

impl DataModel {
    pub fn token(self) -> &'static str {
        match self {
            Self::KeyValue => "kv", Self::Document => "document",
            Self::Relational => "relational", Self::Graph => "graph", Self::Vector => "vector",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "kv" => Some(Self::KeyValue), "document" => Some(Self::Document),
            "relational" => Some(Self::Relational), "graph" => Some(Self::Graph),
            "vector" => Some(Self::Vector), _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Descriptor {
    pub name: String,
    pub model: DataModel,
    pub protocol: u16,
    /// Stable dataset identity. Switching between different identities requires migration.
    pub storage_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Put { namespace: String, key: String, value: Vec<u8> },
    Get { namespace: String, key: String },
    Delete { namespace: String, key: String },
    Scan { namespace: String, prefix: String },
    SqlExec { sql: String },
    SqlQuery { sql: String },
    Ping,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Response {
    Ack,
    Value(Option<Vec<u8>>),
    Items(Vec<(String, Vec<u8>)>),
    Text(String),
}

pub trait Engine: Send + Sync {
    fn descriptor(&self) -> Descriptor;
    fn execute(&self, command: Command) -> Result<Response>;
    fn health(&self) -> Result<()> {
        match self.execute(Command::Ping)? {
            Response::Ack => Ok(()),
            _ => Err(crate::DbError::Plugin("health reply is not ACK".into())),
        }
    }
}
