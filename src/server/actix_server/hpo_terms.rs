//! Implementation of `/hpo/terms`.

use std::collections::HashMap;

use actix_web::{
    get,
    web::{self, Data, Json, Path},
    Responder,
};
use hpo::{annotations::AnnotationId, HpoTerm, HpoTermId, Ontology};

use crate::server::WebServerData;

use super::{CustomError, Match, ResultGene};

/// Parameters for `handle`.
///
/// This allows to query for terms.  The first given of the following is
/// interpreted.
///
/// - `term_id` -- specify term ID
/// - `gene_symbol` -- specify the gene symbol
/// - `max_results` -- the maximum number of records to return
/// - `genes` -- whether to include `"genes"` in result
///
/// The following propery defines how matches are performed:
///
/// - `match` -- how to match
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct Query {
    /// The term ID to search for.
    pub term_id: Option<String>,
    /// The term name to search for.
    pub name: Option<String>,
    /// The match mode.
    #[serde(alias = "match")]
    pub match_: Option<Match>,
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
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
struct ResultEntry {
    /// The HPO term's ID.
    pub term_id: String,
    /// The HPO term's name.
    pub name: String,
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
        match self.term_id.partial_cmp(&other.term_id) {
            Some(core::cmp::Ordering::Equal) => {}
            ord => return ord,
        }
        self.name.partial_cmp(&other.name)
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
    pub fn from_term_with_ontology(
        term: &HpoTerm,
        ontology: &Ontology,
        genes: bool,
        ncbi_to_hgnc: &HashMap<u32, String>,
    ) -> Self {
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
        ResultEntry {
            term_id: term.id().to_string(),
            name: term.name().to_string(),
            genes,
        }
    }
}

/// Container for the result.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct Container {
    /// Version information.
    pub version: crate::common::Version,
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
#[get("/hpo/terms")]
async fn handle(
    data: Data<WebServerData>,
    _path: Path<()>,
    query: web::Query<Query>,
) -> actix_web::Result<impl Responder, CustomError> {
    let ontology = &data.ontology;
    let match_ = query.match_.unwrap_or_default();
    let mut result: Vec<ResultEntry> = Vec::new();

    if match_ == Match::Exact {
        let term = if let Some(term_id) = &query.term_id {
            let term_id = HpoTermId::from(term_id.clone());
            ontology.hpo(term_id)
        } else if let Some(name) = &query.name {
            let mut term = None;
            let mut it = ontology.hpos();
            let mut tmp = it.next();
            while tmp.is_some() && term.is_none() {
                if tmp.expect("checked above").name() == name {
                    term = tmp;
                }
                tmp = it.next();
            }
            term
        } else {
            None
        };
        if let Some(term) = &term {
            result.push(ResultEntry::from_term_with_ontology(
                term,
                ontology,
                query.genes,
                &data.ncbi_to_hgnc,
            ));
        }
    } else if let Some(name) = &query.name {
        let mut it = ontology.hpos();
        let mut term = it.next();
        while term.is_some() && result.len() < query.max_results {
            let term_name = term.as_ref().expect("checked above").name();
            let is_match = match query.match_.unwrap_or_default() {
                Match::Prefix => term_name.starts_with(name),
                Match::Suffix => term_name.ends_with(name),
                Match::Contains => term_name.contains(name),
                Match::Exact => panic!("cannot happen here"),
            };
            if is_match {
                result.push(ResultEntry::from_term_with_ontology(
                    term.as_ref().expect("checked above"),
                    ontology,
                    query.genes,
                    &data.ncbi_to_hgnc,
                ));
            }

            term = it.next();
        }
    }

    result.sort();

    let result = Container {
        version: crate::common::Version::new(&data.ontology.hpo_version()),
        query: query.into_inner(),
        result,
    };

    Ok(Json(result))
}

#[cfg(test)]
mod test {
    /// Helper function for running a query.
    #[allow(dead_code)]
    async fn run_query(uri: &str) -> Result<super::Container, anyhow::Error> {
        let ontology = crate::common::load_hpo("tests/data/hpo")?;
        let ncbi_to_hgnc =
            crate::common::hgnc_xlink::load_ncbi_to_hgnc("tests/data/hgnc_xlink.tsv")?;
        let hgnc_to_ncbi = crate::common::hgnc_xlink::inverse_hashmap(&ncbi_to_hgnc);
        let app = actix_web::test::init_service(
            actix_web::App::new()
                .app_data(actix_web::web::Data::new(crate::server::WebServerData {
                    ontology,
                    db: None,
                    ncbi_to_hgnc,
                    hgnc_to_ncbi,
                }))
                .service(super::handle),
        )
        .await;
        let req = actix_web::test::TestRequest::get().uri(uri).to_request();
        let resp: super::Container = actix_web::test::call_and_read_body_json(&app, req).await;

        Ok(resp)
    }

    #[actix_web::test]
    async fn hpo_terms_term_id_exact_no_genes() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/terms?term_id=HP:0000023").await?
        ))
    }

    #[actix_web::test]
    async fn hpo_terms_term_id_exact_with_genes() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/terms?term_id=HP:0000023&genes=true").await?
        ))
    }

    #[actix_web::test]
    async fn hpo_terms_name_exact_no_genes() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/terms?name=Inguinal+hernia").await?
        ))
    }

    #[actix_web::test]
    async fn hpo_terms_name_exact_with_genes() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/terms?name=Inguinal+hernia&genes=true").await?
        ))
    }

    #[actix_web::test]
    async fn hpo_terms_name_prefix_no_genes() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/terms?name=Inguinal+hern&match=prefix").await?
        ))
    }

    #[actix_web::test]
    async fn hpo_terms_name_prefix_with_genes() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/terms?name=Inguinal+hern&match=prefix&genes=true").await?
        ))
    }

    #[actix_web::test]
    async fn hpo_terms_name_suffix_no_genes() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/terms?name=guinal+hernia&match=suffix").await?
        ))
    }

    #[actix_web::test]
    async fn hpo_terms_name_suffix_with_genes() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/terms?name=guinal+hernia&match=suffix&genes=true").await?
        ))
    }

    #[actix_web::test]
    async fn hpo_terms_name_contains_no_genes() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/terms?name=guinal+hern&match=contains").await?
        ))
    }

    #[actix_web::test]
    async fn hpo_terms_name_contains_with_genes() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/terms?name=guinal+hern&match=contains&genes=true").await?
        ))
    }
}
