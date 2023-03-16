// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { type SuiJsonValue, toB64 } from '@mysten/sui.js';
import clsx from 'clsx';

import styles from '~/pages/transaction-result/TransactionResult.module.css';
import { AddressOrObject } from '~/pages/transaction-result/programmable-transaction-view/AddressOrObject';
import { TableHeader } from '~/ui/TableHeader';

interface Props {
    inputs: SuiJsonValue[];
}

export function Inputs({ inputs }: Props) {
    if (!inputs?.length) {
        return null;
    }

    return (
        <div data-testid="programmable-transactions-inputs" className="mt-8">
            <section
                className={clsx([styles.txcomponent, styles.txgridcolspan2])}
            >
                <TableHeader>Inputs</TableHeader>
                <ul className="flex flex-col gap-y-3">
                    {inputs.map((input) => {
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
            </section>
        </div>
    );
}
