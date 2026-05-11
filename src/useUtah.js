import { useEffect, useState } from 'react';

export function useUtah(channel) {
    const [data, setData] = useState(null);

    useEffect(() => {
        const handler = (event) => {
            setData(event.detail);
        };
        window.addEventListener(channel, handler);
        return () => window.removeEventListener(channel, handler);
    }, [channel]);

    const send = (payload) => {
        if (window.Utah) window.Utah.invoke(channel, payload);
    };

    return [data, send];
}

