#! /usr/bin/env bash

the_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
docker_build_ctx_dir=$(dirname $(dirname $the_script_dir))
docker_build_args=(--build-arg GIT_SHA=$(git rev-parse --short HEAD))
docker_build_log=0
org_id="cesslab"
network="devnet"
image_id=
image_tag=
publish=0
OA=1
VC=1

function usage() {
    cat <<EOF
Easy to build CESS docker images

Usage:
    $0 [options]

Options:
    -b <program name>  which program image to build, default: node
    -n <network profile>  options: devnet, testnet, testnet2, mainnet
    -s <image tag suffix>  padding a suffix for the image tag
    -t <image tag>  specific the tag name of the image, exclusion from option -s
    -x <proxy address>  use proxy access network in build
    -o <enable 'only-attestation' feature to build>  options: 1(default) or 0
    -v <enable 'verify-cesealbin' feature to build>  options: 1(default) or 0
    -l <docker build runtime log print out> options: 1 or 0(default)
    -p  publish image to docker hub
    -h  display this help message.
EOF
    exit 1
}

function docker_build() {
    #echo "docker build $@"
    docker build $@
}

function build_node() {
    local docker_file="$the_script_dir/node/Dockerfile"
    image_id="$org_id/cess-chain:$image_tag"
    echo "begin build image $image_id ..."
    docker_build -t $image_id -f $docker_file ${docker_build_args[@]} $docker_build_ctx_dir
}

function print_docker_build_log() {
    if [ $docker_build_log = "1" ]; then
        echo "Print out the detail log of docker image build" >&2
        echo "--progress=plain"
    elif [ "$docker_build_log" -eq 0 ]; then
        echo "No print out the detail log of docker image build" >&2
    else
        echo "wrong parameter print in '-l',use default value" >&2
    fi
}

while getopts ":hpn:b:x:t:s:o:v:l:" opt; do
    case ${opt} in
    h)
        usage
        ;;
    p)
        publish=1
        ;;
    n)
        network=$OPTARG
        ;;
    x)
        docker_build_args+=(
            --build-arg http_proxy=$OPTARG
            --build-arg https_proxy=$OPTARG
        )
        ;;
    o)
        if [[ $OPTARG -eq 1 ]]; then
            OA=1
        else
            OA=0
        fi
        ;;
    v)
        if [[ $OPTARG -eq 1 ]]; then
            VC=1
        else
            VC=0
        fi
        ;;    
    l)
        docker_build_log=$OPTARG
        ;;
    t)
        image_tag=$OPTARG
        ;;
    s)
        image_tag_suffix=$OPTARG
        ;;    
    \?)
        echo "Invalid option: -$OPTARG" 1>&2
        exit 1
        ;;
    esac
done


if ! [[ $network = "devnet" || $network = "testnet" || $network = "mainnet" || $network = "testnet2" ]]; then
    echo "Invalid network option, use 'devnet' instead"
    network="devnet"
fi

if [[ -z $image_tag ]]; then
    image_tag=$network
    if [[ ! -z $image_tag_suffix ]]; then
        image_tag="$image_tag-$image_tag_suffix"
    fi
fi

docker_build_args+=(
    --build-arg OA=$OA
    --build-arg VC=$VC
    --build-arg CHAIN_NETWORK=$network
)

build_node
if [ $? -ne 0 ]; then
    echo "$image_id build failed!"
    exit 1
fi
echo "$image_id build success"

if [[ $publish -eq 1 ]]; then
    echo "will publish $image_id image"
    docker push $image_id
fi
