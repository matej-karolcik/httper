#!/usr/bin/env bash

set -e

go build

cd echo && go run main.go &
sleep 3

job_num=$(jobs | tail -n 1 | sed -n 's/^\[\([0-9]*\)\].*/\1/p')
echo "Job number: $job_num"

cleanup() {
  echo "Cleaning up job: $job_num"
  kill "%$job_num"
}

trap cleanup EXIT
trap cleanup SIGINT
trap cleanup SIGTERM

for file in "testdata"/*.http; do
  if [ -f "$file" ]; then
    echo "Running $file"
    ./httper $file
  fi
done
