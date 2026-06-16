# Nonce Guard Program v0.1

[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

Nonce Guard is a lightweight Solana/SVM program for executing instructions through
an owner-derived PDA while rejecting transactions that use durable nonces.

The program is designed for cases where an application wants a deterministic
program-derived account to sign downstream CPIs, but does not want that execution
path to be available from a durable nonce transaction.

It provides:

- Guarded PDA signing: derive a PDA from an owner and let that PDA sign proxied
  CPIs.
- Durable nonce protection: inspect the instructions sysvar and reject
  transactions that advance or consume a durable nonce.
- Batched CPI execution: execute one or more compiled instructions in order.
- Minimal state surface: guarded accounts are PDAs derived from the owner and do
  not require program-owned account data.

The program is intentionally small and exposes a single instruction:
`execute_guarded`.

## Content

This repository contains:

- The `nonce-guard-program` Solana program.
- A generated TypeScript SDK in `sdk/ts`.
- A generated Rust client in `sdk/rust`.
- A Codama program definition in `codama/codama.ts`.
- LiteSVM tests for successful guarded execution and durable nonce rejection.
- An OtterSec audit report in `audits/ottersec_audit.pdf`.

## Program Addresses

Nonce Guard is deployed to:

- Solana Mainnet-beta: `guardgkc38afzaHQ7LNAtWHKgdUAAwyayhNdY8MPNnw`
- Solana Devnet: `guardgkc38afzaHQ7LNAtWHKgdUAAwyayhNdY8MPNnw`

The guarded account PDA is derived with:

```text
["guarded_proxy", owner]
```

## Execution Model

`execute_guarded` expects the following accounts:

```text
0. owner signer
1. guarded PDA
2. instructions sysvar
3.. remaining CPI accounts and programs
```

Instruction data is Borsh-serialized as:

```rust
ExecuteGuardedInstructionArgs {
    instructions: SmallVec<u8, CompiledInstruction>,
}
```

Each `CompiledInstruction` contains:

- `program_id_index`: index of the target program in the compact account space.
- `account_indices`: indices of the accounts required by the proxied CPI.
- `data`: raw instruction data for the proxied CPI.

Index `0` is reserved for the guarded PDA. Index `1` maps to the first remaining
account, index `2` maps to the second remaining account, and so on. When the
guarded PDA is included in a proxied CPI, it is marked as a signer by the Nonce
Guard program.

Self-CPI into the Nonce Guard program is blocked.

## Responsibility

By interacting with this program, users accept full responsibility for any
consequences of use, including losses caused by incorrect account ordering,
incorrect PDA derivation, incorrect program IDs, malformed proxied instructions,
or assumptions about replay and nonce behavior.

Important considerations:

- The owner must sign every guarded execution.
- The guarded PDA can sign any CPI encoded in the `execute_guarded` instruction,
  subject to the accounts and programs provided by the transaction.
- Durable nonce transactions are rejected by design.
- Account indices are compact and positional; incorrect ordering can cause the
  wrong CPI target or account metas to be used.
- Funds sent to a guarded PDA are controlled by instructions signed through that
  PDA and should be treated with the same care as any program-derived authority.

## Security

The repository includes an OtterSec audit report:

- `audits/ottersec_audit.pdf`

No audit can eliminate all risk. Review the program, generated clients, and
deployment configuration before integrating or deploying.
