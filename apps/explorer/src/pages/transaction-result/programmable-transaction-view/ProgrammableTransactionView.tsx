// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { type ProgrammableTransaction } from '@mysten/sui.js';

import { Commands } from '~/pages/transaction-result/programmable-transaction-view/Commands';
import { Inputs } from '~/pages/transaction-result/programmable-transaction-view/Inputs';

interface Props {
    transaction: ProgrammableTransaction;
}

export function ProgrammableTransactionView({ transaction }: Props) {
    return (
        <>
            <Inputs inputs={transaction.inputs} />
            <Commands commands={transaction.commands} />
        </>
    );
}
