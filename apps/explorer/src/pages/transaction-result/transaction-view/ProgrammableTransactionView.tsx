// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import {
    type ProgrammableTransaction,
    type SuiTransactionResponse,
    toB64,
} from '@mysten/sui.js';

import { AddressOrObject } from '~/pages/transaction-result/transaction-view/AddressOrObject';
import { TableHeader } from '~/ui/TableHeader';

interface Props {
    transaction: SuiTransactionResponse;
}

export function ProgrammableTransactionView({ transaction }: Props) {
    // const transactionInputs = transaction.transaction!.data.transaction.inputs;
    const transactionData = transaction.transaction!.data
        .transaction as ProgrammableTransaction;

    return (
        <div data-testid="programmable-transactions-inputs">
            <TableHeader>Inputs</TableHeader>
            <ul className="flex flex-col gap-y-3">
                {transactionData.inputs.map((input) => {
                    if (Array.isArray(input)) {
                        const readableInput = toB64(
                            input as unknown as Uint8Array
                        );
                        return (
                            <li key={readableInput}>
                                <div className="mt-1 text-bodySmall font-medium text-steel-dark">
                                    {readableInput}
                                </div>
                            </li>
                        );
                    }

                    return (
                        <li key={String(input)}>
                            <AddressOrObject id={String(input)} />
                        </li>
                    );
                })}
            </ul>
        </div>
    );
}
