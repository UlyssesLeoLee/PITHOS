//! Minimal V1 plugin manifest parser; manifests are trusted local administrator inputs.
use crate::{DataModel, DbError, ProcessEngine, Result};
use crate::model::PROTOCOL_VERSION;
use std::collections::HashMap;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct PluginManifest {
    pub name: String,
    pub model: DataModel,
    pub executable: String,
    pub dataset: String,
    pub protocol: u16,
}

impl PluginManifest {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let data = std::fs::read_to_string(path)?;
        let mut fields = HashMap::new();
        for (index, line) in data.lines().enumerate() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') { continue; }
            let (key, value) = line.split_once('=').ok_or_else(||
                DbError::Invalid(format!("manifest line {}: expected key=value", index + 1)))?;
            let key = key.trim();
            let value = value.trim();
            if !["name", "model", "executable", "dataset", "protocol"].contains(&key) {
                return Err(DbError::Invalid(format!("unknown manifest key: {key}")));
            }
            if value.is_empty() || fields.insert(key.to_string(), value.to_string()).is_some() {
                return Err(DbError::Invalid(format!("empty or duplicate manifest key: {key}")));
            }
        }
        let get = |k: &str| -> Result<String> {
            fields.get(k).cloned().ok_or_else(|| DbError::Invalid(format!("missing manifest key: {k}")))
        };
        let model = DataModel::parse(&get("model")?).ok_or_else(|| DbError::Invalid("unknown model".into()))?;
        let protocol: u16 = get("protocol")?.parse().map_err(|_| DbError::Invalid("bad manifest protocol".into()))?;
        if protocol != PROTOCOL_VERSION { return Err(DbError::Incompatible("manifest protocol mismatch".into())); }
        Ok(Self { name: get("name")?, model, executable: get("executable")?,
            dataset: get("dataset")?, protocol })
    }

    pub fn spawn(&self) -> Result<ProcessEngine> {
        let canonical = std::fs::canonicalize(&self.dataset).unwrap_or_else(|_| self.dataset.clone().into());
        let identity = canonical.to_string_lossy().into_owned();
        match self.model {
            DataModel::KeyValue | DataModel::Document => {
                ProcessEngine::spawn(&self.executable, &[self.model.token(), &self.dataset],
                    &self.name, self.model, &identity)
            }
            DataModel::Relational => ProcessEngine::spawn(&self.executable, &[&self.dataset],
                &self.name, self.model, &identity),
            _ => Err(DbError::Unsupported("graph/vector worker not implemented in V1".into())),
        }
    }
}
