import { Ed25519Keypair, RawSigner, Transaction } from "@mysten/sui.js";
import { provider } from "./rpc";

// This simulates what a server would do to sponsor a transaction
export async function sponsorTransaction(
  sender: string,
  transactionKindBytes: Uint8Array
) {
  // Rather than do gas pool management, we just spin out a new keypair to sponsor the transaction with:
  const keypair = new Ed25519Keypair();
  const signer = new RawSigner(keypair, provider);
  const address = keypair.getPublicKey().toSuiAddress();
  await signer.requestSuiFromFaucet();
  console.log(`Sponsor address: ${address}`);

  const tx = Transaction.fromKind(transactionKindBytes);
  tx.setSender(sender);
  tx.setGasOwner(address);
  return await signer.signTransaction({ transaction: tx });
}
