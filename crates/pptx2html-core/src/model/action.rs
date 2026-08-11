#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionTrigger {
    Click,
    Hover,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionTarget {
    ExternalUri(String),
    InternalSlide(usize),
    Next,
    Previous,
    First,
    Last,
    NoOp,
    MediaPlay,
    Unsupported(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ActionIssue {
    Unsupported,
    UnsafeUri,
    MissingRelationship,
    DuplicateRelationship,
    RelationshipMismatch,
    UnsafeInternalTarget,
    UnresolvedSlide,
}

impl ActionIssue {
    pub const fn code(self) -> &'static str {
        match self {
            Self::Unsupported => "ACTION_UNSUPPORTED",
            Self::UnsafeUri => "ACTION_UNSAFE_URI",
            Self::MissingRelationship => "ACTION_RELATIONSHIP_MISSING",
            Self::DuplicateRelationship => "ACTION_RELATIONSHIP_DUPLICATE",
            Self::RelationshipMismatch => "ACTION_RELATIONSHIP_MISMATCH",
            Self::UnsafeInternalTarget => "ACTION_INTERNAL_TARGET_UNSAFE",
            Self::UnresolvedSlide => "ACTION_INTERNAL_TARGET_UNRESOLVED",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Action {
    pub trigger: ActionTrigger,
    pub target: ActionTarget,
    pub relationship_id: Option<String>,
    pub relationship_type: Option<String>,
    pub relationship_mode: Option<String>,
    pub source_part: Option<String>,
    pub raw_action: Option<String>,
    pub anchor: Option<String>,
    pub tooltip: Option<String>,
    pub issue: Option<ActionIssue>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActionSet {
    pub click: Option<Action>,
    pub hover: Option<Action>,
}

impl ActionSet {
    pub fn assign(&mut self, action: Action) {
        match action.trigger {
            ActionTrigger::Click => self.click = Some(action),
            ActionTrigger::Hover => self.hover = Some(action),
        }
    }
}

/// Product security policy for executable external links.
pub fn is_safe_external_uri(uri: &str) -> bool {
    if uri.is_empty()
        || uri
            .chars()
            .any(|c| c.is_ascii_control() || c.is_ascii_whitespace())
    {
        return false;
    }
    let Some((scheme, remainder)) = uri.split_once(':') else {
        return false;
    };
    if !scheme.bytes().all(|c| c.is_ascii_alphabetic()) {
        return false;
    }
    match scheme.to_ascii_lowercase().as_str() {
        "http" | "https" => {
            let Some(authority_and_path) = remainder.strip_prefix("//") else {
                return false;
            };
            let authority = authority_and_path
                .split(['/', '?', '#'])
                .next()
                .unwrap_or("");
            !authority.is_empty() && !authority.contains('@')
        }
        "mailto" => !remainder.is_empty() && !remainder.starts_with('/'),
        _ => false,
    }
}
