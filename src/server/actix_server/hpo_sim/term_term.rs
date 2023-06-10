//! Entry point `/hpo/sim/term-term` allows the pairwise similary computation between two sets
//! of HPO terms.

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

use crate::common::{to_pairwise_sim, IcBasedOn, ScoreCombiner, SimilarityMethod};
use crate::server::{actix_server::CustomError, WebServerData};

/// Parameters for `handle`.
///
/// This allows to compute differences between
///
/// - `lhs` -- first set of terms to compute similarity for
/// - `rhs` -- econd set of terms to compute similarity for
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Request {
    /// The one set of HPO terms to compute similarity for.
    #[serde(deserialize_with = "super::super::vec_str_deserialize")]
    pub lhs: Vec<String>,
    /// The second set of HPO terms to compute similarity for.
    #[serde(deserialize_with = "super::super::vec_str_deserialize")]
    pub rhs: Vec<String>,
    /// What should information content be based on.
    pub ic_base: IcBasedOn,
    /// The similarity method to use.
    pub similarity: SimilarityMethod,
    /// The score combiner.
    pub combiner: ScoreCombiner,
}

/// Result container.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct Container {
    /// Version of the HPO.
    pub hpo_version: String,
    /// Version of the `viguno` package.
    pub viguno_version: String,
    /// The original query records.
    pub query: Request,
    /// The resulting records for the scored genes.
    pub result: Vec<ResultEntry>,
}

/// Result entry for `handle`.
#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, PartialOrd)]
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
#[get("/hpo/sim/term-term")]
async fn handle(
    data: Data<WebServerData>,
    _path: Path<()>,
    query: web::Query<Request>,
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

    Ok(Json(result))
}

#[cfg(test)]
mod test {
    /// Helper function for running a query.
    #[allow(dead_code)]
    async fn run_query(uri: &str) -> Result<Vec<super::ResultEntry>, anyhow::Error> {
        let hpo_path = "tests/data/hpo";
        let ontology = crate::common::load_hpo("tests/data/hpo")?;
        let db = Some(rocksdb::DB::open_cf_for_read_only(
            &rocksdb::Options::default(),
            format!("{}/{}", hpo_path, "resnik"),
            ["meta", "scores"],
            true,
        )?);

        let app = actix_web::test::init_service(
            actix_web::App::new()
                .app_data(actix_web::web::Data::new(crate::server::WebServerData {
                    ontology,
                    db,
                }))
                .service(super::handle),
        )
        .await;
        let req = actix_web::test::TestRequest::get().uri(uri).to_request();
        let resp: Vec<super::ResultEntry> =
            actix_web::test::call_and_read_body_json(&app, req).await;

        Ok(resp)
    }

    #[actix_web::test]
    async fn hpo_sim_term_term_one_one() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/sim/term-term?lhs=HP:0010442&rhs=HP:0001780").await?
        ))
    }

    #[actix_web::test]
    async fn hpo_sim_term_term_two_two() -> Result<(), anyhow::Error> {
        Ok(insta::assert_yaml_snapshot!(
            &run_query("/hpo/sim/term-term?lhs=HP:0010442,HP:0000347&rhs=HP:0001780,HP:0000252")
                .await?
        ))
    }
}
