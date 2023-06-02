#!/bin/bash
# First argument is the path to the env file
filepath=$1
echo "set  env variables from: $1";

while IFS= read -r line
do
 key=`echo "$line" | cut -d "=" -f 1`
 value=`echo "$line" | cut -d "=" -f 2`
 export $key=$value
done < "$filepath"

cargo run --bin zksync_server --release -- --genesis
