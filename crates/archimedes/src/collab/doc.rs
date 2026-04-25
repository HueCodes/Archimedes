use std::collections::HashMap;
use std::sync::Arc;

use uuid::Uuid;
use yrs::updates::decoder::Decode;
use yrs::updates::encoder::Encode;
use yrs::{Any, Doc, Map, MapRef, ReadTxn, StateVector, Transact, Update, Out};

/// Stable identifier for a point. Generated client-side, never reused.
pub type PointId = String;

#[derive(Clone, Debug, PartialEq)]
pub struct CollabPoint {
    pub id: PointId,
    pub x: f64,
    pub y: f64,
}

#[derive(Debug)]
pub enum CollabError {
    Decode(String),
    Apply(String),
}

impl std::fmt::Display for CollabError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CollabError::Decode(s) => write!(f, "decode: {s}"),
            CollabError::Apply(s) => write!(f, "apply: {s}"),
        }
    }
}

impl std::error::Error for CollabError {}

/// Shared point set backed by a yrs CRDT document.
///
/// Points are stored in a top-level `MapRef` keyed by stable UUIDs, with
/// each value an `Any::Map { x, y }`. Last-writer-wins per point — fine
/// for "two users dragging the same point": one wins, neither flickers.
pub struct CollabDoc {
    doc: Doc,
    points: MapRef,
}

impl Default for CollabDoc {
    fn default() -> Self {
        Self::new()
    }
}

impl CollabDoc {
    pub fn new() -> Self {
        let doc = Doc::new();
        let points = doc.get_or_insert_map("points");
        Self { doc, points }
    }

    /// Add a new point, return its generated id.
    pub fn insert_point(&self, x: f64, y: f64) -> PointId {
        let id = Uuid::new_v4().to_string();
        self.put(&id, x, y);
        id
    }

    /// Move an existing point. No-op if the id is unknown.
    pub fn move_point(&self, id: &PointId, x: f64, y: f64) {
        let exists = {
            let txn = self.doc.transact();
            self.points.contains_key(&txn, id.as_str())
        };
        if exists {
            self.put(id, x, y);
        }
    }

    pub fn delete_point(&self, id: &PointId) {
        let mut txn = self.doc.transact_mut();
        self.points.remove(&mut txn, id.as_str());
    }

    pub fn clear(&self) {
        let keys: Vec<String> = {
            let txn = self.doc.transact();
            self.points.keys(&txn).map(|k| k.to_string()).collect()
        };
        let mut txn = self.doc.transact_mut();
        for k in keys {
            self.points.remove(&mut txn, k.as_str());
        }
    }

    /// Snapshot of the current point set, sorted by id for stable order.
    /// The convex hull algorithm sorts internally, so caller-side order
    /// doesn't affect correctness.
    pub fn points(&self) -> Vec<CollabPoint> {
        let txn = self.doc.transact();
        let mut out: Vec<CollabPoint> = self
            .points
            .iter(&txn)
            .filter_map(|(key, value)| {
                let (x, y) = extract_xy(&value)?;
                Some(CollabPoint {
                    id: key.to_string(),
                    x,
                    y,
                })
            })
            .collect();
        out.sort_by(|a, b| a.id.cmp(&b.id));
        out
    }

    pub fn len(&self) -> usize {
        let txn = self.doc.transact();
        self.points.len(&txn) as usize
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Encode the whole document state — sent to a peer that just joined.
    pub fn encode_state(&self) -> Vec<u8> {
        let txn = self.doc.transact();
        txn.encode_state_as_update_v1(&StateVector::default())
    }

    /// Encode the delta needed to bring a peer with `remote_state` up to date.
    pub fn encode_diff(&self, remote_state: &[u8]) -> Result<Vec<u8>, CollabError> {
        let sv = StateVector::decode_v1(remote_state)
            .map_err(|e| CollabError::Decode(e.to_string()))?;
        let txn = self.doc.transact();
        Ok(txn.encode_state_as_update_v1(&sv))
    }

    pub fn state_vector(&self) -> Vec<u8> {
        let txn = self.doc.transact();
        txn.state_vector().encode_v1()
    }

    /// Apply update bytes received from a remote peer.
    pub fn apply_remote_update(&self, bytes: &[u8]) -> Result<(), CollabError> {
        let update =
            Update::decode_v1(bytes).map_err(|e| CollabError::Decode(e.to_string()))?;
        let mut txn = self.doc.transact_mut();
        txn.apply_update(update)
            .map_err(|e| CollabError::Apply(e.to_string()))
    }

    fn put(&self, id: &str, x: f64, y: f64) {
        let mut txn = self.doc.transact_mut();
        let mut inner: HashMap<String, Any> = HashMap::with_capacity(2);
        inner.insert("x".to_string(), Any::Number(x));
        inner.insert("y".to_string(), Any::Number(y));
        self.points
            .insert(&mut txn, id.to_string(), Any::Map(Arc::new(inner)));
    }
}

fn extract_xy(value: &Out) -> Option<(f64, f64)> {
    let any = match value {
        Out::Any(a) => a,
        _ => return None,
    };
    let map = match any {
        Any::Map(m) => m,
        _ => return None,
    };
    let x = as_number(map.get("x")?)?;
    let y = as_number(map.get("y")?)?;
    Some((x, y))
}

fn as_number(any: &Any) -> Option<f64> {
    match any {
        Any::Number(n) => Some(*n),
        Any::BigInt(n) => Some(*n as f64),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_then_read_round_trips() {
        let doc = CollabDoc::new();
        let id = doc.insert_point(1.5, 2.5);
        let pts = doc.points();
        assert_eq!(pts.len(), 1);
        assert_eq!(pts[0].id, id);
        assert!((pts[0].x - 1.5).abs() < 1e-9);
        assert!((pts[0].y - 2.5).abs() < 1e-9);
    }

    #[test]
    fn move_then_delete() {
        let doc = CollabDoc::new();
        let id = doc.insert_point(0.0, 0.0);
        doc.move_point(&id, 100.0, 200.0);
        assert_eq!(doc.points()[0].x, 100.0);
        assert_eq!(doc.points()[0].y, 200.0);
        doc.delete_point(&id);
        assert!(doc.is_empty());
    }

    #[test]
    fn move_unknown_id_is_noop() {
        let doc = CollabDoc::new();
        doc.move_point(&"ghost".to_string(), 1.0, 2.0);
        assert!(doc.is_empty());
    }

    #[test]
    fn clear_removes_all_points() {
        let doc = CollabDoc::new();
        for i in 0..5 {
            doc.insert_point(i as f64, 0.0);
        }
        assert_eq!(doc.len(), 5);
        doc.clear();
        assert!(doc.is_empty());
    }

    #[test]
    fn full_state_replicates_to_fresh_doc() {
        let a = CollabDoc::new();
        a.insert_point(1.0, 1.0);
        a.insert_point(2.0, 2.0);
        let snapshot = a.encode_state();

        let b = CollabDoc::new();
        b.apply_remote_update(&snapshot).expect("apply failed");

        let mut ap = a.points();
        let mut bp = b.points();
        ap.sort_by(|p, q| p.id.cmp(&q.id));
        bp.sort_by(|p, q| p.id.cmp(&q.id));
        assert_eq!(ap, bp);
    }

    #[test]
    fn diff_brings_stale_peer_current() {
        let a = CollabDoc::new();
        let b = CollabDoc::new();
        a.insert_point(1.0, 1.0);
        b.apply_remote_update(&a.encode_state()).unwrap();

        // Diverge: A adds a point B doesn't have.
        a.insert_point(9.0, 9.0);

        // B asks for what it's missing using its state vector.
        let b_sv = b.state_vector();
        let diff = a.encode_diff(&b_sv).unwrap();
        b.apply_remote_update(&diff).unwrap();

        assert_eq!(a.points(), b.points());
    }

    #[test]
    fn concurrent_moves_converge_lww() {
        let a = CollabDoc::new();
        let b = CollabDoc::new();
        let id = a.insert_point(0.0, 0.0);
        b.apply_remote_update(&a.encode_state()).unwrap();

        // Both move the same point concurrently.
        a.move_point(&id, 10.0, 10.0);
        b.move_point(&id, 20.0, 20.0);

        // Cross-apply state.
        let a_state = a.encode_state();
        let b_state = b.encode_state();
        a.apply_remote_update(&b_state).unwrap();
        b.apply_remote_update(&a_state).unwrap();

        // Both must agree on the same surviving value (LWW per yrs's clock),
        // even if we can't predict which one won.
        assert_eq!(a.points(), b.points());
    }
}
