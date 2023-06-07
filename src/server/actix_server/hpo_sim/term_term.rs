//! Entry point `/hpo/sim/term-term` allows the pairwise similary computation between two sets
//! of HPO terms.

use std::str::FromStr;

use actix_web::{
    get,
    web::{self, Data, Json, Path},
    Responder,
};
use hpo::{
    similarity::{Builtins, Similarity},
    term::InformationContentKind,
    HpoTermId, Ontology,
};
use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::server::{actix_server::CustomError, WebServerData};

/// Enum for representing similarity method to use.
#[derive(Default, Debug, Clone, Copy, derive_more::Display)]
pub enum SimilarityMethod {
    /// Resnik similarity with gene-wise information content.
    #[default]
    #[display(fmt = "resnik::gene")]
    ResnikGene,
}

impl FromStr for SimilarityMethod {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "resnik::gene" => Self::ResnikGene,
            _ => anyhow::bail!("unknown similarity method: {}", s),
        })
    }
}

impl From<SimilarityMethod> for Builtins {
    fn from(val: SimilarityMethod) -> Self {
        match val {
            SimilarityMethod::ResnikGene => Builtins::Resnik(InformationContentKind::Gene),
        }
    }
}

/// Parameters for `handle`.
///
/// This allows to compute differences between
///
/// - `lhs` -- first set of terms to compute similarity for
/// - `rhs` -- econd set of terms to compute similarity for
#[derive(Deserialize, Debug, Clone)]
struct Request {
    /// The one set of HPO terms to compute similarity for.
    #[serde(deserialize_with = "super::super::vec_str_deserialize")]
    pub lhs: Vec<String>,
    /// The second set of HPO terms to compute similarity for.
    #[serde(deserialize_with = "super::super::vec_str_deserialize")]
    pub rhs: Vec<String>,
    /// The similarity method to use.
    #[serde(
        deserialize_with = "help::similarity_deserialize",
        default = "help::default_sim"
    )]
    pub sim: SimilarityMethod,
}

/// Helpers for deserializing `Request`.
mod help {
    /// Helper to deserialize a similarity
    pub fn similarity_deserialize<'de, D>(
        deserializer: D,
    ) -> Result<super::SimilarityMethod, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = <String as serde::Deserialize>::deserialize(deserializer)?;
        std::str::FromStr::from_str(&s).map_err(serde::de::Error::custom)
    }

    /// Default value for `Request::sim`.
    pub fn default_sim() -> super::SimilarityMethod {
        super::SimilarityMethod::ResnikGene
    }
}

/// Result entry for `handle`.
#[derive(Serialize, Debug, Clone)]
struct ResultEntry {
    /// The lhs entry.
    pub lhs: String,
    /// The rhs entry.
    pub rhs: String,
    /// The similarity score.
    pub score: f32,
    /// The score type that was used to compute the similarity for.
    pub sim: String,
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

    let ic: Builtins = query.sim.into();

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
            sim: query.sim.to_string(),
        };
        result.push(elem);
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
