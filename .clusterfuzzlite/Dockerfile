FROM gcr.io/oss-fuzz-base/base-builder-rust@sha256:119c23ff674f7b9680d019601e03bb4810d08a0e35f57abee883bf90583ac759

COPY . $SRC/jira-commands
WORKDIR $SRC/jira-commands
COPY ./.clusterfuzzlite/build.sh $SRC/
