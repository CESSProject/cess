#!/usr/bin/env python3
# /// script
# requires-python = ">=3.10"
# dependencies = [
#     "dotenv",
#     "pyyaml",
# ]
# ///
import argparse
import os
import sys
import subprocess
import yaml
from dotenv import load_dotenv


def gen_compose(args):
    # 1. Parse arguments
    chain = args.chain
    inst = args.inst
    data_dir = os.path.abspath(args.data_dir)
    p2p_port = args.p2p_port
    rpc_port = args.rpc_port

    # 3. Generate docker-compose.yml
    services = {}
    for i in range(1, inst + 1):
        container_name = f"{chain}-n{i}"
        service = {
            "image": f"cesslab/cess-chain:{chain}",
            "network_mode": "host",
            "volumes": [f"{data_dir}/n{i}:/opt/cess/data"],
            "command": [
                "--base-path",
                "/opt/cess/data",
                "--chain",
                f"{chain}",
                "--port",
                str(p2p_port + i - 1),
                "--name",
                f"{chain.upper()}-N{i}",
                "--validator",
                "--rpc-port",
                str(rpc_port + i - 1),
                "--pruning",
                "archive",
                "--node-key",
                f"${{N{i}_NODE_KEY}}",
                "--no-telemetry",
                "--no-prometheus",
                "--no-hardware-benchmarks",
            ],
            "logging": {
                "driver": "json-file",
                "options": {"max-size": "300m", "max-file": "10"},
            },
            "container_name": container_name,
            "environment": ["RUST_LOG=info", "RUST_BACKTRACE=full"],
        }
        # Add special parameters to the second instance
        if i == 2:
            service["command"] += [
                "--wasm-execution",
                "compiled",
                "--rpc-methods",
                "unsafe",
                "--rpc-external",
                "--rpc-cors",
                "all",
            ]
        services[container_name] = service

    compose = {"services": services}
    with open("docker-compose.yml", "w") as f:
        yaml.dump(compose, f, default_flow_style=False, sort_keys=False)

    print("Generated docker-compose.yml")


def get_chain_from_command(command_list):
    try:
        idx = command_list.index("--chain")
        return command_list[idx + 1]
    except (ValueError, IndexError):
        return None


def key_insert(args):
    compose_file = "docker-compose.yml"
    # 1. Create container instances
    # fmt: off
    subprocess.run(
        ["docker", "compose", "-f", compose_file, "--env-file", ".env", "create"],
        check=True,
    )
    # fmt: on
    load_dotenv()
    # 2. Read the compose file
    with open(compose_file) as f:
        compose = yaml.safe_load(f)
    for svc_name in compose["services"]:
        idx = svc_name.split("-n")[-1]
        chain = get_chain_from_command(compose["services"][svc_name]["command"])
        mnemonic = os.environ.get(f"N{idx}_MNEMONIC")
        shell_cmd = f"""
    docker compose -f {compose_file} run --rm {svc_name} key insert --base-path /opt/cess/data --chain {chain} --scheme Sr25519 --key-type babe --suri "{mnemonic}" && \
    docker compose -f {compose_file} run --rm {svc_name} key insert --base-path /opt/cess/data --chain {chain} --scheme Ed25519 --key-type gran --suri "{mnemonic}"
    """
        subprocess.run(shell_cmd, shell=True, check=True)
    print("Keys inserted.")


def run(args):
    # 1. Check if docker-compose.yml exists
    if not os.path.exists("docker-compose.yml"):
        print("docker-compose.yml not found.")
        sys.exit(1)
    # 2. Check docker compose config
    try:
        subprocess.run(
            ["docker", "compose", "config"],
            check=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )
    except subprocess.CalledProcessError:
        print("docker compose config failed.")
        sys.exit(1)
    # 3. Check the keystore directory
    with open("docker-compose.yml") as f:
        compose = yaml.safe_load(f)
    for svc_name in compose["services"]:
        chain = get_chain_from_command(compose["services"][svc_name]["command"])
        # Check if the keystore directory exists inside the container
        # fmt: off
        check_cmd = [
            "docker", "compose", "run", "--rm",
            "--entrypoint", "sh", svc_name,
            "-c", f"test -e /opt/cess/data/chains/cess-{chain}/keystore"
        ]
        # fmt: on
        result = subprocess.run(check_cmd)
        if result.returncode != 0:
            print(
                f"[{svc_name}] keystore directory does not exist, please run the key-insert command first."
            )
            sys.exit(1)
    # 4. Start the network
    subprocess.run(["docker", "compose", "start"], check=True)
    print("CESS Validator started.")


def main():
    parser = argparse.ArgumentParser(description="CESS Chain Validator Launch Tool")
    subparsers = parser.add_subparsers(dest="cmd", required=True)

    # gen
    p_gen = subparsers.add_parser("gen", help="Generate docker-compose.yml")
    p_gen.add_argument(
        "--chain",
        choices=["premainnet", "testnet", "devnet"],
        default="devnet",
        help="CESS chain-specification, default: devnet",
    )
    p_gen.add_argument(
        "--inst", type=int, default=2, help="validator instance counts, default: 2"
    )
    p_gen.add_argument(
        "--data-dir", default="./data", help="data directory, default: ./data"
    )
    p_gen.add_argument(
        "--p2p-port", type=int, default=30333, help="p2p port, default: 30333"
    )
    p_gen.add_argument(
        "--rpc-port", type=int, default=9944, help="rpc port, default: 9944"
    )
    p_gen.set_defaults(func=gen_compose)

    # key-insert
    p_key = subparsers.add_parser(
        "key-insert", help="Insert related keys into validator nodes"
    )
    p_key.set_defaults(func=key_insert)

    # run
    p_run = subparsers.add_parser("run", help="Run CESS Chain validator nodes")
    p_run.set_defaults(func=run)

    args = parser.parse_args()
    args.func(args)


if __name__ == "__main__":
    main()
