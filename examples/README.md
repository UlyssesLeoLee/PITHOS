# Plugin manifests

Format: `key=value` (one per line); comments begin with `#`.
Required: `name`, `model`, `protocol`, `executable`, `dataset`.
Supported model workers: `kv`, `document`, `relational` (Turso experimental).
The process executable MUST be available on PATH or given by absolute path.
Only install administrator-approved, trusted local executables.
