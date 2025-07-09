## What
This script is used to quickly configure and launch validator nodes.

## How to use
### Prerequisites
1. Docker 22+
2. Python 3.10+
3. uv 0.7+ (install: `curl -LsSf https://astral.sh/uv/install.sh | sh`)

### Show usage
`uv run ces-launch.py --help`

### Steps
1. Prepare the node-key and wallet private key (i.e., mnemonic) required for the validator nodes based on the `example.env` file, and rename it to `.env`.
2. Generate `docker-compose.yml`: `uv run ces-launch.py gen`
3. Insert related keys: `uv run ces-launch.py key-insert`
4. Launch validator nodes: `uv run ces-launch.py run`