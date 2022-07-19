#! /usr/bin/env bash
# cess runner builder

usage() {
    echo "Usage:"
	echo "    $0 -h                      Display this help message."
	echo "    $0 [options]"
    echo "Options:"
    echo "     -p publish image"
	exit 1;
}

PUBLISH=0

while getopts ":hp" opt; do
    case ${opt} in
        h )
			usage
            ;;
        p )
            PUBLISH=1
            ;;
        \? )
            echo "Invalid Option: -$OPTARG" 1>&2
            exit 1
            ;;
    esac
done

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
source $DIR/utils.sh

BUILD_DIR="`pwd`"
DIST_FILE="target/release/cess-node"
IMAGEID="cesslab/cess-chain:latest"

if [ ! -f "$DIST_FILE" ]; then
    log_err "Binary from $DIST_FILE doesn't exist, please build cess binary first."
    exit 1
fi

log_info "Building cess-chain image, version: ${CESS_NODE_VER}, bin file $DIST_FILE"

build_dir=$DIR/.tmp
mkdir -p $build_dir
cp -f $DIST_FILE $build_dir
ls -lh $build_dir

docker build $build_dir -t $IMAGEID -f $DIR/Dockerfile

rm -rf $build_dir

if [ $? -eq "0" ]; then
    log_info "Done building cess image, tag: $IMAGEID"
else
    log_err "Failed on building cess."
    exit 1
fi

log_info "Build success"
if [ "$PUBLISH" -eq "1" ]; then
    echo "Publishing image to $IMAGEID"
    docker push $IMAGEID
fi
