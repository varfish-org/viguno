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

use crate::{common::Version, server::run::WebServerData};

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
#[derive(
    serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::IntoParams, Debug, Clone,
)]
#[schema(title = "HpoOmimsQuery")]
pub struct Query {
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
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, Debug, Clone)]
#[schema(title = "HpoOmimsResultEntry")]
pub struct ResultEntry {
    /// The OMIM ID.
    pub omim_id: String,
    /// The OMIM disease name.
    pub name: String,
    /// The gene's associated HPO terms.
    #[serde(default = "Option::default", skip_serializing_if = "Option::is_none")]
    pub hpo_terms: Option<Vec<ResultHpoTerm>>,
}

impl PartialEq for ResultEntry {
    fn eq(&self, other: &Self) -> bool {
        (self.omim_id == other.omim_id) && (self.name == other.name)
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
        match self.omim_id.cmp(&other.omim_id) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.name.cmp(&other.name)
    }
}

impl ResultEntry {
    /// Create a `ResultEntry` from an `OmimDisease`.
    pub fn from_omim_disease_with_ontology(
        omim_disease: &OmimDisease,
        ontology: &Ontology,
        hpo_terms: bool,
    ) -> Self {
        let hpo_terms = if hpo_terms {
            let mut result = omim_disease
                .to_hpo_set(ontology)
                .child_nodes()
                .into_iter()
                .collect::<HpoGroup>()
                .into_iter()
                .filter_map(|term_id| ontology.hpo(term_id))
                .map(|term| ResultHpoTerm {
                    term_id: term.id().to_string(),
                    name: term.name().to_string(),
                })
                .collect::<Vec<_>>();
            result.sort();
            Some(result)
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

/// Container for the result.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, Debug)]
#[schema(title = "HpoOmimsResult")]
pub struct Result {
    /// Version information.
    pub version: Version,
    /// The original query records.
    pub query: Query,
    /// The resulting records for the scored genes.
    pub result: Vec<ResultEntry>,
}

/// Query for OMIM diseases in the HPO database.
#[allow(clippy::unused_async)]
#[utoipa::path(
    params(Query),
    responses(
        (status = 200, description = "The query was successful.", body = Result),
    )
)]
#[get("/hpo/omims")]
async fn handle(
    data: Data<WebServerData>,
    _path: Path<()>,
    query: web::Query<Query>,
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

    result.sort();

    let result = Result {
        version: Version::new(&data.ontology.hpo_version()),
        query: query.into_inner(),
        result,
    };

    Ok(Json(result))
}

#[cfg(test)]
mod test {
    /// Helper function for running a query.
    #[allow(dead_code)]
    async fn run_query(uri: &str) -> Result<super::Result, anyhow::Error> {
        let ontology = crate::common::load_hpo("tests/data/hpo")?;
        let ncbi_to_hgnc =
            crate::common::hgnc_xlink::load_ncbi_to_hgnc("tests/data/hgnc_xlink.tsv")?;
        let hgnc_to_ncbi = crate::common::hgnc_xlink::inverse_hashmap(&ncbi_to_hgnc);
        let hpo_doc = fastobo::from_file("tests/data/hpo/hp.obo")?;
        let app = actix_web::test::init_service(
            actix_web::App::new()
                .app_data(actix_web::web::Data::new(
                    crate::server::run::WebServerData {
                        ontology,
                        ncbi_to_hgnc,
                        hgnc_to_ncbi,
                        full_text_index: crate::index::Index::new(hpo_doc)?,
                    },
                ))
                .service(super::handle),
        )
        .await;
        let req = actix_web::test::TestRequest::get().uri(uri).to_request();
        let resp: super::Result = actix_web::test::call_and_read_body_json(&app, req).await;

        Ok(resp)
    }

    #[actix_web::test]
    async fn hpo_omims_omim_id_exact_no_hpo_terms() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/omims?omim_id=616145").await?
        ))
    }

    #[actix_web::test]
    async fn hpo_omims_omim_id_exact_with_hpo_terms() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/omims?omim_id=616145&hpo_terms=true").await?
        ))
    }

    #[actix_web::test]
    async fn hpo_omims_name_exact_no_hpo_terms() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/omims?name=Catel-Manzke+syndrome").await?
        ))
    }

    #[actix_web::test]
    async fn hpo_omims_name_exact_with_hpo_terms() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/omims?name=Catel-Manzke+syndrome&hpo_terms=true").await?
        ))
    }

    #[actix_web::test]
    async fn hpo_omims_name_prefix_no_hpo_terms() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/omims?name=Catel-Manzke+syndro&match=prefix").await?
        ))
    }

    #[actix_web::test]
    async fn hpo_omims_name_prefix_with_hpo_terms() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/omims?name=Catel-Manzke+syndro&match=prefix&hpo_terms=true").await?
        ))
    }

    #[actix_web::test]
    async fn hpo_omims_name_suffix_no_hpo_terms() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/omims?name=tel-Manzke+syndrome&match=suffix").await?
        ))
    }

    #[actix_web::test]
    async fn hpo_omims_name_suffix_with_hpo_terms() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/omims?name=tel-Manzke+syndrome&match=suffix&hpo_terms=true").await?
        ))
    }

    #[actix_web::test]
    async fn hpo_omims_name_contains_no_hpo_terms() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/omims?name=tel-Manzke+syndro&match=contains").await?
        ))
    }

    #[actix_web::test]
    async fn hpo_omims_name_contains_with_hpo_terms() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/omims?name=tel-Manzke+syndro&match=contains&hpo_terms=true").await?
        ))
    }
}
