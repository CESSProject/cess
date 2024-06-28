#! /usr/bin/env bash

the_script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
docker_build_ctx_dir=$(dirname $(dirname $the_script_dir))
docker_build_args=(--build-arg GIT_SHA=$(git rev-parse --short HEAD))
org_id="cesslab"
network="devnet"
image_id=
image_tag=
publish=0
OA=1
VC=0

function usage() {
    cat <<EOF
Easy to build CESS docker images

Usage:
    $0 [options]

Options:
    -b <program name>  which program image to build, options: node ceseal and cifrost
    -n <network profile>  options: devnet, testnet, mainnet
    -s <image tag suffix>  padding a suffix for the image tag
    -t <image tag>  specific the tag name of the image, exclusion from option -s
    -x <proxy address>  use proxy access network in build
    -o <enable 'only-attestation' feature to build>  options: 1 or 0, 1 for default
    -v <enable 'verify-cesealbin' feature to build>  options: 1 or 0, 0 for default
    -c <ceseal build version>  8-digit integer, date +%y%m%d%H for default value
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

function build_ceseal() {
    if [[ -z $IAS_API_KEY ]]; then
        echo "the IAS_API_KEY environment variable is missing"
        exit 1
    fi
    if [[ -z $IAS_SPID ]]; then
        echo "the IAS_SPID environment variable is missing"
        exit 1
    fi
    docker_build_args+=(
        --build-arg IAS_API_KEY=$IAS_API_KEY
        --build-arg IAS_SPID=$IAS_SPID
    )
    if [[ ! -z $SGX_ENV ]]; then
        docker_build_args+=(
            --build-arg SGX_ENV=$SGX_ENV
        )
    fi
    if [[ -z $CESEAL_VERSION ]]; then
        CESEAL_VERSION=$(date +%y%m%d%H)
    fi
    docker_build_args+=(
        --build-arg CESEAL_VERSION=$CESEAL_VERSION
    )
    echo "SGX_ENV: $SGX_ENV"
    echo "IAS_API_KEY: $IAS_API_KEY"
    echo "IAS_SPID: $IAS_SPID"
    echo "CESEAL_VERSION: $CESEAL_VERSION"
    local docker_file="$the_script_dir/ceseal/gramine/handover.Dockerfile"
    image_id="$org_id/ceseal:$image_tag"
    echo "begin build image $image_id ..."
    docker_build -t $image_id -f $docker_file ${docker_build_args[@]} $docker_build_ctx_dir
}

function build_cifrost() {
    local docker_file="$the_script_dir/cifrost/Dockerfile"
    image_id="cesslab/cifrost:$image_tag"
    echo "begin build image $image_id ..."
    docker_build -t $image_id -f $docker_file ${docker_build_args[@]} $docker_build_ctx_dir
}

while getopts ":hpn:b:x:t:s:o:v:c:" opt; do
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
    c)
        CESEAL_VERSION=$OPTARG
        ;;
    t)
        image_tag=$OPTARG
        ;;
    s)
        image_tag_suffix=$OPTARG
        ;;
    b)
        if [[ $OPTARG != "ceseal" && $OPTARG != "cifrost" && $OPTARG != "node" ]]; then
            echo "Invalid program name: $OPTARG, options: ceseal cifrost or node"
            exit 1
        fi
        which_build_proc=$(echo build_$OPTARG)
        ;;
    \?)
        echo "Invalid option: -$OPTARG" 1>&2
        exit 1
        ;;
    esac
done

if [[ -z $which_build_proc ]]; then
    echo "The program name option -b must be specific!"
    exit 1
fi

if ! [[ $network = "devnet" || $network = "testnet" || $network = "mainnet" ]]; then
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
)

eval $which_build_proc
if [ $? -ne 0 ]; then
    echo "$image_id build failed!"
    exit 1
fi
echo "$image_id build success"

if [[ $publish -eq 1 ]]; then
    echo "will publish $image_id image"
    docker push $image_id
fi
