//! The TOML address of one manifest field, so a refusal can say where it
//! is as data and a reader such as the desktop editor need not parse the
//! prose of the message to find the field.

specmark::scope!("spec://org.vibevm.core/vibevm/VIBEVM-SPEC#manifest-schema");

use std::fmt;

use toml_edit::{Item, Table, TableLike, Value};

/// A path into `vibe.toml`: table and field names as written in the
/// manifest, array elements by zero-based index, the leaf field last.
///
/// Every builder method borrows and returns a new path, so a validator
/// can hold the address of its table and derive one per field.
///
/// ```
/// use vibe_core::manifest::FieldPath;
///
/// let step = FieldPath::root()
///     .table("installer_options")
///     .index(1)
///     .table("steps")
///     .index(0);
/// assert_eq!(step.field("sha256").to_string(), "installer_options[1].steps[0].sha256");
/// assert_eq!(FieldPath::root().table("application").field("id").to_string(), "application.id");
///
/// let source = "[application]\nid = \"Demo\"\n";
/// let at = FieldPath::root().table("application").field("id");
/// assert_eq!(at.locate(source), Some((2, 1)));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct FieldPath {
    segments: Vec<Segment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Segment {
    Key(String),
    Index(usize),
}

impl FieldPath {
    /// The document itself, before any table is named.
    pub fn root() -> Self {
        Self::default()
    }

    /// A table (or an array of tables) under this path.
    pub fn table(&self, name: &str) -> Self {
        self.key(name)
    }

    /// The element at a zero-based index of the array this path names.
    pub fn index(&self, index: usize) -> Self {
        let mut next = self.clone();
        next.segments.push(Segment::Index(index));
        next
    }

    /// The leaf field under this path. Spelled apart from [`Self::table`]
    /// only so the validators read as the manifest does.
    pub fn field(&self, name: &str) -> Self {
        self.key(name)
    }

    /// `true` for the document itself.
    pub fn is_root(&self) -> bool {
        self.segments.is_empty()
    }

    fn key(&self, name: &str) -> Self {
        let mut next = self.clone();
        next.segments.push(Segment::Key(name.to_owned()));
        next
    }

    /// The table a TOML deserialisation error names by its keys
    /// (`in \`package.media\``), when every key is a table in `source`.
    /// A key path that passes through an array is refused: the parser
    /// does not say which element it means, and the address is not guessed.
    pub(crate) fn from_toml_keys(keys: &str, source: &str) -> Option<Self> {
        let document = toml_edit::Document::parse(source).ok()?;
        let mut node = Node::Table(document.as_table());
        let mut path = Self::root();
        for key in keys.split('.') {
            let (_, item) = node.table_like()?.get_key_value(key)?;
            node = Node::Item(item);
            path = path.table(key);
        }
        node.table_like()?;
        Some(path)
    }

    /// The 1-based line and column of this field in `source`: the key of
    /// the leaf when it has one, else the element or table it names.
    /// `None` when the source does not parse or does not spell the path
    /// (a field that is absent, or one the manifest derives from sugar).
    pub fn locate(&self, source: &str) -> Option<(usize, usize)> {
        let document = toml_edit::Document::parse(source).ok()?;
        let mut node = Node::Table(document.as_table());
        let mut span = None;
        for segment in &self.segments {
            match segment {
                Segment::Key(name) => {
                    let (key, item) = node.table_like()?.get_key_value(name)?;
                    span = key.span().or_else(|| item.span());
                    node = Node::Item(item);
                }
                Segment::Index(index) => {
                    node = node.element(*index)?;
                    span = node.span();
                }
            }
        }
        let span = span?;
        Some(crate::error::line_column(source, span.start))
    }
}

/// One step of the walk [`FieldPath::locate`] makes through a parsed
/// document, which keeps the spans the deserialised manifest drops.
#[derive(Clone, Copy)]
enum Node<'a> {
    Table(&'a Table),
    Item(&'a Item),
    Value(&'a Value),
}

impl<'a> Node<'a> {
    fn table_like(self) -> Option<&'a dyn TableLike> {
        match self {
            Node::Table(table) => Some(table),
            Node::Item(item) => item.as_table_like(),
            Node::Value(value) => value.as_inline_table().map(|t| t as &dyn TableLike),
        }
    }

    fn element(self, index: usize) -> Option<Node<'a>> {
        let array = match self {
            Node::Item(Item::ArrayOfTables(tables)) => return tables.get(index).map(Node::Table),
            Node::Item(item) => item.as_array()?,
            Node::Value(value) => value.as_array()?,
            Node::Table(_) => return None,
        };
        array.get(index).map(Node::Value)
    }

    fn span(self) -> Option<std::ops::Range<usize>> {
        match self {
            Node::Table(table) => table.span(),
            Node::Item(item) => item.span(),
            Node::Value(value) => value.span(),
        }
    }
}

impl fmt::Display for FieldPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (position, segment) in self.segments.iter().enumerate() {
            match segment {
                Segment::Index(index) => write!(f, "[{index}]")?,
                Segment::Key(name) => {
                    if position > 0 {
                        f.write_str(".")?;
                    }
                    if is_bare_key(name) {
                        f.write_str(name)?;
                    } else {
                        // A key TOML cannot write bare (`sr@latin`) is
                        // quoted, so the rendered path stays valid TOML.
                        write!(f, "{}", toml_edit::Key::new(name.as_str()).display_repr())?;
                    }
                }
            }
        }
        Ok(())
    }
}

fn is_bare_key(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-'))
}

/// A refusal of a manifest validator: the field it is about and the
/// sentence that explains it. `Display` is the sentence alone, so a
/// caller that only prints the refusal reads what it always read.
///
/// ```
/// use vibe_core::manifest::{FieldError, FieldPath};
///
/// let refusal = FieldError::new(
///     FieldPath::root().table("application").field("id"),
///     "[application].id must be a portable lowercase token",
/// );
/// assert_eq!(refusal.field.to_string(), "application.id");
/// assert_eq!(refusal.to_string(), "[application].id must be a portable lowercase token");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldError {
    pub field: FieldPath,
    pub message: String,
}

impl FieldError {
    pub fn new(field: FieldPath, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }

    /// The same field with a different sentence, for a caller that adds
    /// the context the validator did not have (the option's id).
    pub fn reworded(self, message: impl FnOnce(&str) -> String) -> Self {
        Self {
            message: message(&self.message),
            field: self.field,
        }
    }
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for FieldError {}

#[cfg(test)]
mod tests {
    use super::FieldPath;

    #[test]
    fn renders_tables_indexes_and_the_leaf() {
        let root = FieldPath::root();
        assert_eq!(root.to_string(), "");
        assert!(root.is_root());
        let option = root.table("installer_options").index(0);
        assert_eq!(option.to_string(), "installer_options[0]");
        assert_eq!(
            option.table("launchers").index(1).field("id").to_string(),
            "installer_options[0].launchers[1].id"
        );
        assert_eq!(
            option.table("requires").index(2).to_string(),
            "installer_options[0].requires[2]"
        );
    }

    #[test]
    fn quotes_a_key_toml_cannot_write_bare() {
        let names = FieldPath::root().table("launchers").index(0).table("names");
        assert_eq!(names.field("de").to_string(), "launchers[0].names.de");
        assert_eq!(
            names.field("sr@latin").to_string(),
            "launchers[0].names.\"sr@latin\""
        );
    }

    #[test]
    fn locates_a_field_in_an_array_of_tables() {
        let source = "[package]\nname = \"x\"\n\n[[installer_options]]\nid = \"a\"\n\n\
                      [[installer_options]]\nid = \"b\"\n\n[[installer_options.steps]]\n\
                      id = \"fetch\"\nsha256 = \"nope\"\n";
        let at = FieldPath::root()
            .table("installer_options")
            .index(1)
            .table("steps")
            .index(0)
            .field("sha256");
        assert_eq!(at.locate(source), Some((12, 1)));
        let inline = "args = [\"a\", \"b\"]\n";
        assert_eq!(
            FieldPath::root().table("args").index(1).locate(inline),
            Some((1, 14))
        );
        assert_eq!(
            FieldPath::root().table("missing").locate(source),
            None,
            "an absent field has no position"
        );
    }
}
