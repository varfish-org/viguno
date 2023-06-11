[![Crates.io](https://img.shields.io/crates/d/viguno.svg)](https://crates.io/crates/viguno)
[![Crates.io](https://img.shields.io/crates/v/viguno.svg)](https://crates.io/crates/viguno)
[![Crates.io](https://img.shields.io/crates/l/viguno.svg)](https://crates.io/crates/viguno)
[![CI](https://github.com/bihealth/viguno/actions/workflows/rust.yml/badge.svg)](https://github.com/bihealth/viguno/actions/workflows/rust.yml)
[![codecov](https://codecov.io/gh/bihealth/viguno/branch/main/graph/badge.svg?token=aZchhLWdzt)](https://codecov.io/gh/bihealth/viguno)

# Viguno

<img src="https://raw.githubusercontent.com/bihealth/viguno/main/utils/vicuna-wrangling-ontology-and-genes.png" width="256px" height="256px" align="right">

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
    -it ghcr.io/bihealth/viguno:0.1.1
```

## Preparing Data

The initial step is to download the official HPO data to a local directory.
We fix ourselves to the release from 2023-06-06.

```
# RELEASE=2023-06-06
# URL=https://github.com/obophenotype/human-phenotype-ontology/releases/download
# NAMES="hp.obo phenotype.hpoa phenotype_to_genes.txt genes_to_phenotype.txt"

# mkdir -p /tmp/data/hpo
# for name in $NAMES; do \
    wget \
        -O /tmp/data/hpo/$name \
        $URL/v$RELEASE/$name;
  done
```

You can now conver the downloaded text HPO files to a binary format which will improve performance of loading data.

```
# viguno convert \
    --path-hpo-dir /tmp/data/hpo \
    --path-out-bin /tmp/data/hpo/hpo.bin
```

To use the similarity computations, you will need to run some simulation as precomputation.
You should fix the seed for reproducibility.
The number of simulations should be high for production (the default is 100k) but you can reduce this for a local setup.

```
# viguno simulate \
    --num-simulations 10 \
    --seed 42 \
    --path-hpo-dir /tmp/data/hpo/hpo \
    --path-out-rocksdb /tmp/data/hpo/hpo/scores-fun-sim-avg-resnik-gene \
    --combiner fun-sim-avg \
    --similarity resnik \
    --ic-base gene
```

## Running the Server

After having the precomputed data, you can startup the server as follows:

```
# viguno run-server \
    --path-hpo-dir tests/data/hpo
INFO args_common = Args { verbose: Verbosity { verbose: 0, quiet: 0, phantom: PhantomData<clap_verbosity_flag::InfoLevel> } }
INFO args = Args { path_hpo_dir: "tests/data/hpo", suppress_hints: false, listen_host: "127.0.0.1", listen_port: 8080 }
INFO Loading HPO...
INFO ...done loading HPO in 8.180012599s
INFO Opening RocksDB for reading...
INFO ...done opening RocksDB in 19.027133ms
INFO Launching server main on http://127.0.0.1:8080 ...
INFO   try: http://127.0.0.1:8080/hpo/genes?gene_symbol=TGDS
INFO   try: http://127.0.0.1:8080/hpo/genes?gene_id=23483&hpo_terms=true
INFO   try: http://127.0.0.1:8080/hpo/omims?omim_id=616145&hpo_terms=true
INFO   try: http://127.0.0.1:8080/hpo/terms?term_id=HP:0000023&genes=true
INFO   try: http://127.0.0.1:8080/hpo/sim/term-term?lhs=HP:0001166,HP:0040069&rhs=HP:0005918,HP:0004188
INFO   try: http://127.0.0.1:8080/hpo/sim/term-gene?terms=HP:0001166,HP:0000098&gene_symbols=FBN1,TGDS,TTN
INFO starting 4 workers
INFO Actix runtime found; starting in Actix runtime
```

Now the server is running and you could stop it with `Ctrl-C`.

In another terminal, you then now do as suggested above.
Note that we truncate the output JSON.

```
# curl 'http://127.0.0.1:8080/hpo/genes?gene_symbol=TGDS'
[{"gene_ncbi_id":23483,"gene_symbol":"TGDS"}]

# curl 'http://127.0.0.1:8080/hpo/genes?gene_id=23483&hpo_terms=true'
[{"gene_ncbi_id":23483,"gene_symbol":"TGDS","hpo_terms":[{"term_...

# curl 'http://127.0.0.1:8080/hpo/omims?omim_id=616145&hpo_terms=true'
[{"omim_id":"OMIM:616145","name":"Catel-Manzke syndrome","hpo_te...

# curl 'http://127.0.0.1:8080/hpo/terms?term_id=HP:0000023&genes=true'
[{"term_id":"HP:0000023","name":"Inguinal hernia","genes":[{"gen...

# curl 'http://127.0.0.1:8080/hpo/sim/term-term?lhs=HP:0001166,HP:0040069&rhs=HP:0005918,HP:0004188'
[{"lhs":"HP:0001166","rhs":"HP:0005918","score":1.4280319,"sim":...
```

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

## Managing Project with Terraform

```
# export GITHUB_OWNER=bihealth
# export GITHUB_TOKEN=ghp_TOKEN

# cd utils/terraform
# terraform init
# terraform import github_repository.viguno viguno
# terraform validate
# terraform fmt
# terraform plan
# terraform apply
```
