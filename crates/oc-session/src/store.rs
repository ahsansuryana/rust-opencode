//! CRUD dasar untuk session data (file-based store, padanan SQLite di TS).
//! Sprint 3 menemukan bahwa storage asli adalah file-based, bukan SQLite.

use crate::model::{SessionRow, UserOrAssistant, WithParts};
use oc_storage::StorageService;

/// SessionStore — CRUD level "data access" tanpa business logic.
pub struct SessionStore {
    storage: StorageService,
}

impl SessionStore {
    pub fn new() -> Result<Self, oc_storage::Error> {
        Ok(SessionStore {
            storage: StorageService::new()?,
        })
    }

    /// Create atau update session.
    pub fn upsert_session(&self, session: &SessionRow) -> Result<(), oc_storage::Error> {
        self.storage.write(
            &["session".into(), "info".into(), session.id.clone()],
            &session,
        )
    }

    pub fn get_session(&self, id: &str) -> Result<Option<SessionRow>, oc_storage::Error> {
        match self
            .storage
            .read::<SessionRow>(&["session".into(), "info".into(), id.to_string()])
        {
            Ok(session) => Ok(Some(session)),
            Err(oc_storage::Error::NotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn remove_session(&self, id: &str) -> Result<(), oc_storage::Error> {
        self.storage
            .remove(&["session".into(), "info".into(), id.to_string()])
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionRow>, oc_storage::Error> {
        let keys = self.storage.list(&["session".into(), "info".into()])?;
        let mut sessions = Vec::new();
        for key in keys {
            let id = key.last().cloned().unwrap_or_default();
            if let Some(session) = self.get_session(&id)? {
                sessions.push(session);
            }
        }
        sessions.sort_by_key(|s| std::cmp::Reverse(s.time_created));
        Ok(sessions)
    }

    pub fn append_message(&self, message: &UserOrAssistant) -> Result<(), oc_storage::Error> {
        let (session_id, msg_id) = match message {
            UserOrAssistant::User(m) => (m.session_id.clone(), m.id.clone()),
            UserOrAssistant::Assistant(m) => (m.session_id.clone(), m.id.clone()),
        };
        self.storage
            .write(&["message".into(), session_id, msg_id], &message)
    }

    pub fn get_message(
        &self,
        session_id: &str,
        message_id: &str,
    ) -> Result<Option<UserOrAssistant>, oc_storage::Error> {
        match self.storage.read::<UserOrAssistant>(&[
            "message".into(),
            session_id.into(),
            message_id.into(),
        ]) {
            Ok(message) => Ok(Some(message)),
            Err(oc_storage::Error::NotFound(_)) => Ok(None),
            Err(e) => Err(e),
        }
    }

    pub fn list_messages(&self, session_id: &str) -> Result<Vec<WithParts>, oc_storage::Error> {
        let keys = self.storage.list(&["message".into(), session_id.into()])?;
        let mut messages = Vec::new();
        for key in keys {
            let msg_id = key.last().cloned().unwrap_or_default();
            if let Some(info) = self.get_message(session_id, &msg_id)? {
                let parts = self.list_parts(session_id, &msg_id)?;
                messages.push(WithParts { info, parts });
            }
        }
        // sort by created time ascending
        messages.sort_by_key(|m| match &m.info {
            UserOrAssistant::User(u) => u.time.created,
            UserOrAssistant::Assistant(a) => a.time.created,
        });
        Ok(messages)
    }

    /// Write satu part.
    pub fn write_part(
        &self,
        session_id: &str,
        message_id: &str,
        part: &crate::model::Part,
    ) -> Result<(), oc_storage::Error> {
        let base = part.base_ids();
        self.storage.write(
            &[
                "part".into(),
                session_id.into(),
                message_id.into(),
                base.id.clone(),
            ],
            &part,
        )
    }

    pub fn list_parts(
        &self,
        session_id: &str,
        message_id: &str,
    ) -> Result<Vec<crate::model::Part>, oc_storage::Error> {
        let keys = self
            .storage
            .list(&["part".into(), session_id.into(), message_id.into()])?;
        let mut parts = Vec::new();
        for key in keys {
            let part_id = key.last().cloned().unwrap_or_default();
            let path_key = [
                "part".into(),
                session_id.to_string(),
                message_id.to_string(),
                part_id,
            ];
            match self.storage.read::<crate::model::Part>(&path_key) {
                Ok(part) => parts.push(part),
                Err(oc_storage::Error::NotFound(_)) => {}
                Err(e) => return Err(e),
            }
        }
        Ok(parts)
    }
}
