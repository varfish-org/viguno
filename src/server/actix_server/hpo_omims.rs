//! Implementation of `/hpo/omims`.

use actix_web::{
    get,
    web::{self, Data, Json, Path},
    Responder,
};
use hpo::{
    annotations::{OmimDisease, OmimDiseaseId},
    term::HpoGroup,
    Ontology,
};
use serde::{Deserialize, Serialize};

use crate::server::WebServerData;

use super::{CustomError, Match, ResultHpoTerm};

/// Parameters for `handle`.
///
/// This allows to query for diseases.  The first given of the following
/// is interpreted.
///
/// - `omim_id` -- specify disease ID
/// - `name` -- specify the name to query for
/// - `max_results` -- the maximum number of records to return
/// - `hpo_terms` -- whether to include `"hpo_terms"` in result
///
/// The following propery defines how matches are performed:
///
/// - `match` -- how to match
#[derive(Deserialize, Debug, Clone)]
struct Request {
    /// The OMIM ID to search for.
    pub omim_id: Option<String>,
    /// The disease name to search for.
    pub name: Option<String>,
    /// The match mode.
    #[serde(alias = "match")]
    pub match_: Option<Match>,
    /// Maximal number of results to return.
    #[serde(default = "_default_max_results")]
    pub max_results: usize,
    /// Whether to include HPO terms.
    #[serde(default = "_default_hpo_terms")]
    pub hpo_terms: bool,
}

/// Return default of `Request::max_results`.
fn _default_max_results() -> usize {
    100
}

/// Return default of `Request::hpo_terms`.
fn _default_hpo_terms() -> bool {
    false
}

/// Result entry for `handle`.
#[derive(Serialize, Debug, Clone)]
struct ResultEntry {
    /// The OMIM ID.
    pub omim_id: String,
    /// The OMIM disease name.
    pub name: String,
    /// The gene's associated HPO terms.
    #[serde(default = "Option::default", skip_serializing_if = "Option::is_none")]
    pub hpo_terms: Option<Vec<ResultHpoTerm>>,
}

impl ResultEntry {
    pub fn from_omim_disease_with_ontology(
        omim_disease: &OmimDisease,
        ontology: &Ontology,
        hpo_terms: bool,
    ) -> Self {
        let hpo_terms = if hpo_terms {
            Some(
                omim_disease
                    .to_hpo_set(ontology)
                    .child_nodes()
                    .into_iter()
                    .collect::<HpoGroup>()
                    .into_iter()
                    .map(|term_id| ontology.hpo(term_id))
                    .filter(std::option::Option::is_some)
                    .map(|term| {
                        let term = term.expect("filtered above");
                        ResultHpoTerm {
                            term_id: term.id().to_string(),
                            name: term.name().to_string(),
                        }
                    })
                    .collect(),
            )
        } else {
            None
        };
        ResultEntry {
            omim_id: omim_disease.id().to_string(),
            name: omim_disease.name().to_string(),
            hpo_terms,
        }
    }
}

/// Query for OMIM diseases in the HPO database.
#[allow(clippy::unused_async)]
#[get("/hpo/omims")]
async fn handle(
    data: Data<WebServerData>,
    _path: Path<()>,
    query: web::Query<Request>,
) -> actix_web::Result<impl Responder, CustomError> {
    let ontology = &data.ontology;
    let match_ = query.match_.unwrap_or_default();
    let mut result: Vec<ResultEntry> = Vec::new();

    if match_ == Match::Exact {
        let omim_disease = if let Some(omim_id) = &query.omim_id {
            let omim_id = OmimDiseaseId::try_from(omim_id.as_ref())
                .map_err(|e| CustomError::new(anyhow::anyhow!(e)))?;
            ontology.omim_disease(&omim_id)
        } else if let Some(name) = &query.name {
            let mut omim_disease = None;
            let mut it = ontology.omim_diseases();
            let mut tmp = it.next();
            while tmp.is_some() && omim_disease.is_none() {
                if tmp.expect("checked above").name() == name {
                    omim_disease = tmp;
                }
                tmp = it.next();
            }
            omim_disease
        } else {
            None
        };
        if let Some(omim_disease) = &omim_disease {
            result.push(ResultEntry::from_omim_disease_with_ontology(
                omim_disease,
                ontology,
                query.hpo_terms,
            ));
        }
    } else if let Some(name) = &query.name {
        let mut it = ontology.omim_diseases();
        let mut omim_disease = it.next();
        while omim_disease.is_some() && result.len() < query.max_results {
            let omim_name = omim_disease.as_ref().expect("checked above").name();
            let is_match = match query.match_.unwrap_or_default() {
                Match::Prefix => omim_name.starts_with(name),
                Match::Suffix => omim_name.ends_with(name),
                Match::Contains => omim_name.contains(name),
                Match::Exact => panic!("cannot happen here"),
            };
            if is_match {
                result.push(ResultEntry::from_omim_disease_with_ontology(
                    omim_disease.as_ref().expect("checked above"),
                    ontology,
                    query.hpo_terms,
                ));
            }

            omim_disease = it.next();
        }
    }

    Ok(Json(result))
}

#[cfg(test)]
mod test {
    #[test]
    fn test_handle() -> Result<(), anyhow::Error> {
        assert!(false, "actually write the test");

        Ok(())
    }
}
