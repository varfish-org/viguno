//! Implementation of `/hpo/terms`.

use std::{collections::HashMap, sync::Arc};

use actix_web::{
    get,
    web::{self, Data, Json, Path},
    Responder,
};
use hpo::{annotations::AnnotationId, HpoTerm, HpoTermId, Ontology};

use crate::{common::Version, server::run::WebServerData};

use super::{CustomError, ResultGene};

/// Parameters for `handle`.
///
/// This allows to query for terms.  The first given of the following is
/// interpreted.
///
/// - `term_id` -- specify term ID
/// - `gene_symbol` -- specify the gene symbol
/// - `max_results` -- the maximum number of records to return
/// - `genes` -- whether to include `"genes"` in result
#[derive(
    serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::IntoParams, Debug, Clone,
)]
#[schema(title = "HpoTermsQuery")]
pub struct Query {
    /// The term ID to search for.
    pub term_id: Option<String>,
    /// The term name to search for.
    pub name: Option<String>,
    /// Maximal number of results to return.
    #[serde(default = "_default_max_results")]
    pub max_results: usize,
    /// Whether to include genes.
    #[serde(default = "_default_genes")]
    pub genes: bool,
}

/// Return default of `Request::max_results`.
fn _default_max_results() -> usize {
    100
}

/// Return default of `Request::genes`.
fn _default_genes() -> bool {
    false
}

/// Result entry for `fetch_hpo_genes`.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, Debug, Clone)]
#[schema(title = "HpoTermsResultEntry")]
pub struct ResultEntry {
    /// The HPO term's ID.
    pub term_id: String,
    /// The HPO term's name.
    pub name: String,
    /// Any matching description.
    pub definition: Option<String>,
    /// Any matching synonyms.
    pub synonyms: Option<Vec<String>>,
    /// Any matching xref.
    pub xrefs: Option<Vec<String>>,
    /// The gene's associated HPO terms.
    #[serde(default = "Option::default", skip_serializing_if = "Option::is_none")]
    pub genes: Option<Vec<ResultGene>>,
}

impl PartialEq for ResultEntry {
    fn eq(&self, other: &Self) -> bool {
        (self.term_id == other.term_id) && (self.name == other.name)
    }
}

impl Eq for ResultEntry {}

impl PartialOrd for ResultEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ResultEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.term_id.cmp(&other.term_id) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.name.cmp(&other.name)
    }
}

impl ResultEntry {
    /// Create a `ResultEntry` from an `HpoTerm`.
    #[allow(clippy::missing_panics_doc)]
    pub fn from_term_with_ontology(
        term: &HpoTerm,
        ontology: &Ontology,
        genes: bool,
        ncbi_to_hgnc: &HashMap<u32, String>,
        index: &crate::index::Index,
        doc: Option<&tantivy::Document>,
    ) -> Result<Self, anyhow::Error> {
        let field_term_id = index
            .schema()
            .get_field("term_id")
            .map_err(|e| anyhow::anyhow!("field term_id must exist: {}", e))?;
        let field_def = index
            .index()
            .schema()
            .get_field("def")
            .map_err(|e| anyhow::anyhow!("field def must exist: {}", e))?;
        let field_synonym = index
            .index()
            .schema()
            .get_field("synonym")
            .map_err(|e| anyhow::anyhow!("field synonym must exist: {}", e))?;
        let field_xref = index
            .index()
            .schema()
            .get_field("xref")
            .map_err(|e| anyhow::anyhow!("field xref must exist: {}", e))?;

        let searcher = index.reader().searcher();
        let doc = if let Some(doc) = doc {
            doc.clone()
        } else {
            let query_parser =
                tantivy::query::QueryParser::for_index(index.index(), vec![field_term_id]);
            let query = query_parser
                .parse_query(&format!("\"{}\"", term.id()))
                .map_err(|e| anyhow::anyhow!("problem with term ID query: {}", e))?;
            let top_docs = searcher
                .search(&query, &tantivy::collector::TopDocs::with_limit(1))
                .map_err(|e| anyhow::anyhow!("problem searching for query: {}", e))?;

            searcher
                .doc(top_docs[0].1)
                .map_err(|e| anyhow::anyhow!("problem retrieving document: {}", e))?
        };

        let definition = doc
            .get_all(field_def)
            .filter_map(|f| f.as_text().map(std::string::ToString::to_string))
            .collect::<Vec<_>>();
        let definition = definition.first().cloned();
        let synonyms = doc
            .get_all(field_synonym)
            .filter_map(|f| f.as_text().map(std::string::ToString::to_string))
            .collect::<Vec<_>>();
        let synonyms = if synonyms.is_empty() {
            None
        } else {
            Some(synonyms)
        };
        let xrefs = doc
            .get_all(field_xref)
            .filter_map(|f| f.as_text().map(std::string::ToString::to_string))
            .collect::<Vec<_>>();
        let xrefs = if xrefs.is_empty() { None } else { Some(xrefs) };

        let genes = if genes {
            let mut genes = term
                .gene_ids()
                .iter()
                .filter_map(|gene_id| ontology.gene(gene_id))
                .map(|gene| ResultGene {
                    ncbi_gene_id: gene.id().as_u32(),
                    gene_symbol: gene.name().to_string(),
                    hgnc_id: ncbi_to_hgnc.get(&gene.id().as_u32()).cloned(),
                })
                .collect::<Vec<_>>();
            genes.sort();
            Some(genes)
        } else {
            None
        };
        Ok(ResultEntry {
            term_id: term.id().to_string(),
            name: term.name().to_string(),
            genes,
            definition,
            synonyms,
            xrefs,
        })
    }
}

/// Container for the result.
#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
#[schema(title = "HpoTermsResult")]
pub struct Result_ {
    /// Version information.
    pub version: Version,
    /// The original query records.
    pub query: Query,
    /// The resulting records for the scored genes.
    pub result: Vec<ResultEntry>,
}

/// Query for terms in the HPO database.
///
/// # Errors
///
/// In the case that there is an error running the server.
#[allow(clippy::unused_async)]
#[allow(clippy::too_many_lines)]
#[utoipa::path(
    operation_id = "hpo_terms",
    params(Query),
    responses(
        (status = 200, description = "The query was successful.", body = Result),
    )
)]
#[get("/hpo/terms")]
async fn handle(
    data: Data<Arc<WebServerData>>,
    _path: Path<()>,
    query: web::Query<Query>,
) -> actix_web::Result<impl Responder, CustomError> {
    let ontology = &data.ontology;
    let mut result: Vec<ResultEntry> = Vec::new();

    let field_term_id = data
        .full_text_index
        .index()
        .schema()
        .get_field("term_id")
        .expect("field must exist");
    let field_alt_id = data
        .full_text_index
        .schema()
        .get_field("alt_id")
        .expect("field must exist");
    let field_name = data
        .full_text_index
        .index()
        .schema()
        .get_field("name")
        .expect("field must exist");
    let field_def = data
        .full_text_index
        .index()
        .schema()
        .get_field("def")
        .expect("field must exist");
    let field_synonym = data
        .full_text_index
        .index()
        .schema()
        .get_field("synonym")
        .expect("field must exist");
    let field_xref = data
        .full_text_index
        .index()
        .schema()
        .get_field("xref")
        .expect("field must exist");

    if let Some(term_id) = &query.term_id {
        let re = regex::Regex::new(r"hpo:\d+$").unwrap();
        if !re.is_match(&term_id.to_lowercase()) {
            return Err(CustomError::new(anyhow::anyhow!(
                "Invalid term ID: {}",
                term_id
            )));
        }

        let term_id = HpoTermId::from(term_id.clone());
        let term = ontology.hpo(term_id).ok_or_else(|| {
            CustomError::new(anyhow::anyhow!("Term ID {} not found in HPO", term_id))
        })?;
        result.push(
            ResultEntry::from_term_with_ontology(
                &term,
                ontology,
                query.genes,
                &data.ncbi_to_hgnc,
                &data.full_text_index,
                None,
            )
            .map_err(|e| CustomError::new(anyhow::anyhow!("Problem parsing term: {}", e)))?,
        );
    } else if let Some(name) = &query.name {
        let searcher = data.full_text_index.reader().searcher();
        let query_parser = {
            let mut query_parser = tantivy::query::QueryParser::for_index(
                data.full_text_index.index(),
                vec![
                    field_term_id,
                    field_alt_id,
                    field_name,
                    field_def,
                    field_synonym,
                    field_xref,
                ],
            );
            query_parser.set_conjunction_by_default();
            query_parser.set_field_boost(field_name, 3.0);
            query_parser.set_field_boost(field_synonym, 0.8);
            query_parser.set_field_boost(field_def, 0.6);
            query_parser.set_field_fuzzy(field_name, true, 1, true);
            query_parser.set_field_fuzzy(field_def, true, 1, true);
            query_parser.set_field_fuzzy(field_synonym, true, 1, true);
            query_parser
        };
        let name = if name.contains(":") {
            format!("\"{name}\"")
        } else {
            name.to_string()
        };
        let index_query = query_parser
            .parse_query(&name)
            .map_err(|e| CustomError::new(anyhow::anyhow!("Error parsing query: {}", e)))?;
        let top_docs = searcher
            .search(
                &index_query,
                &tantivy::collector::TopDocs::with_limit(query.max_results),
            )
            .map_err(|e| CustomError::new(anyhow::anyhow!("Error searching index: {}", e)))?;

        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address).map_err(|e| {
                CustomError::new(anyhow::anyhow!("Error retrieving document: {}", e))
            })?;
            let term_id = retrieved_doc
                .get_first(field_term_id)
                .ok_or_else(|| {
                    CustomError::new(anyhow::anyhow!("Document has no `term_id` field"))
                })?
                .as_text()
                .unwrap_or_default();
            let term_id = HpoTermId::from(term_id.to_string());
            let term = ontology.hpo(term_id).ok_or_else(|| {
                CustomError::new(anyhow::anyhow!("Term ID {} not found in HPO", term_id))
            })?;

            result.push(
                ResultEntry::from_term_with_ontology(
                    &term,
                    ontology,
                    query.genes,
                    &data.ncbi_to_hgnc,
                    &data.full_text_index,
                    Some(&retrieved_doc),
                )
                .map_err(|e| CustomError::new(anyhow::anyhow!("Problem parsing term: {}", e)))?,
            );
        }
    };

    let result = Result_ {
        version: Version::new(&data.ontology.hpo_version()),
        query: query.into_inner(),
        result,
    };

    Ok(Json(result))
}

#[cfg(test)]
mod test {
    use std::sync::Arc;

    use crate::server::run::hpo_genes::test::web_server_data;

    /// Helper function for running a query.
    #[allow(dead_code)]
    pub async fn run_query(
        web_server_data: Arc<crate::server::run::WebServerData>,
        uri: &str,
    ) -> Result<super::Result_, anyhow::Error> {
        let app = actix_web::test::init_service(
            actix_web::App::new()
                .app_data(actix_web::web::Data::new(web_server_data))
                .service(super::handle),
        )
        .await;
        let req = actix_web::test::TestRequest::get().uri(uri).to_request();
        let resp: super::Result_ = actix_web::test::call_and_read_body_json(&app, req).await;

        Ok(resp)
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_terms_term_id_exact_no_genes(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(web_server_data.clone(), "/hpo/terms?term_id=HP:0000023").await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_terms_term_id_exact_with_genes(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/hpo/terms?term_id=HP:0000023&genes=true"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_terms_name_exact_no_genes(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(web_server_data.clone(), "/hpo/terms?name=Inguinal+hernia").await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_terms_name_exact_with_genes(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/hpo/terms?name=Inguinal+hernia&genes=true"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_terms_name_fuzzy_no_genes(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(web_server_data.clone(), "/hpo/terms?name=Inguinal+hern").await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_terms_name_fuzzy_with_genes(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/hpo/terms?name=Inguinal+hern&genes=true"
            )
            .await?
        ))
    }
}
