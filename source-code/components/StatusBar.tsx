import React, { useState, useEffect } from 'react';
import { Wifi, WifiOff, Battery, BatteryCharging, Clock } from 'lucide-react';
import { systemService } from '../services/systemService';
import { SystemState } from '../types';

export const StatusBar: React.FC = () => {
    const [time, setTime] = useState(new Date());
    const [state, setState] = useState<Partial<SystemState>>({
        batteryLevel: 100,
        batteryCharging: false,
        wifiEnabled: true
    });

    useEffect(() => {
        // Clock Timer
        const timer = setInterval(() => setTime(new Date()), 1000);

        // System State Polling (for battery/network updates)
        const statusPoller = setInterval(async () => {
            const currentState = await systemService.getState();
            setState(currentState);
        }, 2000); // Poll every 2 seconds

        // Initial fetch
        systemService.getState().then(setState);

        return () => {
            clearInterval(timer);
            clearInterval(statusPoller);
        };
    }, []);

    const getBatteryIcon = () => {
        if (state.batteryCharging) return <BatteryCharging size={18} className="text-green-400" />;
        if ((state.batteryLevel || 0) < 20) return <Battery size={18} className="text-red-500" />;
        return <Battery size={18} />;
    };

    return (
        <div className="h-12 w-full flex items-center justify-between px-8 bg-black/20 backdrop-blur-md fixed top-0 z-50 text-gray-400">
        <div className="text-xs font-bold tracking-widest uppercase text-steam-blue">Hacker Mode</div>
        <div className="flex items-center gap-6">

        {/* Network Status */}
        <div className="flex items-center gap-2">
        {state.wifiEnabled ? (
            <Wifi size={18} className="text-gray-200" />
        ) : (
            <WifiOff size={18} className="text-red-400" />
        )}
        </div>

        {/* Real Battery Status */}
        <div className="flex items-center gap-2">
        <span className={`text-sm font-mono ${(state.batteryLevel || 0) < 20 ? 'text-red-500' : 'text-gray-300'}`}>
        {state.batteryLevel}%
        </span>
        {getBatteryIcon()}
        </div>

        {/* Clock */}
        <div className="flex items-center gap-2 text-gray-200">
        <span className="text-lg font-medium">
        {time.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
        </span>
        <Clock size={16} />
        </div>
        </div>
        </div>
    );
};
