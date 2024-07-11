//! Entry point `/hpo/sim/term-term` allows the pairwise similary computation between two sets
//! of HPO terms.

use std::sync::Arc;

use actix_web::{
    get,
    web::{self, Data, Json, Path},
    Responder,
};
use hpo::{
    similarity::{Builtins, Similarity},
    HpoTermId, Ontology,
};
use itertools::Itertools;

use crate::common::{to_pairwise_sim, IcBasedOn, ScoreCombiner, SimilarityMethod, Version};
use crate::server::{run::CustomError, run::WebServerData};

/// Parameters for `handle`.
///
/// This allows to compute differences between
///
/// - `lhs` -- first set of terms to compute similarity for
/// - `rhs` -- econd set of terms to compute similarity for
#[derive(serde::Serialize, serde::Deserialize, utoipa::IntoParams, Default, Debug, Clone)]
pub struct RequestQuery {
    /// The one set of HPO terms to compute similarity for.
    #[serde(deserialize_with = "super::super::vec_str_deserialize")]
    pub lhs: Vec<String>,
    /// The second set of HPO terms to compute similarity for.
    #[serde(deserialize_with = "super::super::vec_str_deserialize")]
    pub rhs: Vec<String>,
    /// What should information content be based on.
    #[serde(default = "IcBasedOn::default")]
    pub ic_base: IcBasedOn,
    /// The similarity method to use.
    #[serde(default = "SimilarityMethod::default")]
    pub similarity: SimilarityMethod,
    /// The score combiner.
    #[serde(default = "ScoreCombiner::default")]
    pub combiner: ScoreCombiner,
}

/// Request as sent together with the response.
///
/// The difference is that the `lhs` and `rhs` fields are replaced by vecs.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, Default, Debug, Clone)]
#[schema(title = "HpoSimTermTermQuery")]
pub struct ResponseQuery {
    /// The one set of HPO terms to compute similarity for.
    pub lhs: Vec<String>,
    /// The second set of HPO terms to compute similarity for.
    pub rhs: Vec<String>,
    /// What should information content be based on.
    #[serde(default = "IcBasedOn::default")]
    pub ic_base: IcBasedOn,
    /// The similarity method to use.
    #[serde(default = "SimilarityMethod::default")]
    pub similarity: SimilarityMethod,
    /// The score combiner.
    #[serde(default = "ScoreCombiner::default")]
    pub combiner: ScoreCombiner,
}

/// Result container.
#[derive(serde::Serialize, serde::Deserialize, utoipa::ToSchema, Default, Debug, Clone)]
#[schema(title = "HpoSimTermTermResult")]
pub struct Result {
    /// Version information.
    pub version: Version,
    /// The original query records.
    pub query: ResponseQuery,
    /// The resulting records for the scored genes.
    pub result: Vec<ResultEntry>,
}

/// Result entry for `handle`.
#[derive(
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    Default,
    Debug,
    Clone,
    PartialEq,
    PartialOrd,
)]
#[schema(title = "HpoSimTermTermResultEntry")]
pub struct ResultEntry {
    /// The lhs entry.
    pub lhs: String,
    /// The rhs entry.
    pub rhs: String,
    /// The similarity score.
    pub score: f32,
}

/// Query for pairwise term similarity.
///
/// In the case of Resnik, this corresponds to `IC(MICA(t_1, t_2))`.
///
/// # Errors
///
/// In the case that there is an error running the server.
#[allow(clippy::unused_async)]
#[utoipa::path(
    params(RequestQuery),
    responses(
        (status = 200, description = "The query was successful.", body = Result),
    )
)]
#[get("/hpo/sim/term-term")]
async fn handle(
    data: Data<Arc<WebServerData>>,
    _path: Path<()>,
    query: web::Query<RequestQuery>,
) -> actix_web::Result<impl Responder, CustomError> {
    let ontology: &Ontology = &data.ontology;
    let mut result = Vec::new();

    let ic: Builtins = to_pairwise_sim(query.similarity, query.ic_base);

    // Translate strings from the query into HPO terms.
    let lhs = query
        .lhs
        .iter()
        .filter_map(|lhs| ontology.hpo(HpoTermId::from(lhs.clone())))
        .collect::<Vec<_>>();
    let rhs = query
        .rhs
        .iter()
        .filter_map(|rhs| ontology.hpo(HpoTermId::from(rhs.clone())))
        .collect::<Vec<_>>();

    // Compute the similarity for each pair.
    for (lhs, rhs) in lhs.iter().cartesian_product(rhs.iter()) {
        let similarity = ic.calculate(lhs, rhs);
        let elem = ResultEntry {
            lhs: lhs.id().to_string(),
            rhs: rhs.id().to_string(),
            score: similarity,
        };
        result.push(elem);
    }

    result.sort_by(|lhs, rhs| {
        rhs.score
            .partial_cmp(&lhs.score)
            .expect("could not sort by score")
    });

    // We need to convert between Request and RequestResponse here so we can serialize the
    // lhs and rhs as Vec (they must be strings to parse the GET).
    let RequestQuery {
        lhs,
        rhs,
        ic_base,
        similarity,
        combiner,
    } = query.into_inner();

    let result = Result {
        version: Version::new(&data.ontology.hpo_version()),
        query: ResponseQuery {
            lhs,
            rhs,
            ic_base,
            similarity,
            combiner,
        },
        result,
    };

    dbg!(&result);

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
    ) -> Result<super::Result, anyhow::Error> {
        let app = actix_web::test::init_service(
            actix_web::App::new()
                .app_data(actix_web::web::Data::new(web_server_data))
                .service(super::handle),
        )
        .await;
        let req = actix_web::test::TestRequest::get().uri(uri).to_request();
        let resp: super::Result = actix_web::test::call_and_read_body_json(&app, req).await;

        Ok(resp)
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_sim_term_term_one_one(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/hpo/sim/term-term?lhs=HP:0010442&rhs=HP:0001780"
            )
            .await?
        ))
    }

    #[rstest::rstest]
    #[actix_web::test]
    async fn hpo_sim_term_term_two_two(
        web_server_data: &Arc<crate::server::run::WebServerData>,
    ) -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query(
                web_server_data.clone(),
                "/hpo/sim/term-term?lhs=HP:0010442,HP:0000347&rhs=HP:0001780,HP:0000252"
            )
            .await?
        ))
    }
}
