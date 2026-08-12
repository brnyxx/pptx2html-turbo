#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommentKind {
    Legacy,
    Modern,
}

impl CommentKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Legacy => "legacy",
            Self::Modern => "modern",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommentAuthor {
    pub id: String,
    pub name: String,
    pub initials: Option<String>,
    pub part_name: String,
    pub kind: CommentKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlideNote {
    pub slide_number: usize,
    pub part_name: String,
    pub relationship_id: String,
    pub text: String,
    pub notes_master_part: Option<String>,
    pub notes_master_relationship_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlideComment {
    pub kind: CommentKind,
    pub slide_number: usize,
    pub part_name: String,
    pub relationship_id: String,
    pub id: String,
    pub parent_id: Option<String>,
    pub author_id: String,
    pub author: Option<CommentAuthor>,
    pub created_at: Option<String>,
    pub text: String,
    pub raw_extension_xml: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnnotationIssueCode {
    AuthorDuplicate,
    AuthorUnresolved,
    ElementNamespaceInvalid,
    PartMalformed,
    PartMissing,
    RelationshipDuplicate,
    RelationshipUnsafe,
}

impl AnnotationIssueCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AuthorDuplicate => "COMMENT_AUTHOR_DUPLICATE",
            Self::AuthorUnresolved => "COMMENT_AUTHOR_UNRESOLVED",
            Self::ElementNamespaceInvalid => "ANNOTATION_ELEMENT_NAMESPACE_INVALID",
            Self::PartMalformed => "ANNOTATION_PART_MALFORMED",
            Self::PartMissing => "ANNOTATION_PART_MISSING",
            Self::RelationshipDuplicate => "ANNOTATION_RELATIONSHIP_DUPLICATE",
            Self::RelationshipUnsafe => "ANNOTATION_RELATIONSHIP_UNSAFE",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotationIssue {
    pub code: AnnotationIssueCode,
    pub slide_number: Option<usize>,
    pub part_name: Option<String>,
    pub relationship_id: Option<String>,
    pub relationship_type: Option<String>,
    pub qualified_element_name: Option<String>,
    pub author_id: Option<String>,
    pub text: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NotesCommentsInventory {
    pub authors: Vec<CommentAuthor>,
    pub notes: Vec<SlideNote>,
    pub comments: Vec<SlideComment>,
    pub issues: Vec<AnnotationIssue>,
}
