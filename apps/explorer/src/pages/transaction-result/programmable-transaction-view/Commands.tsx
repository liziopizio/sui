// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0
import { type ProgrammableTransactionCommand } from '@mysten/sui.js';

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

    return (
        <div data-testid="programmable-transactions-commands" className="mt-8">
            <section className="pt-4 pb-4">
                <TableHeader>Commands</TableHeader>
                <ul className="flex flex-col gap-8">
                    {commands.map((command, index) => {
                        const commandName = Object.keys(command)[0];
                        const commandData =
                            command[commandName as keyof typeof command];
                        const formattedCommandData =
                            formatCommandData(commandData);

                        return (
                            <li key={index}>
                                <div className="text-heading6 font-semibold text-steel-darker">
                                    {commandName}
                                </div>
                                <div className="text-bodyMedium pt-2 text-steel">
                                    ({formattedCommandData})
                                </div>
                            </li>
                        );
                    })}
                </ul>
            </section>
        </div>
    );
}
