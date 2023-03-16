// Copyright (c) Mysten Labs, Inc.
// SPDX-License-Identifier: Apache-2.0

import { useSearch } from '~/hooks/useSearch';
import { AddressLink, ObjectLink } from '~/ui/InternalLink';
import { LoadingSpinner } from '~/ui/LoadingSpinner';

interface Props {
    id: string;
}

export function AddressOrObject({ id }: Props) {
    const { data, isLoading, isError } = useSearch(id);

    if (isLoading) {
        return (
            <div className="flex items-center justify-center">
                <LoadingSpinner />
            </div>
        );
    }

    if (!data || isError) {
        return null;
    }

    return (
        <>
            {data.map((result, index) => {
                if (result.label === 'address') {
                    return (
                        <div key={index}>
                            <AddressLink noTruncate address={id} />
                        </div>
                    );
                }
                if (result.label === 'object') {
                    return (
                        <div key={index}>
                            <ObjectLink noTruncate objectId={id} />
                        </div>
                    );
                }
                return null;
            })}
        </>
    );
}
