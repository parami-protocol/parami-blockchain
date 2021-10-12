# Parami Blockchain

Dana-v3 testnet.

## Versions

```
rustc 1.55.0 (32da73ab1 2021-08-23)
rustc 1.57.0-nightly (41dfaaa3c 2021-10-10)

substrate-node-template v3.0.0+monthly-2021-10
substrate polkadot-v0.9.11
```

```bash
$ rustup toolchain install nightly-2021-09-05
$ rustup target add wasm32-unknown-unknown --toolchain nightly-2021-09-05
```

## Types

NOTE: The current `types.json` is at [parami-types.json](./parami-types.json).

## Pallets

### Airdrop

5EYCAe5ijiYdQH8WgewkjxMti9fCfnSbCuFsGtn4wkHeYGDJ

### ChainBridge

```
const MODULE_ID: ModuleId = ModuleId(*b"cb/bridg");
```

5EYCAe5g7bGpFHagwe26HiRHdHdE3hobrwV6hq1UD2BPAiZb

All cross assets should under this account.

### CrossAssets

Cross-chain assets. Current implementation only handles native AD3 token.

### Assets

Mirrored from `pallet-assets`, with fn visiblity modifications. Thus it can be used as a dependent pallet.

```text
create(id: AssetId, admin: LookupSource, min_balance: Balance)
set_metadata(id: T::AssetId, name: Vec<u8>, symbol: Vec<u8>, decimals: u8)
mint(id: T::AssetId, beneficiary: LookupSource, amount: T::Balance)
transfer(id: T::AssetId, target: LookupSource, amount: T::Balance)
```

### Swap

```text
create(asset_id: AssetId)
add_liquidity(asset_id: AssetId, native_amount: Balance, maybe_asset_amount: Option<AssetBalance>)
remove_liquidity(asset_id: AssetId, liquidity_amount: Balance)
buy(asset_id: AssetId, native_amount: Balance)
sell(asset_id: AssetId, asset_amount: AssetBalance)
```
