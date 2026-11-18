use crate::{Command, DbError, Engine, Response, Result};
use crate::model::PROTOCOL_VERSION;
use std::collections::HashMap;
use std::sync::{Arc, Condvar, Mutex, RwLock};
use std::time::{Duration, Instant};

struct SlotState {
    engine: Arc<dyn Engine>,
    in_flight: usize,
    transitioning: bool,
    generation: u64,
}
struct Slot { state: Mutex<SlotState>, idle: Condvar }

/// Admission barrier: new operations are rejected while a plugin is quiescing.
/// A completed operation decrements in_flight even if it returned an error.
struct Lease { slot: Arc<Slot> }
impl Drop for Lease {
    fn drop(&mut self) {
        if let Ok(mut state) = self.slot.state.lock() {
            state.in_flight = state.in_flight.saturating_sub(1);
            if state.in_flight == 0 { self.slot.idle.notify_all(); }
        }
    }
}

#[derive(Default)]
pub struct PluginManager { slots: RwLock<HashMap<String, Arc<Slot>>> }
impl PluginManager {
    pub fn new() -> Self { Self::default() }

    pub fn register(&self, route: &str, engine: Arc<dyn Engine>) -> Result<()> {
        if route.is_empty() { return Err(DbError::Invalid("empty route".into())); }
        engine.health()?;
        let descriptor = engine.descriptor();
        if descriptor.protocol != PROTOCOL_VERSION {
            return Err(DbError::Incompatible("plugin protocol version".into()));
        }
        let mut slots = self.slots.write().map_err(|_| DbError::Plugin("registry lock poisoned".into()))?;
        if slots.contains_key(route) { return Err(DbError::Invalid("duplicate route".into())); }
        slots.insert(route.into(), Arc::new(Slot { state: Mutex::new(SlotState {
            engine, in_flight: 0, transitioning: false, generation: 1,
        }), idle: Condvar::new() }));
        Ok(())
    }

    fn slot(&self, route: &str) -> Result<Arc<Slot>> {
        self.slots.read().map_err(|_| DbError::Plugin("registry lock poisoned".into()))?
            .get(route).cloned().ok_or_else(|| DbError::Missing(route.into()))
    }

    pub fn execute(&self, route: &str, command: Command) -> Result<Response> {
        let slot = self.slot(route)?;
        let engine = {
            let mut state = slot.state.lock().map_err(|_| DbError::Plugin("slot lock poisoned".into()))?;
            if state.transitioning { return Err(DbError::Busy("plugin transition in progress".into())); }
            state.in_flight += 1;
            Arc::clone(&state.engine)
        };
        let _lease = Lease { slot };
        engine.execute(command)
    }

    pub fn generation(&self, route: &str) -> Result<u64> {
        Ok(self.slot(route)?.state.lock().map_err(|_| DbError::Plugin("slot lock poisoned".into()))?.generation)
    }

    /// Safe routing switchover for the SAME dataset identity (no data migration).
    /// The loader is called only after all admitted requests complete.
    /// If loading/health/compatibility fails, the old engine stays active.
    pub fn hot_swap_with<F>(&self, route: &str, timeout: Duration, loader: F) -> Result<u64>
    where F: FnOnce() -> Result<Arc<dyn Engine>> {
        let slot = self.slot(route)?;
        let deadline = Instant::now() + timeout;
        let old_description;
        {
            let mut state = slot.state.lock().map_err(|_| DbError::Plugin("slot lock poisoned".into()))?;
            if state.transitioning { return Err(DbError::Busy("already transitioning".into())); }
            state.transitioning = true;
            while state.in_flight > 0 {
                let remaining = deadline.saturating_duration_since(Instant::now());
                if remaining.is_zero() {
                    state.transitioning = false;
                    slot.idle.notify_all();
                    return Err(DbError::Busy("quiesce timeout; old plugin retained".into()));
                }
                let (next, wait_result) = slot.idle.wait_timeout(state, remaining)
                    .map_err(|_| DbError::Plugin("condition variable poisoned".into()))?;
                state = next;
                if wait_result.timed_out() && state.in_flight > 0 {
                    state.transitioning = false;
                    slot.idle.notify_all();
                    return Err(DbError::Busy("quiesce timeout; old plugin retained".into()));
                }
            }
            old_description = state.engine.descriptor();
        }
        // Keep admission closed while loading and validating the candidate.
        let candidate = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let new_engine = loader()?;
            let new_description = new_engine.descriptor();
            if new_description.protocol != PROTOCOL_VERSION
                || new_description.model != old_description.model
                || new_description.storage_id != old_description.storage_id {
                return Err(DbError::Incompatible(
                    "model/protocol/storage identity mismatch; explicit migration needed".into()));
            }
            new_engine.health()?;
            Ok(new_engine)
        }))
        .map_err(|_| DbError::Plugin("candidate plugin panicked; old plugin retained".into()))
        .and_then(|result| result);
        let mut state = slot.state.lock().map_err(|_| DbError::Plugin("slot lock poisoned".into()))?;
        let result = match candidate {
            Ok(engine) => {
                state.engine = engine;
                state.generation += 1;
                Ok(state.generation)
            }
            Err(e) => Err(e),
        };
        state.transitioning = false;
        slot.idle.notify_all();
        result
    }
}
