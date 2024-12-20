//! Implementation of `/hpo/omims`.

use std::sync::Arc;

use actix_web::{
    get,
    web::{self, Data, Json, Path},
};
use hpo::{
    annotations::{Disease as _, OmimDisease, OmimDiseaseId},
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
    Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema, utoipa::IntoParams,
)]
pub struct HpoOmimsQuery {
    /// The OMIM ID to search for.
    pub omim_id: Option<String>,
    /// The disease name to search for.
    pub name: Option<String>,
    /// The match mode, default is `Match::Exact`.
    pub r#match: Option<Match>,
    /// Whether case is insentivie, default is `false`.
    pub ignore_case: Option<bool>,
    /// Maximal number of results to return.
    #[serde(default = "_default_max_results")]
    pub max_results: usize,
    /// Whether to include HPO terms.
    #[serde(default = "_default_hpo_terms")]
    pub hpo_terms: bool,
}

impl HpoOmimsQuery {
    /// Strip "OMIM:" prefix from `omim_id`, if any.
    fn with_stripped_prefix(self) -> Self {
        Self {
            omim_id: self.omim_id.map(|omim_id| {
                let lower_omim_id = omim_id.to_lowercase();
                if lower_omim_id.starts_with("omim:") {
                    omim_id[5..].to_string()
                } else if lower_omim_id.starts_with("mim:") {
                    omim_id[4..].to_string()
                } else {
                    omim_id
                }
            }),
            ..self
        }
    }
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
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct HpoOmimsResultEntry {
    /// The OMIM ID.
    pub omim_id: String,
    /// The OMIM disease name.
    pub name: String,
    /// The gene's associated HPO terms.
    #[serde(default = "Option::default", skip_serializing_if = "Option::is_none")]
    pub hpo_terms: Option<Vec<ResultHpoTerm>>,
}

impl PartialEq for HpoOmimsResultEntry {
    fn eq(&self, other: &Self) -> bool {
        (self.omim_id == other.omim_id) && (self.name == other.name)
    }
}

impl Eq for HpoOmimsResultEntry {}

impl PartialOrd for HpoOmimsResultEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HpoOmimsResultEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        match self.omim_id.cmp(&other.omim_id) {
            core::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        self.name.cmp(&other.name)
    }
}

impl HpoOmimsResultEntry {
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
        HpoOmimsResultEntry {
            omim_id: omim_disease.id().to_string(),
            name: omim_disease.name().to_string(),
            hpo_terms,
        }
    }
}

/// Container for the result.
#[derive(Debug, serde::Serialize, serde::Deserialize, utoipa::ToSchema)]
pub struct HpoOmimsResult {
    /// Version information.
    pub version: Version,
    /// The original query records.
    pub query: HpoOmimsQuery,
    /// The resulting records for the scored genes.
    pub result: Vec<HpoOmimsResultEntry>,
}

/// Query for OMIM diseases in the HPO database.
#[allow(clippy::too_many_lines)]
#[utoipa::path(
    get,
    operation_id = "hpoOmims",
    params(HpoOmimsQuery),
    responses(
        (status = 200, description = "The query was successful.", body = HpoOmimsResult),
        (status = 500, description = "The server encountered an error.", body = CustomError)
    )
)]
#[get("/api/v1/hpo/omims")]
async fn handle(
    data: Data<Arc<WebServerData>>,
    _path: Path<()>,
    query: web::Query<HpoOmimsQuery>,
) -> actix_web::Result<Json<HpoOmimsResult>, CustomError> {
    let ontology = &data.ontology;
    let match_ = query.r#match.unwrap_or_default();
    let mut result: Vec<HpoOmimsResultEntry> = Vec::new();

    // Strip "OMIM:" and "MIM:" prefix from `query.omim_id` if given.
    let query = query.into_inner().with_stripped_prefix();

    if match_ == Match::Exact {
        let omim_disease = if let Some(omim_id) = &query.omim_id {
            let omim_id = OmimDiseaseId::try_from(omim_id.as_ref())
                .map_err(|e| CustomError::new(anyhow::anyhow!(e)))?;
            ontology.omim_disease(&omim_id)
        } else if let Some(name) = &query.name {
            let name = if query.ignore_case.unwrap_or_default() {
                name.to_lowercase()
            } else {
                name.clone()
            };
            let mut omim_disease = None;
            let mut it = ontology.omim_diseases();
            let mut tmp = it.next();
            while tmp.is_some() && omim_disease.is_none() {
                let tmp_name = tmp.expect("checked above").name();
                let tmp_name = if query.ignore_case.unwrap_or_default() {
                    tmp_name.to_lowercase()
                } else {
                    tmp_name.to_string()
                };
                if tmp_name == name {
                    omim_disease = tmp;
                }
                tmp = it.next();
            }
            omim_disease
        } else {
            None
        };
        if let Some(omim_disease) = &omim_disease {
            result.push(HpoOmimsResultEntry::from_omim_disease_with_ontology(
                omim_disease,
                ontology,
                query.hpo_terms,
            ));
        }
    } else if let Some(name) = &query.name {
        let mut it = ontology.omim_diseases();
        let mut omim_disease = it.next();
        while omim_disease.is_some() && result.len() < query.max_results {
            let name = if query.ignore_case.unwrap_or_default() {
                name.to_lowercase()
            } else {
                name.clone()
            };
            let omim_name = omim_disease.as_ref().expect("checked above").name();
            let omim_name = if query.ignore_case.unwrap_or_default() {
                omim_name.to_lowercase()
            } else {
                omim_name.to_string()
            };
            let is_match = match query.r#match.unwrap_or_default() {
                Match::Prefix => omim_name.starts_with(&name),
                Match::Suffix => omim_name.ends_with(&name),
                Match::Contains => omim_name.contains(&name),
                Match::Exact => panic!("cannot happen here"),
            };
            if is_match {
                result.push(HpoOmimsResultEntry::from_omim_disease_with_ontology(
                    omim_disease.as_ref().expect("checked above"),
                    ontology,
                    query.hpo_terms,
                ));
            }

            omim_disease = it.next();
        }
    }

    result.sort();

    let result = HpoOmimsResult {
        version: Version::new(&data.ontology.hpo_version()),
        query,
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
    ) -> Result<super::HpoOmimsResult, anyhow::Error> {
        let app = actix_web::test::init_service(
            actix_web::App::new()
                .app_data(actix_web::web::Data::new(web_server_data))
                .service(super::handle),
        )
        .await;
        let req = actix_web::test::TestRequest::get().uri(uri).to_request();
        let resp: super::HpoOmimsResult = actix_web::test::call_and_read_body_json(&app, req).await;

        Ok(resp)
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_omims_omim_id_exact_no_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(web_server_data.clone(), "/api/v1/hpo/omims?omim_id=616145").await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_omims_omim_id_exact_with_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/omims?omim_id=616145&hpo_terms=true"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_omims_name_exact_no_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/omims?name=Catel-Manzke+syndrome"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_omims_name_exact_with_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/omims?name=Catel-Manzke+syndrome&hpo_terms=true"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_omims_name_prefix_no_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/omims?name=Catel-Manzke+syndro&match=prefix"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_omims_name_prefix_with_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/omims?name=Catel-Manzke+syndro&match=prefix&hpo_terms=true"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_omims_name_suffix_no_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/omims?name=tel-Manzke+syndrome&match=suffix"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_omims_name_suffix_with_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/omims?name=tel-Manzke+syndrome&match=suffix&hpo_terms=true"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_omims_name_contains_no_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/omims?name=tel-Manzke+syndro&match=contains"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_omims_name_contains_with_hpo_terms(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/api/v1/hpo/omims?name=tel-Manzke+syndro&match=contains&hpo_terms=true"
            )
            .await?
        ))
    }
}
