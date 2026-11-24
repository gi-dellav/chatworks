use std::sync::Arc;

use crate::ids::UserId;
use crate::model::UserSummary;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct RawPayload {
    pub format: String,
    pub data: Arc<[u8]>,
}

impl RawPayload {
    pub fn new(format: impl Into<String>, data: impl Into<Arc<[u8]>>) -> Self {
        Self {
            format: format.into(),
            data: data.into(),
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Content {
    pub plain: String,
    pub blocks: Vec<Block>,
    pub raw: Option<RawPayload>,
}

impl Content {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            plain: text.into(),
            blocks: Vec::new(),
            raw: None,
        }
    }

    pub fn rich(plain: impl Into<String>, blocks: Vec<Block>) -> Self {
        Self {
            plain: plain.into(),
            blocks,
            raw: None,
        }
    }

    pub fn with_raw(mut self, raw: RawPayload) -> Self {
        self.raw = Some(raw);
        self
    }

    pub fn is_plain(&self) -> bool {
        self.blocks.is_empty()
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Block {
    Text(String),
    Mention {
        user: UserId,
        label: String,
    },
    Link {
        url: String,
        label: Option<String>,
    },
    Code {
        language: Option<String>,
        text: String,
    },
    Quote {
        author: Option<UserSummary>,
        blocks: Vec<Block>,
    },
    Unknown {
        kind: String,
        raw: RawPayload,
    },
}
