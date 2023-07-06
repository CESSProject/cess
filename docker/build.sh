#! /usr/bin/env bash
# cess runner builder

usage() {
    echo "Usage:"
	echo "    $0 -h                      Display this help message."
	echo "    $0 [options]"
    echo "Options:"
    echo "     -p publish image"
    echo "     -t image tag, options: devnet, testnet, mainnet, latest"
	exit 1;
}

PUBLISH=0
IMG_TAG="testnet"

while getopts ":hpt:" opt; do
    case ${opt} in
        h )
			usage
            ;;
        p )
            PUBLISH=1
            ;;
        t )
            IMG_TAG=$OPTARG

            ;;
        \? )
            echo "Invalid Option: -$OPTARG" 1>&2
            exit 1
            ;;
    esac
done

SH_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null 2>&1 && pwd )"
BUILD_DIR=$(dirname $SH_DIR)
DIST_FILE="$BUILD_DIR/target/release/cess-node"
IMAGEID="cesslab/cess-chain"

source $SH_DIR/utils.sh

if [ x"$IMG_TAG" == x"devnet" ] || [ x"$IMG_TAG" == x"testnet" ] || [ x"$IMG_TAG" == x"mainnet" ] || [ x"$IMG_TAG" == x"latest" ]; then
    IMAGEID="$IMAGEID:$IMG_TAG"
else
    echo "invalid image tag option, use 'devnet' instead"
    IMAGEID="$IMAGEID:devnet"
fi

if [ ! -f "$DIST_FILE" ]; then
    log_err "Binary from $DIST_FILE doesn't exist, please build cess binary first."
    exit 1
fi

log_info "Building cess-chain image, version: ${CESS_NODE_VER}, bin file $DIST_FILE"

build_dir=$SH_DIR/.tmp
mkdir -p $build_dir
cp -f $DIST_FILE $build_dir
ls -lh $build_dir

docker build $build_dir -t $IMAGEID -f $SH_DIR/Dockerfile --build-arg http_proxy=$http_proxy --build-arg https_proxy=$https_proxy

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
