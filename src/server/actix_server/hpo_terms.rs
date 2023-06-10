//! Implementation of `/hpo/terms`.

use actix_web::{
    get,
    web::{self, Data, Json, Path},
    Responder,
};
use hpo::{annotations::AnnotationId, HpoTerm, HpoTermId, Ontology};
use serde::{Deserialize, Serialize};

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
#[derive(Deserialize, Debug, Clone)]
struct Request {
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
#[derive(Serialize, Debug, Clone)]
struct ResultEntry {
    /// The HPO term's ID.
    pub term_id: String,
    /// The HPO term's name.
    pub name: String,
    /// The gene's associated HPO terms.
    #[serde(default = "Option::default", skip_serializing_if = "Option::is_none")]
    pub genes: Option<Vec<ResultGene>>,
}

impl ResultEntry {
    pub fn from_term_with_ontology(term: &HpoTerm, ontology: &Ontology, genes: bool) -> Self {
        let genes = if genes {
            Some(
                term.gene_ids()
                    .iter()
                    .map(|gene_id| ontology.gene(gene_id))
                    .filter(std::option::Option::is_some)
                    .map(|term| {
                        let gene = term.expect("filtered above");
                        ResultGene {
                            gene_id: gene.id().as_u32(),
                            gene_symbol: gene.name().to_string(),
                        }
                    })
                    .collect(),
            )
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
    query: web::Query<Request>,
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
                ));
            }

            term = it.next();
        }
    }

    Ok(Json(result))
}

// #[cfg(test)]
// mod test {
//     use actix_web::{http::header::ContentType, test, web, App};

//     #[actix_web::test]
//     async fn test_index_ok() {
//         let app =
//             test::init_service(App::new().servic(super::handle))).await;
//         let req = test::TestRequest::default()
//             .insert_header(ContentType::plaintext())
//             .to_request();
//         let resp = test::call_service(&app, req).await;
//         assert!(resp.status().is_success());
//     }
// }
