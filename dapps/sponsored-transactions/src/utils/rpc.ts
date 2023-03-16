import { JsonRpcProvider, localnetConnection } from '@mysten/sui.js';

export const provider = new JsonRpcProvider(localnetConnection);
