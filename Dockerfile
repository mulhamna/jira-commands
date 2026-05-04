FROM gcr.io/oss-fuzz-base/base-builder-rust

WORKDIR $SRC/jira-commands
COPY . .
