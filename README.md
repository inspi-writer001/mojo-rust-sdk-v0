# Mojo Rust SDK

A Rust SDK for building on-chain games and applications on Solana. It wraps the [accel-Mojo program](https://github.com/Turbin3/accel-Mojo) to handle world/state management, and integrates [Metaplex MPL Core](https://developers.metaplex.com/core) for NFT profile pictures and playable character collections, with [Arweave](https://www.arweave.org/) for decentralized image and metadata storage.

## On-Chain Program

All world and state operations talk to the **accel-Mojo** program:

- **Repository:** https://github.com/Turbin3/accel-Mojo
- **Program ID:** `7iMdvW8A4Tw3yxjbXjpx4b8LTW13EQLB4eTmPyqRvxzM`

The program manages the lifecycle of world accounts and their delegated state — creation, delegation to the [MagicBlock](https://magicblock.gg) ephemeral rollup for fast writes, and committing state back to the Solana base layer.

NFT operations (profile pictures, character collections, character minting) go directly through Metaplex MPL Core; the accel-Mojo program is not involved for those.

---

## Installation

```toml
[dependencies]
mojo-rust-sdk = { git = "https://github.com/inspi-writer001/mojo-rust-sdk-v0" }
```

### Feature Flags

The SDK is split into three layers controlled by Cargo features:

| Feature | Default | What it adds |
|---------|---------|--------------|
| `native` | yes | `WorldClient`, all `World` RPC methods, account fetching. Pulls in `solana-client`, `solana-sdk`, `solana-transaction`, `solana-message`. |
| `arweave` | yes | `ArweaveUploader`, full end-to-end create methods. Implies `native` + `image-upload`. Pulls in `arweave-rs`, `tempfile`, `dirs`. |
| `image-upload` | yes | Image validation (`image` crate) and `tokio::fs` file loading. |

The core layer — types, instruction builders, `TransactionBundle`, the `mojo!` macros — is always compiled regardless of features, and is safe to use in WASM and Bevy frontends.

**Full SDK (default):**
```toml
mojo-rust-sdk = { git = "..." }
```

**Frontend / WASM (instruction builders only, no RPC):**
```toml
mojo-rust-sdk = { git = "...", default-features = false }
```

**Native RPC without Arweave (bring your own upload):**
```toml
mojo-rust-sdk = { git = "...", default-features = false, features = ["native"] }
```

---

## How It Works

### Worlds and State

A **World** is a PDA account owned by the accel-Mojo program. It stores arbitrary state as raw bytes using `bytemuck`. The SDK seeds the PDA deterministically from the owner pubkey and a name string.

State accounts go through two steps on the base layer:
1. **Create** — allocates the account with initial data.
2. **Delegate** — transfers execution authority to the MagicBlock ephemeral rollup so state can be written with low latency without waiting for full block confirmation.

Writes go to the ephemeral rollup; reads come from the ephemeral layer too. NFT operations always go through the Solana base layer.

The delegation program ID is `DELeGGvXpWV2fqJUhqcF5ZSYMS4JTLjteaAMARRSaeSh`.

### NFTs

**Profile pictures** are standalone MPL Core assets (`CreateV1Builder`).

**Character collections** use MPL Core collection accounts (`CreateCollectionV2Builder`). **Characters** are MPL Core assets linked to a collection (`CreateV2Builder`), which lets you query all assets in a collection on-chain without an indexer.

Metadata (name, description, image URL) is serialized to JSON and stored on Arweave. The Arweave transaction ID becomes the metadata URI written into the asset.

### TransactionBundle

The instruction builders return a `TransactionBundle`:

```rust
pub struct TransactionBundle {
    pub instructions: Vec<Instruction>,
    pub signers: Vec<Keypair>,   // ephemeral keypairs the SDK generated (asset, collection)
}
```

The `signers` are keypairs that the SDK generated internally (e.g. the new asset or collection account). In a backend context you sign with all of them. In a frontend context you partial-sign with the ephemeral signers and hand the transaction to the wallet adapter for the user's signature.

---

## Quick Start

### Defining State Types

Use the `mojo!` macro to define structs that can be stored on-chain. It derives `Pod`, `Zeroable`, `Clone`, `Copy` and gives you `LEN`, `to_bytes()`, and `len()`:

```rust
use mojo_rust_sdk::mojo;

mojo! {
    pub struct PlayerState {
        pub x: u64,
        pub y: u64,
        pub health: u64,
        pub score: u64,
    }
}

// PlayerState::LEN == 32
// player.to_bytes() -> &[u8]
```

For enum-like constants that live inside a `mojo!` struct, use `mojo_enum!`. It creates a `#[repr(transparent)]` newtype wrapping a backing integer — `Pod` compatible, zero-cost:

```rust
use mojo_rust_sdk::mojo_enum;

mojo_enum! { pub enum Direction: u8 {
    Up    = 0,
    Down  = 1,
    Left  = 2,
    Right = 3,
}}

// Direction::Up, Direction::Down, etc. are associated constants.
// Use inside a mojo! struct:
mojo! {
    pub struct PlayerState {
        pub x:         u64,
        pub y:         u64,
        pub facing:    Direction,
        _pad:          [u8; 7],
    }
}
```

> **Note:** All fields in a `mojo!` struct must be `Pod`. Non-primitive types (like `Direction`) must also be `Pod` (which `mojo_enum!` guarantees). Structs must be properly padded to avoid alignment issues — add explicit `_pad: [u8; N]` fields as needed.

---

### Backend Usage (default features)

The backend path holds keypairs directly and handles sign + send in one call.

#### Create a World

```rust
use mojo_rust_sdk::client::RpcType;
use mojo_rust_sdk::world::World;
use solana_keypair::Keypair;

let payer = Keypair::new();
let world = World::create_world(RpcType::Devnet, &payer, "my-game")?;
```

#### Create and Delegate a State Account

```rust
let initial = PlayerState { x: 0, y: 0, health: 100, score: 0 };
let state_pda = world.create_state(&payer, "player-state", &initial)?;
```

#### Write State (via ephemeral rollup)

```rust
let updated = PlayerState { x: 10, y: 20, health: 95, score: 50 };
let signature = world.write_state(&payer, "player-state", &updated)?;
```

#### Read State (from ephemeral layer)

```rust
let current: PlayerState = world.read_state(&payer.pubkey(), "player-state")?;
```

#### Profile Picture (full pipeline — requires `arweave` feature)

```rust
use mojo_rust_sdk::profile::ImageSource;

let profile = world.create_profile_picture(
    &user_keypair,
    None,                                   // payer — None means user pays
    ImageSource::from_path("avatar.png"),   // or ImageSource::from_url("https://...")
    "My Avatar",
    Some("A cool avatar"),
    None,                                   // ArweaveUploader — None uses default config
).await?;

println!("asset: {}", profile.asset);
println!("owner: {}", profile.owner);
```

#### Create a Character Collection (requires `arweave` feature)

```rust
let collection = world.create_character_collection(
    &creator_keypair,
    None,
    ImageSource::from_path("banner.png"),
    "Warriors",
    Some("A collection of warrior characters"),
    None,
).await?;
```

#### Add a Character to a Collection (requires `arweave` feature)

```rust
let character = world.create_character(
    &creator_keypair,
    None,
    &collection.collection,
    ImageSource::from_path("warrior.png"),
    "Fire Knight",
    Some("A warrior wielding flames"),
    None,
).await?;
```

#### Mint a Character for a Player (requires `native` feature)

The collection authority signs; a new ephemeral keypair is created for the asset account internally.

```rust
let minted = world.select_character(
    &authority_keypair,
    &player_pubkey,
    None,                   // payer
    &collection.collection,
    "Fire Knight",
    "https://arweave.net/<metadata-tx-id>",
)?;
```

#### Fetch Characters (requires `native` feature)

```rust
// By asset pubkey
let data = world.fetch_character(&asset_pubkey).await?;

// All characters owned by a wallet
let owned = world.fetch_characters_by_owner(&player_pubkey).await?;

// All characters in a collection (on-chain filter, no indexer needed)
let in_col = world.fetch_characters_by_collection(&collection.collection).await?;

// Collection metadata
let col_data = world.fetch_collection_data(&collection.collection)?;
println!("{} — {} minted", col_data.name, col_data.num_minted);

// Profile picture
let pic = world.get_profile_picture(&profile.asset).await?;
println!("{} — {}", pic.name, pic.image_uri);
```

---

### Frontend / WASM Usage (no default features)

With `default-features = false`, the entire RPC, signing, and filesystem stack is dropped. You get pure instruction builders that return `TransactionBundle`. No `async`, no network calls, no keypair storage.

```rust
use mojo_rust_sdk::world::World;
use mojo_rust_sdk::transaction::TransactionBundle;

// Build a character collection creation transaction
let bundle = World::build_character_collection_tx(
    payer_pubkey,
    "Warriors",
    "https://arweave.net/<metadata-tx-id>",
)?;

// Partial-sign with the SDK-generated ephemeral keypairs
// (these are the new account keypairs — asset, collection, etc.)
for signer in &bundle.signers {
    tx.partial_sign(&[signer], recent_blockhash);
}

// Hand `tx` to the wallet adapter for the user's signature, then submit.
```

All six `build_*_tx` methods are available with no features:

```rust
// Profile picture
let bundle = World::build_profile_picture_tx(owner, payer, "Avatar", metadata_uri)?;

// Character collection
let bundle = World::build_character_collection_tx(payer, "Warriors", metadata_uri)?;

// Character inside a collection
let bundle = World::build_character_tx(&collection_pubkey, owner, payer, "Knight", metadata_uri)?;

// Mint a character for a player (no ephemeral signers — wallet is the only signer)
let bundle = World::build_select_character_tx(
    &collection_pubkey, authority, buyer, payer, "Knight", metadata_uri,
)?;

// Create + delegate a state account (no ephemeral signers)
let bundle = World::build_create_state_tx(owner, "player-state", &initial_state_bytes)?;

// Write state via ephemeral rollup (no ephemeral signers)
let bundle = World::build_write_state_tx(owner, "player-state", &updated_state_bytes)?;
```

> `build_create_state_tx` and `build_select_character_tx` return `signers: vec![]` — only the wallet needs to sign.

---

## Arweave Wallet

The `ArweaveUploader` (requires `arweave` feature) looks for a wallet in this order:

1. Path passed to `ArweaveUploader::new(Some("path/to/wallet.json"), None)`
2. `ARWEAVE_WALLET` environment variable
3. `~/.arweave/wallet.json`

```rust
use mojo_rust_sdk::profile::ArweaveUploader;

// Explicit path
let uploader = ArweaveUploader::new(Some("/path/to/wallet.json".into()), None);

// Custom gateway
let uploader = ArweaveUploader::new(None, Some("https://arweave.net".into()));

// Default (env var or ~/.arweave/wallet.json, default gateway)
let uploader = ArweaveUploader::default();
```

---

## RPC Endpoints

`WorldClient` selects the endpoint based on `RpcType` and the layer being targeted:

| Network | Layer | Endpoint |
|---------|-------|----------|
| Devnet | Base Layer | `https://api.devnet.solana.com` |
| Devnet | Ephemeral | `https://devnet-eu.magicblock.app` |
| Mainnet | Base Layer | `https://api.mainnet-beta.solana.com` |
| Mainnet | Ephemeral | `https://mainnet-beta-eu.magicblock.app` |

World/state creates and NFT operations go to the Base Layer. State writes go to the Ephemeral layer.

---

## Source Layout

```
src/
├── lib.rs                      module declarations
├── transaction.rs              TransactionBundle (always compiled)
├── constants.rs                accel-Mojo program ID
├── error.rs                    WorldError enum
├── instructions.rs             raw Solana instruction builders (create_world, delegate, write)
├── pda.rs                      PDA derivation (SHA-256 seed hash + find_program_address)
├── mojo_types.rs               GenIxHandler, MojoInstructions — instruction encoding helpers
├── m_macro.rs                  mojo! and mojo_enum! macros
├── client.rs                   WorldClient, RpcType, RpcLayer  [native]
├── world.rs                    World struct — primary API surface
│                               ├── Layer 1 impl  (always, 6 build_*_tx methods)
│                               ├── Layer 2 impl  [native]
│                               └── Layer 3 impl  [arweave]
├── profile/
│   ├── types.rs                ProfilePicture, ProfilePictureData, Metadata, ImageSource
│   ├── asset.rs                create_mpl_core_asset_ix, build_profile_picture_tx,
│   │                           fetch_mpl_core_asset, fetch_metadata_from_uri  [native]
│   ├── image.rs                load_image_data  [native], validate_image  [image-upload]
│   └── uploader.rs             ArweaveUploader  [arweave]
└── playable-characters/
    ├── types.rs                Character, CharacterCollection, CharacterData,
    │                           CollectionData, CharacterMetadata
    └── asset.rs                create_collection_ix, create_character_ix, mint_character_ix,
                                build_character_collection_tx, build_character_tx,
                                build_select_character_tx,
                                fetch_character, fetch_collection, fetch_character_data,
                                fetch_character_metadata_from_uri,
                                fetch_characters_by_owner, fetch_characters_by_collection  [native]
```

---

## Development

```bash
# Full build (all features)
cargo build

# Core only — no RPC, no Arweave, no image crate (WASM-safe)
cargo build --no-default-features

# Native RPC layer without Arweave
cargo build --no-default-features --features native

# Tests
cargo test
```

---

## License

MIT OR Apache-2.0 — see [LICENSE](LICENSE).
