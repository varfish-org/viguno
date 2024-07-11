[![Crates.io](https://img.shields.io/crates/d/viguno.svg)](https://crates.io/crates/viguno)
[![Crates.io](https://img.shields.io/crates/v/viguno.svg)](https://crates.io/crates/viguno)
[![Crates.io](https://img.shields.io/crates/l/viguno.svg)](https://crates.io/crates/viguno)
[![CI](https://github.com/varfish-org/viguno/actions/workflows/rust.yml/badge.svg)](https://github.com/varfish-org/viguno/actions/workflows/rust.yml)
[![codecov](https://codecov.io/gh/varfish-org/viguno/branch/main/graph/badge.svg?token=aZchhLWdzt)](https://codecov.io/gh/varfish-org/viguno)

# Viguno

<img src="https://raw.githubusercontent.com/varfish-org/viguno/main/utils/vicuna-wrangling-ontology-and-genes.png" width="256px" height="256px" align="right">

Viguno (Versatile Interface for Genetics Utilization of Nice Ontologies) is the component of a VarFish installation that provides information on HPO phenotypes, diseases, and their relation to genes.

Viguno provides a REST API.

## Running the Docker Image

The commands used below assume that you have a `viguno` executable on your machine (e.g., via `cargo install`).
However, we also provide Docker images.
You can run version `v0.1.1` as follows.
We assume that the current working directory is a check of `viguno` so we have a copy of the HPO and HGNC xlink table in `tests/data`.
We bind-mount this into the container with `-v $PWD/tests/data:/data`.

```
# docker run \
    -v $PWD/tests/data:/data \
    -it ghcr.io/varfish-org/viguno:0.1.1
```

## Preparing Data

The initial step is to download the official HPO data to a local directory.
We fix ourselves to the release from 2023-06-06.

```
# RELEASE=2023-06-06
# URL=https://github.com/obophenotype/human-phenotype-ontology/releases/download
# NAMES="hp-base.obo phenotype.hpoa phenotype_to_genes.txt genes_to_phenotype.txt"

# mkdir -p /tmp/data/hpo
# for name in $NAMES; do \
    wget \
        -O /tmp/data/hpo/$name \
        $URL/v$RELEASE/$name;
  done
# mv /tmp/data/hpo/hp-base.obo /tmp/data/hpo/hp.obo
# sed -i -e 's|/hp-base.owl||' /tmp/data/hpo/hp.obo
```

Next, generate the cross-link file between different gene identifiers.

```
# wget -O /tmp/hgnc_complete_set.json \
    https://ftp.ebi.ac.uk/pub/databases/genenames/hgnc/json/hgnc_complete_set.json
# echo -e "hgnc_id\tensembl_gene_id\tentrez_id\tgene_symbol" \
    > /tmp/data/hpo/hgnc_xlink.tsv
# jq -r '.response.docs[] | select(.entrez_id != null) | [.hgnc_id, .ensembl_gene_id, .entrez_id, .symbol] | @tsv' \
    /tmp/hgnc_complete_set.json \
  | LC_ALL=C sort -t $'\t' -k3,3n \
  >> /tmp/data/hpo/hgnc_xlink.tsv
```

You can now conver the downloaded text HPO files to a binary format which will improve performance of loading data.

```
# viguno convert \
    --path-hpo-dir /tmp/data/hpo \
    --path-out-bin /tmp/data/hpo/hpo.bin
```

## Running the Server

After having the precomputed data, you can startup the server as follows:

```
# viguno server run \
    --path-hpo-dir tests/data/hpo
INFO args_common = Args { verbose: Verbosity { verbose: 0, quiet: 0, phantom: PhantomData<clap_verbosity_flag::InfoLevel> } }
INFO args = Args { path_hpo_dir: "tests/data/hpo", suppress_hints: false, listen_host: "127.0.0.1", listen_port: 8080 }
INFO Loading HPO...
INFO   attempting to load binary HPO file from tests/data/hpo
INFO ...done loading HPO in 4.788750172s
INFO Loading HGNC xlink...
INFO ... done loading HGNC xlink in 156.362034ms
INFO Loading HPO OBO...
INFO ... done loading HPO OBO in 1.90213703s
INFO Indexing OBO...
INFO ... done indexing OBO in 835.558794ms
INFO Launching server main on http://127.0.0.1:8080 ...
INFO   SEE SWAGGER UI FOR INTERACTIVE DOCS: http://127.0.0.1:8080/swagger-ui/
INFO starting 8 workers
INFO Actix runtime found; starting in Actix runtime
```

Now the server is running and you could stop it with `Ctrl-C`.

You can go to http://127.0.0.1/swagger-ui to see the automatically generated interactive API documentation.
You can find the OpenAPI YAML file for the `main` branch [here on GitHub](https://raw.githubusercontent.com/varfish-org/viguno/main/openapi.yaml) and e.g., open it [here in the public Swagger editor](https://editor.swagger.io?url=https://raw.githubusercontent.com/varfish-org/viguno/main/openapi.yaml).

# Developer Documentation

The following is for developers of Viguno itself.

## Creating Docker Builds

We automatically build Docker images using GitHub actions.
To do this locally, you can do:

```
# bash utils/docker/build-docker.sh
```

You can set the following environment variables

- `GIT_TAG` a git "treeish" (can be a tag or version) to build (inferred via `git describe --tags` by default)
- `DOCKER_VERSION` the version to use for the Docker label (by default it uses `$GIT_TAG` and the `v` prefix will be stripped)
