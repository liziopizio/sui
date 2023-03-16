// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
import { type ProgrammableTransactionCommand } from '@mysten/sui.js';
import clsx from 'clsx';

import styles from '~/pages/transaction-result/TransactionResult.module.css';
import { TableHeader } from '~/ui/TableHeader';

interface Props {
    commands: ProgrammableTransactionCommand[];
}

const formatCommandData = (commandData: any): string => {
    if (Array.isArray(commandData)) {
        return commandData
            .map((data: any) => {
                if (typeof data === 'object') {
                    return JSON.stringify(data);
                }
                return data;
            })
            .join(', ');
    }
    return JSON.stringify(commandData);
};

export function Commands({ commands }: Props) {
    if (!commands?.length) {
        return null;
    }

    console.log('commands', commands);

    return (
        <div data-testid="programmable-transactions-commands" className="mt-8">
            <section
                className={clsx([styles.txcomponent, styles.txgridcolspan2])}
            >
                <TableHeader>Commands</TableHeader>
                <ul className="flex flex-col gap-y-3">
                    {commands.map((command, index) => {
                        const commandName = Object.keys(command)[0];
                        const commandData: any =
                            command[commandName as keyof typeof command];
                        const formattedCommandData =
                            formatCommandData(commandData);

                        return (
                            <li key={`${commandName}-${index}`}>
                                <div className="text-heading6 font-semibold text-steel-darker">
                                    {commandName}
                                </div>
                                <div className="text-bodyMedium text-steel">{`(${formattedCommandData})`}</div>
                            </li>
                        );
                    })}
                </ul>
            </section>
        </div>
    );
}
