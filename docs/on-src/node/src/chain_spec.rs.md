The relationship of how the functions related to each others, in diagram.

Refer to [mermaid doc: Flowchart](https://mermaid.js.org/syntax/flowchart.html) for syntax.

`id` below is the `id` parameter used in **Cli** [`load_spec()`](https://github.com/CESSProject/cess/blob/main/node/src/command.rs#L57-L69)

```mermaid
flowchart LR
  %% main
  testnet_genesis("testnet_genesis()") --> main_genesis("cess_main_genesis()") --> main("cess_main()") --> initial_testnet(["id: `cess-initial-testnet`"])

  %% testnet
  testnet_genesis --> testnet_config_genesis("cess_testnet_config_genesis()") --> testnet_generate_config("cess_testnet_generate_config()") --> initial_devnet(["id: `cess-initial-devnet`"])

  %% local
  testnet_genesis --> local_testnet_genesis("local_testnet_genesis()") --> local_testnet_config("local_testnet_config()") --> local(["id: `local`"])

  %% dev
  testnet_genesis --> development_config_genesis("development_config_genesis()") --> development_config("development_config()") --> dev(["id: `dev`"])

  %% Reading from chainspec file
  chainSpec("Read from chainSpec") --> testnet_config("cess_testnet_config()") --> cess_testnet(["id: `cess-testnet`"])
  chainSpec --> develop_config("cess_develop_config()") --> cess_devnet(["id: `cess-devnet`"])
```
