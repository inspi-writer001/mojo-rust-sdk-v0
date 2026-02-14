# Mojo Rust SDK

A Rust SDK for building on-chain games and applications with the Mojo World system on Solana. The SDK provides world and state management, NFT profile pictures, and playable character collections powered by [Metaplex MPL Core](https://developers.metaplex.com/core) and [Arweave](https://www.arweave.org/) for decentralized storage.

## Features

- **World Management** -- Create worlds, initialize delegated state accounts, and read/write state on-chain with support for both base layer (Solana) and ephemeral rollup (MagicBlock) execution
- **Profile Pictures** -- Mint NFT profile pictures as MPL Core assets with images and metadata stored on Arweave
- **Playable Characters** -- Create character collections, define character templates, and mint characters for players using MPL Core collections
- **State Macros** -- `mojo!` and `mojo_enum!` macros for defining on-chain state structs that are `Pod`/`Zeroable` compatible
- **Network Support** -- Built-in RPC configuration for both Devnet and Mainnet
- **Frontend Compatible** -- Three-layer architecture with Cargo feature gates lets frontends (wallet adapters, Bevy, WASM) use instruction builders without pulling in native dependencies

## Installation

Add the SDK to your project:

```toml
[dependencies]
mojo-rust-sdk = { git = "https://github.com/inspi-writer001/mojo-rust-sdk-v0" }
```

### Feature Flags

The SDK uses Cargo features to control which layers are compiled:

| Feature | Default | Description |
| ------- | ------- | ----------- |
| `native` | Yes | RPC client, transaction signing, account fetching (`WorldClient`, `World` convenience methods) |
| `arweave` | Yes | Arweave upload pipeline (implies `native` and `image-upload`) |
| `image-upload` | Yes | Image validation via the `image` crate and `tokio::fs` loading |

**With all defaults** you get the full SDK -- upload, build, sign, and send in one call.

**For frontends / WASM**, disable defaults to get only the core layer (types, instruction builders, `TransactionBundle`):

```toml
[dependencies]
mojo-rust-sdk = { git = "https://github.com/inspi-writer001/mojo-rust-sdk-v0", default-features = false }
```

**Native without Arweave** (bring your own upload pipeline):

```toml
[dependencies]
mojo-rust-sdk = { git = "https://github.com/inspi-writer001/mojo-rust-sdk-v0", default-features = false, features = ["native"] }
```

### Prerequisites

- Rust 2021 edition
- A funded Solana keypair (for `native` features)
- An [Arweave wallet](https://www.arweave.org/) (for `arweave` feature)

#### Arweave Wallet Setup

The SDK looks for an Arweave wallet in the following order:

1. Explicit path passed to `ArweaveUploader::new(Some("path/to/wallet.json"), None)`
2. `ARWEAVE_WALLET` environment variable
3. `~/.arweave/wallet.json`

## Quick Start

### Frontend / WASM Usage (no default features)

When using `default-features = false`, the SDK exposes pure instruction builders that return a `TransactionBundle` containing instructions and ephemeral signers. Pass these to a wallet adapter for signing.

```rust
use mojo_rust_sdk::transaction::TransactionBundle;
use mojo_rust_sdk::world::World;
use mojo_rust_sdk::playable_characters::{CharacterData, CharacterMetadata};

// Build instructions -- no RPC, no signing, no async
let bundle = World::build_character_collection_tx(
    payer_pubkey,
    "Warriors",
    "https://arweave.net/metadata-uri",
)?;

// bundle.instructions -- pass to your wallet adapter
// bundle.signers     -- partial-sign with these ephemeral keypairs
// The wallet signs the rest (user's signature)
```

Available `build_*_tx()` methods on `World`:

| Method | Description |
| ------ | ----------- |
| `build_profile_picture_tx(owner, payer, name, metadata_uri)` | Build a profile picture mint instruction |
| `build_character_collection_tx(payer, name, metadata_uri)` | Build a character collection creation instruction |
| `build_character_tx(collection, owner, payer, name, metadata_uri)` | Build a character asset creation instruction |
| `build_select_character_tx(collection, authority, buyer, payer, name, uri)` | Build a character mint instruction |

These are also available as standalone functions in `mojo_rust_sdk::profile::build_profile_picture_tx` and `mojo_rust_sdk::playable_characters::{build_character_collection_tx, build_character_tx, build_select_character_tx}`.

### Creating a World

*Requires `native` feature.*

```rust
use mojo_rust_sdk::client::RpcType;
use mojo_rust_sdk::world::World;
use solana_keypair::Keypair;
use solana_signer::Signer;

let payer = Keypair::new();
let world = World::create_world(RpcType::Devnet, &payer, "my-game")?;
```

### Defining On-Chain State

Use the `mojo!` macro to define state structs that can be stored on-chain. These structs are automatically `Pod`, `Zeroable`, `Clone`, and `Copy`.

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
```

You can also define enum-like constants with `mojo_enum!`:

```rust
use mojo_rust_sdk::mojo_enum;

mojo_enum! { pub enum Direction: u8 {
    Up = 0,
    Down = 1,
    Left = 2,
    Right = 3,
}}
```

### Creating and Managing State

*Requires `native` feature.*

```rust
// Create a delegated state account
let initial = PlayerState {
    x: 0,
    y: 0,
    health: 100,
    score: 0,
};
let state_pda = world.create_state(&payer, "player-state", &initial)?;

// Write state (goes through the ephemeral rollup layer)
let updated = PlayerState {
    x: 10,
    y: 20,
    health: 95,
    score: 50,
};
let signature = world.write_state(&payer, "player-state", &updated)?;

// Read state
let current: PlayerState = world.read_state(&payer.pubkey(), "player-state")?;
```

### Creating a Profile Picture

*Requires `arweave` feature (default).*

```rust
use mojo_rust_sdk::profile::{ImageSource, ArweaveUploader};

let image = ImageSource::from_path("avatar.png");
// or from a URL:
// let image = ImageSource::from_url("https://example.com/avatar.png");

let profile = world.create_profile_picture(
    &user_keypair,
    None,                // payer (None = user pays)
    image,
    "My Avatar",
    Some("A cool avatar"),
    None,                // ArweaveUploader (None = default config)
).await?;

println!("Profile picture asset: {}", profile.asset);
```

#### Fetching a Profile Picture

*Requires `native` feature.*

```rust
let data = world.get_profile_picture(&profile.asset).await?;

println!("Name: {}", data.name);
println!("Image: {}", data.image_uri);
```

### Playable Characters

#### 1. Create a Character Collection

*Requires `arweave` feature (default).*

A collection groups related characters together as an MPL Core collection.

```rust
let collection = world.create_character_collection(
    &creator_keypair,
    None,                              // payer
    ImageSource::from_path("collection-banner.png"),
    "Warriors",
    Some("A collection of warrior characters"),
    None,                              // ArweaveUploader
).await?;

println!("Collection: {}", collection.collection);
```

#### 2. Create a Character Template

*Requires `arweave` feature (default).*

Create a character asset inside a collection. This is the "master" character definition.

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

println!("Character asset: {}", character.asset);
```

#### 3. Mint a Character for a Player

*Requires `native` feature.*

The collection authority mints a new character for a buyer/player.

```rust
let minted = world.select_character(
    &authority_keypair,     // collection creator must sign
    &player_pubkey,         // new owner
    None,                   // payer
    &collection.collection,
    "Fire Knight",
    "https://arweave.net/metadata-uri",
)?;

println!("Minted character {} for {}", minted.asset, minted.owner);
```

#### 4. Fetch Characters

*Requires `native` feature.*

```rust
// Single character by asset address
let character_data = world.fetch_character(&asset_pubkey).await?;

// All characters owned by a wallet
let owned = world.fetch_characters_by_owner(&player_pubkey).await?;

// All characters in a collection
let in_collection = world.fetch_characters_by_collection(&collection_pubkey).await?;

// Collection metadata
let col_data = world.fetch_collection_data(&collection_pubkey)?;
println!("Collection '{}' has {} minted", col_data.name, col_data.num_minted);
```

## Architecture

The SDK is split into three layers, each gated behind Cargo features:

```
Layer 1 -- Core (always available, WASM-safe)
  Types, instruction builders, TransactionBundle
  No RPC, no signing, no filesystem access

Layer 2 -- Native (feature = "native")
  WorldClient, World convenience methods, account fetching
  Requires solana-client, solana-sdk

Layer 3 -- Arweave (feature = "arweave", implies native + image-upload)
  ArweaveUploader, full end-to-end create methods
  Requires arweave-rs, tempfile, dirs
```

```
src/
├── lib.rs              # Module exports (client gated behind native)
├── transaction.rs      # TransactionBundle (always available)
├── world.rs            # World struct -- primary API surface (3 impl blocks by layer)
├── client.rs           # RPC client (native only)
├── constants.rs        # Program ID
├── error.rs            # Error types (variants gated per feature)
├── instructions.rs     # Solana instruction builders (create, delegate, write)
├── pda.rs              # PDA derivation helpers
├── mojo_types.rs       # Instruction handler types
├── m_macro.rs          # mojo! and mojo_enum! macros
├── profile/
│   ├── asset.rs        # MPL Core asset creation & fetching, build_profile_picture_tx
│   ├── image.rs        # Image loading & validation (image-upload feature)
│   ├── types.rs        # ProfilePicture, Metadata, ImageSource
│   └── uploader.rs     # Arweave upload client (arweave feature)
└── playable-characters/
    ├── asset.rs        # Collection & character builders, build_*_tx, queries
    └── types.rs        # Character, CharacterCollection, CollectionData
```

### Key Concepts

**World** -- The central struct through which all SDK operations are performed. With `native`, wraps a `WorldData` account (creator, seed, address) and a network type. Without `native`, exposes only static `build_*_tx()` methods.

**TransactionBundle** -- Returned by all `build_*_tx()` methods. Contains `instructions: Vec<Instruction>` and `signers: Vec<Keypair>` (ephemeral keypairs generated for assets/collections). Frontends partial-sign with the ephemeral signers, then pass to a wallet adapter for the user's signature.

**State Delegation** -- State accounts are created on the Solana base layer and then delegated to the MagicBlock ephemeral rollup for fast writes. Reads go through the ephemeral layer; NFT operations go through the base layer.

**MPL Core** -- Profile pictures are standalone MPL Core assets (`CreateV1Builder`). Playable characters use MPL Core collections (`CreateCollectionV2Builder`) and collection-linked assets (`CreateV2Builder`).

**Arweave** -- Images and JSON metadata are uploaded to Arweave for permanent decentralized storage. The SDK handles the full pipeline: load image, validate format/size, upload image, build metadata JSON, upload metadata, then pass the metadata URI to the on-chain instruction.

### RPC Endpoints

| Network | Layer      | Endpoint                                 |
| ------- | ---------- | ---------------------------------------- |
| Devnet  | Base Layer | `https://api.devnet.solana.com`          |
| Devnet  | Ephemeral  | `https://devnet-eu.magicblock.app`       |
| Mainnet | Base Layer | `https://api.mainnet-beta.solana.com`    |
| Mainnet | Ephemeral  | `https://mainnet-beta-eu.magicblock.app` |

### Image Support

- Formats: PNG, JPG, GIF, WebP
- Max size: 10 MB
- Sources: local file path (`native` feature) or URL

## API Reference

### `World` -- Layer 1 (always available)

| Method | Description |
| ------ | ----------- |
| `build_profile_picture_tx(owner, payer, name, metadata_uri)` | Build profile picture mint instructions |
| `build_character_collection_tx(payer, name, metadata_uri)` | Build collection creation instructions |
| `build_character_tx(collection, owner, payer, name, metadata_uri)` | Build character creation instructions |
| `build_select_character_tx(collection, authority, buyer, payer, name, uri)` | Build character mint instructions |

### `World` -- Layer 2 (`native` feature)

| Method                                       | Description                              |
| -------------------------------------------- | ---------------------------------------- |
| `create_world(network, payer, name)`         | Create a new world account on-chain      |
| `create_state(payer, name, initial_state)`   | Create and delegate a state account      |
| `write_state(payer, name, new_state)`        | Write state via the ephemeral rollup     |
| `read_state(owner, name)`                    | Read state from the ephemeral layer      |
| `select_character(...)`                      | Mint a character for a player            |
| `get_profile_picture(asset)`                 | Fetch profile picture data and metadata  |
| `fetch_character(asset)`                     | Fetch a single character's data          |
| `fetch_characters_by_owner(owner)`           | Query all characters owned by a wallet   |
| `fetch_characters_by_collection(collection)` | Query all characters in a collection     |
| `fetch_collection_data(collection)`          | Fetch collection metadata                |

### `World` -- Layer 3 (`arweave` feature)

| Method | Description |
| ------ | ----------- |
| `create_profile_picture(...)` | Upload image/metadata to Arweave, mint NFT profile picture |
| `create_character_collection(...)` | Upload image/metadata, create MPL Core collection |
| `create_character(...)` | Upload image/metadata, create character asset in collection |

### Payer Pattern

Most creation methods accept an optional `payer` parameter. When `None`, the user/authority keypair pays for the transaction. When `Some`, a separate keypair covers fees -- useful for gasless experiences where a backend pays on behalf of users.

## Development

```bash
# Build (all features)
cargo build

# Build core only (WASM-safe)
cargo build --no-default-features

# Build native without arweave
cargo build --no-default-features --features native

# Run tests
cargo test
```

## License

See [LICENSE](LICENSE) for details.
