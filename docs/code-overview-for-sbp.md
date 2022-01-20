# Project Details
- Version：**v0.1.3**
- Substrate version：**monthly-2021-10**
- In the continuous development, the number of blocks is close to **30w**, and the online upgrade is **3** times
- With testable block explorers:（http://data.cesslab.co.uk/browser/#/explorer）
- **Passed W3F Grants Milestone 1 and Milestone 2, and Milestone 3 has also been submitted**.

# Hackathon Details
The data trading market developed during the hackathon includes several modules related to Substrate:
- The CESS Cloud Data Network (cess-node): is developed based on *the Substrate*.
- The FMD-CESS (web-app): An web platform using  *polkadot-js-app*.
- Cumulus Encrypted Storage System: consists of storage-mining-tool and scheduler-mining-tool, which using *Go Substrate RPC Client Library (GSRPC)*.
- Blockchain explorer (cess-ui-js): is tailored for the CESS Cloud Data Network, modified from *polkadot-js-app*.
- See more: https://github.com/CESSProject/fmd-cess#%EF%B8%8F-what-it-does

# Overall Architecture
The project is based on Substrate FRAME, and the following modules are developed on STORAGE (Wasm Runtime) according to functional requirements. The blue part is the official module (Pallets), and the orange part is the custom module (Pallets).

# ![CESS](https://raw.githubusercontent.com/Cumulus2021/W3F-illustration/main/CESS-TestNet.png)

Next, introduce our own developed pallets:
https://github.com/CESSProject/cess#module-documentation
