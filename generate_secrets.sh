#!/bin/bash

if [ ! -s $1 ]; then
  /usr/bin/zksync_external_node generate-secrets | grep -E "^(validator_key|node_key|#)" > $1
fi