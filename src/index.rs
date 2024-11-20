//! Full text index for OBO documents using tantivy.

use tantivy::schema::Schema;

/// Encapsulation of a Tantivy index.
///
/// The index lives in a directory and will only persist as long as the lifetime of this struct.
pub struct Index {
    /// The index directory.
    #[allow(dead_code)]
    tmpdir: tempdir::TempDir, // keep around for RAII
    /// The HPO document.
    hpo_doc: fastobo::ast::OboDoc,
    /// The index.
    index: tantivy::Index,
    /// The index schema.
    schema: tantivy::schema::Schema,
    /// The single reader.
    reader: tantivy::IndexReader,
}

/// Convert ident to String.
fn ident_to_string(ident: &fastobo::ast::Ident) -> String {
    match ident {
        fastobo::ast::Ident::Prefixed(val) => format!("{}:{}", val.prefix(), val.local()),
        fastobo::ast::Ident::Unprefixed(val) => val.as_str().to_string(),
        fastobo::ast::Ident::Url(val) => val.as_str().to_string(),
    }
}

// Code for creating an `Index`.
impl Index {
    /// Create a new index from an OBO document.
    ///
    /// # Args
    ///
    /// * `hpo_doc` - The OBO document to index.
    ///
    /// # Errors
    ///
    /// In the case that the index cannot be created.
    pub fn new(hpo_doc: fastobo::ast::OboDoc) -> Result<Self, anyhow::Error> {
        let schema = Self::build_schema();
        let tmpdir = tempdir::TempDir::new("viguno")?;
        let index = tantivy::Index::create_in_dir(tmpdir.path(), schema.clone())?;

        let mut index_writer = index.writer(100_000_000).map_err(|e| {
            anyhow::anyhow!(
                "Error creating tantivy index writer for directory {:?}: {}",
                tmpdir.path(),
                e
            )
        })?;
        Self::write_hpo_index(&hpo_doc, &schema, &mut index_writer).map_err(|e| {
            anyhow::anyhow!(
                "Error writing HPO index for directory {:?}: {}",
                tmpdir.path(),
                e
            )
        })?;

        let reader = index
            .reader_builder()
            .reload_policy(tantivy::ReloadPolicy::Manual)
            .try_into()?;

        Ok(Self {
            tmpdir,
            hpo_doc,
            index,
            schema,
            reader,
        })
    }

    /// Build the tantivy schema for the HPO.
    fn build_schema() -> Schema {
        use tantivy::schema::{STORED, STRING, TEXT};

        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("term_id", STRING | STORED);
        schema_builder.add_text_field("alt_id", STRING | STORED);
        schema_builder.add_text_field("name", TEXT | STORED);
        schema_builder.add_text_field("def", TEXT | STORED);
        schema_builder.add_text_field("synonym", TEXT | STORED);
        schema_builder.add_text_field("xref", STRING | STORED);
        schema_builder.build()
    }

    /// Index the HPO document.
    fn write_hpo_index(
        hpo_doc: &fastobo::ast::OboDoc,
        schema: &tantivy::schema::Schema,
        index_writer: &mut tantivy::IndexWriter,
    ) -> Result<(), anyhow::Error> {
        for term_frame in hpo_doc
            .entities()
            .iter()
            .filter_map(fastobo::ast::EntityFrame::as_term)
        {
            let mut doc = tantivy::TantivyDocument::default();

            doc.add_field_value(
                schema.get_field("term_id")?,
                ident_to_string(term_frame.id().as_inner().as_ref()),
            );

            for line in term_frame
                .clauses()
                .iter()
                .map(fastobo::ast::Line::as_inner)
            {
                match line {
                    fastobo::ast::TermClause::Name(name) => {
                        doc.add_field_value(schema.get_field("name")?, name.as_str());
                    }
                    fastobo::ast::TermClause::AltId(alt_id) => {
                        doc.add_field_value(
                            schema.get_field("alt_id")?,
                            ident_to_string(alt_id).as_str(),
                        );
                    }
                    fastobo::ast::TermClause::Def(def) => {
                        doc.add_field_value(schema.get_field("def")?, def.text().as_str());
                    }
                    fastobo::ast::TermClause::Synonym(synonym) => {
                        doc.add_field_value(
                            schema.get_field("synonym")?,
                            synonym.description().as_str(),
                        );
                    }
                    fastobo::ast::TermClause::Xref(xref) => {
                        doc.add_field_value(schema.get_field("xref")?, ident_to_string(xref.id()));
                    }
                    fastobo::ast::TermClause::IsAnonymous(_)
                    | fastobo::ast::TermClause::Comment(_)
                    | fastobo::ast::TermClause::Namespace(_)
                    | fastobo::ast::TermClause::Subset(_)
                    | fastobo::ast::TermClause::Builtin(_)
                    | fastobo::ast::TermClause::PropertyValue(_)
                    | fastobo::ast::TermClause::IsA(_)
                    | fastobo::ast::TermClause::IntersectionOf(_, _)
                    | fastobo::ast::TermClause::UnionOf(_)
                    | fastobo::ast::TermClause::EquivalentTo(_)
                    | fastobo::ast::TermClause::DisjointFrom(_)
                    | fastobo::ast::TermClause::Relationship(_, _)
                    | fastobo::ast::TermClause::CreatedBy(_)
                    | fastobo::ast::TermClause::CreationDate(_)
                    | fastobo::ast::TermClause::IsObsolete(_)
                    | fastobo::ast::TermClause::ReplacedBy(_)
                    | fastobo::ast::TermClause::Consider(_) => (),
                }
            }

            index_writer.add_document(doc).map_err(|e| {
                anyhow::anyhow!(
                    "Error adding document to tantivy index writer: {}",
                    e.to_string()
                )
            })?;
        }

        index_writer
            .commit()
            .map_err(|e| anyhow::anyhow!("Error committing tantivy index writer: {}", e))?;
        Ok(())
    }
}

// Accessor code.
impl Index {
    /// Get the HPO document.
    pub fn hpo_doc(&self) -> &fastobo::ast::OboDoc {
        &self.hpo_doc
    }

    /// Get the index.
    pub fn index(&self) -> &tantivy::Index {
        &self.index
    }

    /// Get the schema.
    pub fn schema(&self) -> &tantivy::schema::Schema {
        &self.schema
    }

    /// Get the reader.
    pub fn reader(&self) -> &tantivy::IndexReader {
        &self.reader
    }
}
