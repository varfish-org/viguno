#!/usr/bin/bash

set -x
set -euo pipefail

# Interpreted environment variables.
#
#   PATH_HPO_DIR    -- path to the directory with HPO files
#                      default: /data/hpo
#   PATH_HGNC_XLINK -- path to the TSV xlink file
#                      default: /data/hgnc_xlink.tsv
#   HTTP_HOST       -- host to listen on
#                      default: 0.0.0.0
#   HTTP_PORT       -- port
#                      default: 8080

PATH_HPO_DIR=${PATH_HPO_DIR-/data/hpo}
PATH_HGNC_XLINK=${PATH_HGNC_XLINK-/data/hgnc_xlink.tsv}
HTTP_HOST=${HTTP_HOST-0.0.0.0}
HTTP_PORT=${HTTP_PORT-8080}

first=${1-}

if [ "$first" == exec ]; then
  shift
  exec "$@"
else
  exec \
    viguno server run \
      --path-hpo-dir "$PATH_HPO_DIR" \
      --path-hgnc-xlink "$PATH_HGNC_XLINK" \
      --listen-host "$HTTP_HOST" \
      --listen-port "$HTTP_PORT"
fi

exit $?
