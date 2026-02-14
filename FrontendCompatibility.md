Here's what the research uncovered:

The Core Problem

The SDK bundles three concerns into single methods: instruction building + signing + transaction sending. This works for backend use (where you hold a Keypair), but breaks for frontends.

1. Wallet Adapter Mismatch  


Solana wallet adapters (Phantom, Solflare, etc.) never expose a private key. They provide async signTransaction(tx) methods. The Rust Signer trait is synchronous and expects secret key access -- you  
 literally cannot implement it for a wallet adapter. So every method that takes &impl Signer is unusable from a frontend wallet context.

2. Partial Signing is the Standard Pattern  


Your Keypair::new() for ephemeral asset/collection keypairs is fine conceptually -- every Solana NFT mint does this. But the workflow needs to be split:

1. App generates ephemeral keypair, builds instructions
2. App partialSign(tx) with the ephemeral keypair
3. Wallet signs the remaining slot (user approval popup)
4. Send the fully-signed transaction

Currently send_ixs_with_payer does all of this atomically, which is impossible when one signer is a browser wallet.

3. WASM is a No-Go Right Now

At least 4 critical blockers:

- solana_client::rpc_client::RpcClient -- uses reqwest::blocking which doesn't compile to WASM (no OS threads)
- tokio with features = ["full"] -- includes rt-multi-thread, fs, net etc., none of which exist in WASM
- arweave-rs -- depends on filesystem, tokio, file-based wallet loading
- dirs, tempfile, tokio::fs -- no filesystem in browsers

4. Bevy Integration

There's a Rust project called wallet-adapter (by mucks) that provides wallet-adapter-bevy -- a Bevy plugin for connecting to Solana wallets (Phantom, Solflare, etc.) on both native and WASM targets.
Bevy is tick-based/synchronous, so async RPC calls need to go through bevy_tasks::IoTaskPool or bevy_async_task, with results polled each frame via channels.

5. The Fix: Three-Layer Architecture

The research points to a clear pattern used by mpl-core, Anchor, and Umi:

┌───────────────────┬──────────────────────────────────────────────────────────────────────────────────────────────────────────┬───────────────┐
│ Layer │ Responsibility │ WASM-safe? │
├───────────────────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────┼───────────────┤
│ Instruction Layer │ Returns Vec<Instruction> + ephemeral Keypairs. No RPC, no signing. │ Yes │
├───────────────────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────┼───────────────┤
│ Transaction Layer │ Builds + signs transactions. Native uses Signer, WASM returns partially-signed tx for wallet completion. │ Feature-gated │
├───────────────────┼──────────────────────────────────────────────────────────────────────────────────────────────────────────┼───────────────┤
│ Client Layer │ Sends transactions, queries accounts. Native uses solana-client, WASM uses wasm_client_solana. │ Feature-gated │
└───────────────────┴──────────────────────────────────────────────────────────────────────────────────────────────────────────┴───────────────┘

Right now, every World method merges all three layers. The instruction builders (create_collection_ix, create_character_ix, etc.) already exist in asset.rs -- they just aren't the primary API surface.

What Metaplex Umi Does (the gold standard)

For the partial-signing problem specifically, Umi uses a NoopSigner pattern:

- Backend creates a placeholder signer for the user's pubkey (no secret key needed)
- Builds and signs the tx with the ephemeral keypair + authority
- Serializes the partially-signed tx to base64
- Frontend deserializes, wallet fills in the remaining signature, sends

Bottom Line

The SDK works great as a backend/CLI tool today. To support frontends (web wallet adapters, Bevy games), the main changes would be:

1. Expose instruction-builder methods that return (Vec<Instruction>, Vec<Keypair>) without signing/sending
2. Feature-gate the RPC client and filesystem-dependent code (#[cfg(not(target_arch = "wasm32"))])
3. Keep the current convenience methods as high-level wrappers for backend users
