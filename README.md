[![Crates.io](https://img.shields.io/crates/d/viguno.svg)](https://crates.io/crates/viguno)
[![Crates.io](https://img.shields.io/crates/v/viguno.svg)](https://crates.io/crates/viguno)
[![Crates.io](https://img.shields.io/crates/l/viguno.svg)](https://crates.io/crates/viguno)
[![CI](https://github.com/bihealth/viguno/actions/workflows/rust.yml/badge.svg)](https://github.com/bihealth/viguno/actions/workflows/rust.yml)
[![codecov](https://codecov.io/gh/bihealth/viguno/branch/main/graph/badge.svg?token=aZchhLWdzt)](https://codecov.io/gh/bihealth/viguno)

# Viguno

<img src="https://raw.githubusercontent.com/bihealth/viguno/main/utils/vicuna-wrangling-ontology-and-genes.png" width="256px" height="256px" align="right">

Viguno (Versatile Interface for Genetics Utilization of Nice Ontologies) is the component of a VarFish installation that provides information on HPO phenotypes, diseases, and their relation to genes.

Viguno provides a REST API.

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
