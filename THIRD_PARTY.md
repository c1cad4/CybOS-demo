# cybOS third-party integrations

## rqbit / librqbit

cybOS embeds the `librqbit` BitTorrent engine as the CybLex P2P transport.

- Source: https://github.com/ikatson/rqbit
- Crate: `librqbit` 9.0.1
- License: Apache-2.0
- Integration: native Rust library, isolated behind the CybLex worker

The rqbit license text is available from the upstream repository:
https://github.com/ikatson/rqbit/blob/main/LICENSE

CybLex is intended for files the user owns or is authorized to distribute.

## PumpFun-style Solana program reference

cybOS registers the following MIT-licensed public reference implementation as a
read-only program adapter:

- Source: https://github.com/0xAllan123/pumpfun-smart-contract
- Program: `pump`
- Program ID: `7wUQXRQtBzTmyp9kcrmok9FKcc4RSYXxPYN9FGDLnqxb`
- Cluster: devnet
- License: MIT

The current cybOS adapter does not sign transactions, launch tokens, execute
swaps or deploy the program. It only exposes program identity/provenance to the
Assets/CybDex layer.

## cybLaunch

- Source: https://github.com/c1cad4/cybLaunch
- Current upstream state: external integration boundary
- cybOS does not copy or fabricate launcher functionality while the upstream
  repository has no implementation files.
