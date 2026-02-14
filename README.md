# Mojo Rust SDK

A Rust SDK for building on-chain games and applications with the Mojo World system on Solana. The SDK provides world and state management, NFT profile pictures, and playable character collections powered by [Metaplex MPL Core](https://developers.metaplex.com/core) and [Arweave](https://www.arweave.org/) for decentralized storage.

## Features

- **World Management** -- Create worlds, initialize delegated state accounts, and read/write state on-chain with support for both base layer (Solana) and ephemeral rollup (MagicBlock) execution
- **Profile Pictures** -- Mint NFT profile pictures as MPL Core assets with images and metadata stored on Arweave
- **Playable Characters** -- Create character collections, define character templates, and mint characters for players using MPL Core collections
- **State Macros** -- `mojo!` and `mojo_enum!` macros for defining on-chain state structs that are `Pod`/`Zeroable` compatible
- **Network Support** -- Built-in RPC configuration for both Devnet and Mainnet

## Installation

Add the SDK to your project:

```toml
[dependencies]
mojo-rust-sdk = { git = "https://github.com/inspi-writer001/mojo-rust-sdk-v0" }
```

### Prerequisites

- Rust 2021 edition
- A funded Solana keypair
- An [Arweave wallet](https://www.arweave.org/) (for image/metadata uploads)

#### Arweave Wallet Setup

The SDK looks for an Arweave wallet in the following order:

1. Explicit path passed to `ArweaveUploader::new(Some("path/to/wallet.json"), None)`
2. `ARWEAVE_WALLET` environment variable
3. `~/.arweave/wallet.json`

## Quick Start

### Creating a World

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

```rust
let data = world.get_profile_picture(&profile.asset).await?;

println!("Name: {}", data.name);
println!("Image: {}", data.image_uri);
```

### Playable Characters

#### 1. Create a Character Collection

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

```
src/
├── lib.rs              # Module exports
├── world.rs            # World struct -- primary API surface
├── client.rs           # RPC client (Devnet/Mainnet, BaseLayer/Ephemeral)
├── constants.rs        # Program ID
├── error.rs            # Error types
├── instructions.rs     # Solana instruction builders (create, delegate, write)
├── pda.rs              # PDA derivation helpers
├── mojo_types.rs       # Instruction handler types
├── m_macro.rs          # mojo! and mojo_enum! macros
├── profile/
│   ├── asset.rs        # MPL Core asset creation & fetching
│   ├── image.rs        # Image loading & validation (max 10 MB)
│   ├── types.rs        # ProfilePicture, Metadata, ImageSource
│   └── uploader.rs     # Arweave upload client
└── playable-characters/
    ├── asset.rs        # Collection & character instruction builders, queries
    └── types.rs        # Character, CharacterCollection, CollectionData
```

### Key Concepts

**World** -- The central struct through which all SDK operations are performed. Wraps a `WorldData` account (creator, seed, address) and a network type.

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
- Sources: local file path or URL

## API Reference

### `World`

| Method                                       | Description                              |
| -------------------------------------------- | ---------------------------------------- |
| `create_world(network, payer, name)`         | Create a new world account on-chain      |
| `create_state(payer, name, initial_state)`   | Create and delegate a state account      |
| `write_state(payer, name, new_state)`        | Write state via the ephemeral rollup     |
| `read_state(owner, name)`                    | Read state from the ephemeral layer      |
| `create_profile_picture(...)`                | Mint an NFT profile picture              |
| `get_profile_picture(asset)`                 | Fetch profile picture data and metadata  |
| `create_character_collection(...)`           | Create an MPL Core character collection  |
| `create_character(...)`                      | Create a character asset in a collection |
| `select_character(...)`                      | Mint a character for a player            |
| `fetch_character(asset)`                     | Fetch a single character's data          |
| `fetch_characters_by_owner(owner)`           | Query all characters owned by a wallet   |
| `fetch_characters_by_collection(collection)` | Query all characters in a collection     |
| `fetch_collection_data(collection)`          | Fetch collection metadata                |

### Payer Pattern

Most creation methods accept an optional `payer` parameter. When `None`, the user/authority keypair pays for the transaction. When `Some`, a separate keypair covers fees -- useful for gasless experiences where a backend pays on behalf of users.

## Development

```bash
# Build
cargo build

# Run tests
cargo test
```

## License

See [LICENSE](LICENSE) for details.
