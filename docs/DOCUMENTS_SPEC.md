# MEVDAN — Documents Specification

`mevdan-documents` handles reading and understanding documents.

## Formats

| Format | Detection | Extraction |
|--------|-----------|------------|
| Markdown | ✅ | ✅ (with sections) |
| Text | ✅ | ✅ |
| CSV | ✅ | ✅ (row count) |
| JSON | ✅ | ✅ (top-level items) |
| PDF | ✅ | ❌ (V5.5+) |
| DOCX | ✅ | ❌ (V5.5+) |
| EPUB | ✅ | ❌ (V5.5+) |

PDF/DOCX/EPUB are recognized but not extracted in V5.3. A real PDF
parser (500+ lines + deps) arrives later as a dedicated phase.

## Concepts

### DocumentFormat

```rust
pub enum DocumentFormat {
    Pdf, Docx, Markdown, Text, Epub, Csv, Json, Unknown,
}
Helpers:

is_extractable() — Markdown, Text, CSV, JSON.

is_plain_text() — Markdown, Text.

extension() — main extension (md, txt, ...).

from_path(path) — detect by extension.

Document
Descriptor with path, format, size, title and metadata.

rust
pub struct Document {
    pub path: PathBuf,
    pub format: DocumentFormat,
    pub size_bytes: u64,
    pub title: Option<String>,
    pub metadata: serde_json::Value,
}
Section
For structured documents (Markdown), each section has:

rust
pub struct Section {
    pub level: u8,          // 1 = H1, 2 = H2, ..., 0 = no heading
    pub title: String,
    pub content: String,
    pub start_line: usize,
}
DocumentContent
Extracted content:

rust
pub struct DocumentContent {
    pub text: String,
    pub line_count: usize,
    pub word_count: usize,
    pub sections: Vec<Section>,
    pub extracted: bool,
}
Helpers:

preview(max_chars) — truncated text, UTF-8 safe.

is_empty().

LoadedDocument
Combines Document + DocumentContent:

rust
let loaded = DocumentLoader::new().load("/path/to/README.md")?;
println!("{}", loaded.summary());
DocumentLoader
rust
let loader = DocumentLoader::new();
let loaded = loader.load("README.md")?;

assert_eq!(loaded.document.format, DocumentFormat::Markdown);
assert!(loaded.content.extracted);
assert_eq!(loaded.document.title.as_deref(), Some("Intro"));
Title inference
If the document has a first heading, use it.

Otherwise, use the file name.

Markdown section parsing
Heading parser: # followed by space, up to ######.

# Title → level 1

## Section → level 2

### Deep → level 3

####### x → ignored (level 7)

Trailing # are stripped: ## Title ## → "Title".

What this is NOT
No full markdown parsing. Only headings.

No HTML rendering. Text only.

No code block extraction. Content is opaque.

No tables. CSV is treated as rows, not columns.

No PDF extraction. Arrives later.

Design notes
UTF-8 safe. Slicing uses char_boundary.

Deterministic. Same input, same output.

Fast. No external parsers for text formats.

Extensible. Adding a format = add an extractor.
