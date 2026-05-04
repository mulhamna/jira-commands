FROM gcr.io/oss-fuzz-base/base-builder-rust

COPY . $SRC/jira-commands
WORKDIR $SRC/jira-commands
COPY ./.clusterfuzzlite/build.sh $SRC/
