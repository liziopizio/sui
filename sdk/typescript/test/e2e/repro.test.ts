import { expect, it } from 'vitest';
import { Transaction } from '../../src';
import { setup } from './utils/setup';

it('repro', async () => {
  const toolbox = await setup();
  const tx = new Transaction();

  const coin = tx.splitCoin(tx.gas, tx.pure(1000000000n));
  tx.transferObjects([coin], tx.pure(toolbox.address()));

  const result = await toolbox.signer.dryRunTransaction({ transaction: tx });
	console.log(result);
	expect(result.effects.status.status).toBe('success');
});
