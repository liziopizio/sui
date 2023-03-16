// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import {
    type ProgrammableTransaction,
    type SuiTransactionResponse,
} from '@mysten/sui.js';

import { Commands } from '~/pages/transaction-result/programmable-transaction-view/Commands';
import { Inputs } from '~/pages/transaction-result/programmable-transaction-view/Inputs';

interface Props {
    transaction: SuiTransactionResponse;
}

export function ProgrammableTransactionView({ transaction }: Props) {
    const transactionData = transaction.transaction!.data
        .transaction as ProgrammableTransaction;

    console.log('transactionData', transactionData);

    return (
        <>
            <Inputs inputs={transactionData.inputs} />
            <Commands commands={transactionData.commands} />
        </>
    );
}
