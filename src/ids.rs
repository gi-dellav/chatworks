use std::fmt;
use std::marker::PhantomData;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BackendId(Arc<str>);

impl BackendId {
    pub fn new(value: impl Into<Arc<str>>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for BackendId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Display for BackendId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for BackendId {
    fn from(value: &str) -> Self {
        Self(Arc::from(value))
    }
}

impl From<String> for BackendId {
    fn from(value: String) -> Self {
        Self(Arc::from(value))
    }
}

impl From<Arc<str>> for BackendId {
    fn from(value: Arc<str>) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id<T> {
    backend: BackendId,
    raw: Arc<str>,
    kind: PhantomData<fn() -> T>,
}

impl<T> Id<T> {
    pub fn new(backend: BackendId, raw: impl Into<Arc<str>>) -> Self {
        Self {
            backend,
            raw: raw.into(),
            kind: PhantomData,
        }
    }

    pub fn backend(&self) -> &BackendId {
        &self.backend
    }

    pub fn as_str(&self) -> &str {
        &self.raw
    }

    pub fn into_raw(self) -> Arc<str> {
        self.raw
    }
}

impl<T> AsRef<str> for Id<T> {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl<T> fmt::Display for Id<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UserMarker;

#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ConversationMarker;

#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ThreadMarker;

#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageMarker;

#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttachmentMarker;

#[doc(hidden)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CursorMarker;

pub type UserId = Id<UserMarker>;
pub type ConversationId = Id<ConversationMarker>;
pub type ThreadId = Id<ThreadMarker>;
pub type MessageId = Id<MessageMarker>;
pub type AttachmentId = Id<AttachmentMarker>;
pub type Cursor = Id<CursorMarker>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_scoped_to_their_backend() {
        let first = UserId::new(BackendId::new("first"), "42");
        let second = UserId::new(BackendId::new("second"), "42");

        assert_eq!(first.as_str(), second.as_str());
        assert_ne!(first, second);
        assert_eq!(first.backend().as_str(), "first");
    }
}
